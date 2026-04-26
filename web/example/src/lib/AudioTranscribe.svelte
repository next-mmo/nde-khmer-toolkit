<script>
  import { onMount } from 'svelte';
  import {
    AUDIO_EXTS,
    formatFileSize,
    prepareAudioForTranscription,
  } from './audio-transcribe/audio-processing.js';
  import { DEMO_AUDIO_SAMPLES, loadDemoAudioFile } from './audio-transcribe/samples.js';
  import { fmtTime, offsetSrtTimes, parseSrt, segmentWords } from './audio-transcribe/srt.js';

  // ── WASM state ──────────────────────────────────────────────────────────────
  let wasmReady = false;
  let wasmError = null;
  let transcribeFn = null;
  let normalizeFn = null;
  let generateSrtFn = null;

  onMount(async () => {
    try {
      const mod = await import('./wasm-transcribe/transcribe_audio_to_text.js');
      await mod.default('/transcribe_audio_to_text_bg.wasm');
      transcribeFn = mod.transcribe_audio;

      const kfaMod = await import('./wasm-kfa/kfa_wasm.js');
      await kfaMod.default('/kfa_wasm_bg.wasm');
      normalizeFn = kfaMod.normalize_khmer_text;
      generateSrtFn = kfaMod.generate_linear_srt;

      wasmReady = true;
    } catch (e) {
      wasmError = e.message || String(e);
    }
  });

  // ── UI state ────────────────────────────────────────────────────────────────
  let language = 'km-KH';
  let isDragging = false;
  let audioFile = null;     // File object
  let audioUrl = null;      // object URL for <audio> preview
  let transcript = '';
  let transcriptNormalized = '';
  let transcriptSrt = '';
  let status = 'idle';      // 'idle' | 'transcribing' | 'done' | 'error'
  let errorMsg = '';
  let convertTo16k = true;
  let reduceBackground = false;
  let processingStep = '';
  let sampleLoading = null;

  const LANGUAGES = [
    { code: 'km-KH', label: '🇰🇭 Khmer' },
    { code: 'en-US', label: '🇺🇸 English' },
    { code: 'zh-CN', label: '🇨🇳 Chinese' },
  ];

  // ── File handling ───────────────────────────────────────────────────────────
  function acceptFile(file) {
    if (!file) return;
    if (!AUDIO_EXTS.test(file.name)) {
      errorMsg = `Unsupported file type: "${file.name}". Use FLAC, WAV, MP3, OGG, etc.`;
      status = 'error';
      return;
    }
    if (audioUrl) URL.revokeObjectURL(audioUrl);
    audioFile = file;
    audioUrl = URL.createObjectURL(file);
    transcript = '';
    transcriptNormalized = '';
    transcriptSrt = '';
    status = 'idle';
    errorMsg = '';
    processingStep = '';
  }

  function onDrop(e) {
    e.preventDefault();
    isDragging = false;
    acceptFile(e.dataTransfer?.files?.[0]);
  }
  function onPick(e) { acceptFile(e.target.files?.[0]); }
  function onDragOver(e) { e.preventDefault(); isDragging = true; }
  function onDragLeave() { isDragging = false; }

  async function loadDemoAudio(sample) {
    if (sampleLoading || status === 'transcribing') return;
    sampleLoading = sample.id;
    status = 'idle';
    errorMsg = '';
    try {
      acceptFile(await loadDemoAudioFile(sample));
    } catch (e) {
      errorMsg = e?.message || String(e);
      status = 'error';
    } finally {
      sampleLoading = null;
    }
  }

  // ── Transcription ───────────────────────────────────────────────────────────
  async function transcribe() {
    if (!audioFile || !wasmReady) return;
    status = 'transcribing';
    transcript = '';
    transcriptNormalized = '';
    transcriptSrt = '';
    errorMsg = '';
    processingStep = '';
    try {
      const prepared = await prepareAudioForTranscription(audioFile, {
        convertTo16k,
        reduceBackground,
        onStep: (step) => { processingStep = step; },
      });
      const { bounds } = prepared;
      processingStep = 'Transcribing...';
      transcript = await transcribeFn(prepared.bytes, language, prepared.contentType);

      if (normalizeFn) {
        transcriptNormalized = normalizeFn(transcript, true, true);
      }

      if (generateSrtFn && bounds.end > bounds.start) {
        const textForSrt = segmentWords(transcriptNormalized || transcript, language);
        const speechDuration = bounds.end - bounds.start;
        const rawSrt = generateSrtFn(textForSrt, speechDuration);
        transcriptSrt = offsetSrtTimes(rawSrt, bounds.start);
      }
      
      status = 'done';
    } catch (e) {
      errorMsg = e?.message || String(e);
      status = 'error';
    } finally {
      processingStep = '';
    }
  }

  function reset() {
    if (audioUrl) URL.revokeObjectURL(audioUrl);
    audioFile = null; audioUrl = null;
    transcript = ''; transcriptNormalized = ''; transcriptSrt = ''; status = 'idle'; errorMsg = ''; processingStep = '';
  }

  function copyText(text) {
    if (text) navigator.clipboard.writeText(text).catch(() => {});
  }

  function downloadSrt() {
    if (!transcriptSrt) return;
    const blob = new Blob([transcriptSrt], { type: 'text/plain' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = (audioFile?.name.replace(/\.[^.]+$/, '') ?? 'subtitles') + '.srt';
    a.click();
    URL.revokeObjectURL(a.href);
  }

  $: fmtSize = formatFileSize(audioFile);

  // ── SRT player ──────────────────────────────────────────────────────────────
  // NOTE: kfa-wasm `generate_linear_srt` produces **one cue per word** with
  // linear (duration / word-count) timing — true acoustic forced alignment
  // requires an ONNX runtime that isn't loaded in WASM yet. So each cue IS
  // a single karaoke word; we just group cues into visual lines for display.
  const WORDS_PER_LINE = 8;

  let audioEl = null;
  let srtCues = [];          // [{start, end, text}, ...]  (one entry per word)
  let activeCueIndex = -1;
  let activeWordFill = 0;    // 0..1 — wipe progress within the current word
  let rafId = null;

  $: lines = chunk(srtCues, WORDS_PER_LINE);
  $: activeLineIndex = activeCueIndex >= 0 ? Math.floor(activeCueIndex / WORDS_PER_LINE) : -1;
  $: activeIndexInLine = activeCueIndex >= 0 ? activeCueIndex % WORDS_PER_LINE : -1;

  function chunk(arr, n) {
    const out = [];
    for (let i = 0; i < arr.length; i += n) out.push(arr.slice(i, i + n));
    return out;
  }

  function syncFromTime(t) {
    if (!srtCues.length) return;
    let idx = -1;
    for (let i = 0; i < srtCues.length; i++) {
      if (t >= srtCues[i].start && t < srtCues[i].end) { idx = i; break; }
    }
    if (idx === -1 && t >= srtCues[srtCues.length - 1].end) {
      idx = srtCues.length - 1;
      activeWordFill = 1;
    } else if (idx >= 0) {
      const c = srtCues[idx];
      activeWordFill = Math.min(1, Math.max(0, (t - c.start) / Math.max(c.end - c.start, 0.001)));
    } else {
      activeWordFill = 0;
    }
    activeCueIndex = idx;
  }

  function tick() {
    if (!audioEl) return;
    syncFromTime(audioEl.currentTime);
    rafId = requestAnimationFrame(tick);
  }

  function onPlay() {
    if (rafId == null) rafId = requestAnimationFrame(tick);
  }
  function onPauseOrEnd() {
    if (rafId != null) { cancelAnimationFrame(rafId); rafId = null; }
    if (audioEl) syncFromTime(audioEl.currentTime);
  }
  function onSeeked() {
    if (audioEl) syncFromTime(audioEl.currentTime);
  }

  $: if (transcriptSrt) {
    srtCues = parseSrt(transcriptSrt);
  } else {
    srtCues = [];
    activeCueIndex = -1;
    activeWordFill = 0;
  }

</script>

<!-- ===== HEADER ===== -->
<header>
  <div class="header-inner">
    <div class="brand">
      <span class="brand-icon">🎙️</span>
      <div>
        <h1>Audio → Text</h1>
        <span class="sub">Khmer Speech Recognition · WebAssembly</span>
      </div>
    </div>
    <div class="header-chips">
      {#if wasmReady}
        <span class="chip green">● WASM Ready</span>
      {:else if wasmError}
        <span class="chip red">⚠ WASM Failed</span>
      {:else}
        <span class="chip">⟳ Loading WASM…</span>
      {/if}
      <span class="chip">Google Speech API v2</span>
    </div>
  </div>
</header>

<!-- ===== MAIN ===== -->
<main>
  {#if wasmError}
    <div class="alert-banner error">
      <span>⚠️</span>
      <p>Failed to load WebAssembly module: <code>{wasmError}</code></p>
    </div>
  {/if}

  <!-- Language selector -->
  <section class="glass controls-panel">
    <div class="lang-row">
      <label for="lang-select">Language</label>
      <div class="select-wrap">
        <select id="lang-select" bind:value={language}>
          {#each LANGUAGES as l}
            <option value={l.code}>{l.label}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="audio-options" aria-label="Audio preprocessing options">
      <label class="check-option">
        <input type="checkbox" bind:checked={convertTo16k} />
        <span>
          <strong>Convert to 16 kHz mono</strong>
          <small>Browser-side PCM conversion before transcription</small>
        </span>
      </label>

      <label class="check-option">
        <input type="checkbox" bind:checked={reduceBackground} />
        <span>
          <strong>Remove background, keep vocal</strong>
          <small>Experimental Web Audio filter; also sends 16 kHz mono</small>
        </span>
      </label>
    </div>

    <!-- Note about FLAC/WAV -->
    <p class="note">
      💡 For best results, keep <strong>Convert to 16 kHz mono</strong> enabled unless your file is already prepared.
    </p>
  </section>

  <!-- Drop zone -->
  <section
    class="glass drop-zone {isDragging ? 'dragging' : ''} {audioFile ? 'has-file' : ''}"
    aria-label="Audio file drop zone"
    on:dragover={onDragOver}
    on:dragleave={onDragLeave}
    on:drop={onDrop}
  >
    {#if !audioFile}
      <div class="drop-icon">{isDragging ? '📂' : '🎵'}</div>
      <h2>Drop your audio file here</h2>
      <p>Drag & drop a FLAC, WAV, MP3, OGG, or similar file, or click to browse.</p>
      <div class="drop-actions">
        <label class="btn-pick" for="audio-file-input">
          Browse file…
          <input id="audio-file-input" type="file" accept="audio/*,.flac" on:change={onPick} />
        </label>
        {#each DEMO_AUDIO_SAMPLES as sample}
          <button
            class="btn-sample"
            disabled={sampleLoading || status === 'transcribing'}
            on:click={() => loadDemoAudio(sample)}
          >
            {sampleLoading === sample.id ? 'Loading...' : sample.label}
          </button>
        {/each}
      </div>
    {:else}
      <!-- File preview -->
      <div class="file-preview">
        <div class="file-info">
          <span class="file-icon">🎵</span>
          <div class="file-meta">
            <span class="file-name">{audioFile.name}</span>
            <span class="file-size">{fmtSize}</span>
          </div>
          <button class="btn-reset" title="Remove file" on:click={reset}>✕</button>
        </div>

        <!-- Audio player -->
        <audio
          bind:this={audioEl}
          controls
          src={audioUrl}
          class="audio-player"
          on:play={onPlay}
          on:pause={onPauseOrEnd}
          on:ended={onPauseOrEnd}
          on:seeked={onSeeked}
          on:loadedmetadata={onSeeked}
        >
          Your browser does not support the audio element.
        </audio>

        <!-- Karaoke stage (shown once SRT is ready) -->
        {#if srtCues.length}
          <div class="karaoke-stage">
            <p class="karaoke-meta" title="Linear timing — true acoustic forced alignment is not yet enabled in WASM">
              ♪ Karaoke · linear estimation
            </p>

            <p class="karaoke-ctx karaoke-prev">
              {activeLineIndex > 0 ? lines[activeLineIndex - 1].map(c => c.text).join(' ') : ''}
            </p>

            <div class="karaoke-line">
              {#if activeLineIndex >= 0}
                {#key activeLineIndex}
                  {#each lines[activeLineIndex] as cue, wi}
                    {#if wi < activeIndexInLine}
                      <span class="kw kw-done">{cue.text}</span>
                    {:else if wi === activeIndexInLine}
                      <span class="kw kw-now" aria-current="true">
                        <span class="kw-base">{cue.text}</span>
                        <span
                          class="kw-fill"
                          style="clip-path: inset(0 {(1 - activeWordFill) * 100}% 0 0); -webkit-clip-path: inset(0 {(1 - activeWordFill) * 100}% 0 0);"
                          aria-hidden="true"
                        >{cue.text}</span>
                      </span>
                    {:else}
                      <span class="kw kw-next">{cue.text}</span>
                    {/if}
                  {/each}
                {/key}
              {:else}
                <span class="karaoke-idle">♪ &nbsp; ♪ &nbsp; ♪</span>
              {/if}
            </div>

            <p class="karaoke-ctx karaoke-next">
              {activeLineIndex >= 0 && activeLineIndex < lines.length - 1
                ? lines[activeLineIndex + 1].map(c => c.text).join(' ') : ''}
            </p>
          </div>
        {/if}

        <!-- Action row -->
        <div class="action-row">
          <button
            class="btn-transcribe"
            disabled={!wasmReady || status === 'transcribing'}
            on:click={transcribe}
          >
            {#if status === 'transcribing'}
              <span class="spin">⟳</span> {processingStep || 'Transcribing...'}
            {:else}
              Transcribe →
            {/if}
          </button>
          <label class="btn-pick-sm" for="audio-file-input2">
            Change file
            <input id="audio-file-input2" type="file" accept="audio/*,.flac" on:change={onPick} />
          </label>
          {#each DEMO_AUDIO_SAMPLES as sample}
            <button
              class="btn-pick-sm"
              disabled={sampleLoading || status === 'transcribing'}
              on:click={() => loadDemoAudio(sample)}
            >
              {sampleLoading === sample.id ? 'Loading...' : sample.label}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </section>

  <!-- Results -->
  {#if status === 'done' && transcript}
    <div class="results-grid">
      <section class="glass result-section">
        <div class="result-header">
          <h2>Raw Transcript</h2>
          <button class="btn-copy" on:click={() => copyText(transcript)} title="Copy to clipboard">📋 Copy</button>
        </div>
        <p class="transcript-text">{transcript}</p>
      </section>

      {#if transcriptNormalized}
        <section class="glass result-section">
          <div class="result-header">
            <h2>Normalized (kfa-wasm)</h2>
            <button class="btn-copy" on:click={() => copyText(transcriptNormalized)} title="Copy to clipboard">📋 Copy</button>
          </div>
          <p class="transcript-text">{transcriptNormalized}</p>
        </section>
      {/if}

      {#if transcriptSrt}
        <section class="glass result-section srt-section">
          <div class="result-header">
            <h2>SRT Subtitles</h2>
            <div class="srt-actions">
              <button class="btn-copy" on:click={() => copyText(transcriptSrt)} title="Copy raw SRT">📋 Copy</button>
              <button class="btn-copy" on:click={downloadSrt} title="Download .srt file">⬇ Download</button>
            </div>
          </div>
          <div class="cue-list">
            {#each srtCues as cue}
              <div
                class="cue-item"
                role="button"
                tabindex="0"
                on:click={() => { if (audioEl) { audioEl.currentTime = cue.start; audioEl.play(); } }}
                on:keydown={(e /** @type {KeyboardEvent} */) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (audioEl) { audioEl.currentTime = cue.start; audioEl.play(); } } }}
              >
                <span class="cue-time">{fmtTime(cue.start)}</span>
                <span class="cue-text">{cue.text}</span>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {:else if status === 'error'}
    <section class="glass error-section">
      <span class="error-icon">⚠️</span>
      <div>
        <p class="error-title">Transcription failed</p>
        <p class="error-body">{errorMsg}</p>
      </div>
    </section>
  {:else if status === 'idle' && !audioFile}
    <div class="empty-state">
      <div class="empty-icon">🎤</div>
      <p>Upload an audio file above to transcribe it</p>
      <p class="empty-sub">
        Runs entirely in your browser via WebAssembly. Audio is sent to Google Speech API v2
        — no data touches any other server.
      </p>
    </div>
  {/if}
</main>

<!-- ===== FOOTER ===== -->
<footer>
  <span>transcribe-audio-to-text · Rust → WASM</span>
  <span>WASM: ~71 KB · Google Speech API v2</span>
</footer>

<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;600&display=swap');

  header {
    background: rgba(10,14,26,0.9);
    backdrop-filter: blur(16px);
    border-bottom: 1px solid rgba(20,184,166,0.18);
    position: sticky;
    top: 0;
    z-index: 100;
    padding: 0 1.5rem;
  }

  .header-inner {
    max-width: 860px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 64px;
    gap: 1rem;
  }

  .brand { display: flex; align-items: center; gap: 0.75rem; }
  .brand-icon { font-size: 1.8rem; line-height: 1; }

  h1 {
    font-size: 1.3rem;
    font-weight: 700;
    color: #e0fdf4;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }

  .sub { font-size: 0.7rem; color: #475569; letter-spacing: 0.04em; }

  .header-chips { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .chip {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 999px;
    padding: 0.25em 0.75em;
    font-size: 0.72rem;
    color: #94a3b8;
    white-space: nowrap;
  }
  .chip.green { color: #4ade80; border-color: rgba(74,222,128,0.3); background: rgba(74,222,128,0.07); }
  .chip.red   { color: #f87171; border-color: rgba(239,68,68,0.3);  background: rgba(239,68,68,0.07); }

  main {
    max-width: 860px;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .glass {
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 18px;
    backdrop-filter: blur(12px);
  }

  /* ── Alert banner ── */
  .alert-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    border-radius: 14px;
    font-size: 0.87rem;
  }
  .alert-banner.error {
    background: rgba(239,68,68,0.08);
    border: 1px solid rgba(239,68,68,0.25);
    color: #fca5a5;
  }
  .alert-banner code {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.82em;
    background: rgba(0,0,0,0.2);
    padding: 0.1em 0.4em;
    border-radius: 4px;
  }

  /* ── Controls panel ── */
  .controls-panel {
    padding: 1.25rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .lang-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .lang-row label {
    font-size: 0.83rem;
    color: #64748b;
    font-weight: 500;
    min-width: 80px;
  }

  .select-wrap {
    position: relative;
  }

  select {
    appearance: none;
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 10px;
    padding: 0.55em 2.2em 0.55em 1em;
    color: #e2e8f4;
    font-size: 0.9rem;
    font-family: inherit;
    cursor: pointer;
    outline: none;
    transition: border-color 0.2s;
  }
  select:focus { border-color: rgba(20,184,166,0.6); box-shadow: 0 0 0 3px rgba(20,184,166,0.12); }

  .select-wrap::after {
    content: '▾';
    position: absolute;
    right: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
    color: #475569;
    font-size: 0.8rem;
  }

  .audio-options {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .check-option {
    display: flex;
    align-items: flex-start;
    gap: 0.7rem;
    padding: 0.85rem 0.95rem;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 12px;
    color: #cbd5e1;
    text-align: left;
    cursor: pointer;
  }

  .check-option input {
    width: 1rem;
    height: 1rem;
    margin-top: 0.15rem;
    accent-color: #14b8a6;
    flex-shrink: 0;
  }

  .check-option span {
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
    min-width: 0;
  }

  .check-option strong {
    font-size: 0.84rem;
    font-weight: 600;
    color: #dbeafe;
    line-height: 1.25;
  }

  .check-option small {
    font-size: 0.74rem;
    color: #64748b;
    line-height: 1.35;
  }

  .note {
    font-size: 0.8rem;
    color: #475569;
    line-height: 1.6;
  }
  .note strong { color: #94a3b8; }

  /* ── Drop zone ── */
  .drop-zone {
    padding: 3rem 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    text-align: center;
    min-height: 280px;
    justify-content: center;
    transition: border-color 0.2s, background 0.2s;
    cursor: default;
  }
  .drop-zone.has-file { padding: 1.75rem; align-items: stretch; }

  .drop-zone.dragging {
    border-color: rgba(20,184,166,0.6);
    background: rgba(20,184,166,0.06);
    box-shadow: 0 0 0 4px rgba(20,184,166,0.1), inset 0 0 40px rgba(20,184,166,0.04);
  }

  .drop-icon { font-size: 3rem; transition: transform 0.2s; }
  .drop-zone.dragging .drop-icon { transform: scale(1.15); }

  h2 { font-size: 1.2rem; font-weight: 600; color: #ccfbf1; }
  .drop-zone > p { color: #64748b; font-size: 0.88rem; max-width: 360px; line-height: 1.6; }

  .btn-pick {
    background: linear-gradient(135deg, #0d9488, #0891b2);
    color: white;
    padding: 0.72em 1.8em;
    border-radius: 12px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s, transform 0.15s;
    box-shadow: 0 4px 14px rgba(20,184,166,0.3);
    display: inline-block;
  }
  .btn-pick:hover { opacity: 0.9; transform: translateY(-1px); }
  .btn-pick input, .btn-pick-sm input { display: none; }

  .drop-actions {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .btn-sample {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.12);
    color: #cbd5e1;
    padding: 0.72em 1.4em;
    border-radius: 12px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
    white-space: nowrap;
  }
  .btn-sample:hover:not(:disabled) {
    background: rgba(20,184,166,0.1);
    border-color: rgba(20,184,166,0.3);
    color: #ccfbf1;
  }
  .btn-sample:disabled { opacity: 0.45; cursor: not-allowed; }

  /* ── File preview ── */
  .file-preview {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: rgba(20,184,166,0.06);
    border: 1px solid rgba(20,184,166,0.2);
    border-radius: 12px;
  }

  .file-icon { font-size: 1.5rem; }
  .file-meta { flex: 1; min-width: 0; }
  .file-name {
    display: block;
    font-size: 0.9rem;
    font-weight: 500;
    color: #e2e8f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file-size { font-size: 0.75rem; color: #475569; }

  .btn-reset {
    background: none;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    color: #475569;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 0.75rem;
    flex-shrink: 0;
    transition: all 0.15s;
  }
  .btn-reset:hover { background: rgba(239,68,68,0.1); color: #f87171; border-color: rgba(239,68,68,0.3); }

  .audio-player {
    width: 100%;
    border-radius: 10px;
    height: 40px;
    accent-color: #14b8a6;
    filter: invert(0.85) hue-rotate(140deg);
  }

  .action-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .btn-transcribe {
    flex: 1;
    background: linear-gradient(135deg, #0d9488, #0891b2);
    border: none;
    color: white;
    padding: 0.85rem 1.5rem;
    border-radius: 12px;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
    transition: opacity 0.2s, transform 0.15s;
    box-shadow: 0 4px 16px rgba(20,184,166,0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    white-space: nowrap;
  }
  .btn-transcribe:hover:not(:disabled) { opacity: 0.9; transform: translateY(-1px); }
  .btn-transcribe:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-pick-sm {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.12);
    color: #94a3b8;
    padding: 0.75em 1.2em;
    border-radius: 12px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
  }
  .btn-pick-sm:hover { background: rgba(255,255,255,0.09); }
  .btn-pick-sm:disabled { opacity: 0.45; cursor: not-allowed; }

  .spin { display: inline-block; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Result ── */
  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1.5rem;
  }

  .result-section {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    animation: fadeUp 0.3s ease both;
  }

  @keyframes fadeUp {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .result-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .result-header h2 { font-size: 0.9rem; color: #4ade80; font-weight: 600; letter-spacing: 0.04em; }

  .btn-copy {
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    color: #64748b;
    padding: 0.35em 0.85em;
    font-size: 0.78rem;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s;
  }
  .btn-copy:hover { background: rgba(20,184,166,0.1); color: #2dd4bf; border-color: rgba(20,184,166,0.3); }

  .transcript-text {
    font-size: 1.15rem;
    line-height: 1.9;
    color: #e2e8f4;
    letter-spacing: 0.01em;
    word-break: break-word;
    padding: 1rem;
    background: rgba(20,184,166,0.05);
    border: 1px solid rgba(20,184,166,0.15);
    border-radius: 12px;
  }
  
  /* ── Karaoke stage ── */
  .karaoke-stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.55rem;
    padding: 1.1rem 1rem 1.25rem;
    background: rgba(0,0,0,0.45);
    border: 1px solid rgba(20,184,166,0.18);
    border-radius: 16px;
    text-align: center;
  }

  .karaoke-meta {
    font-size: 0.65rem;
    color: #475569;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    margin-bottom: 0.15rem;
    cursor: help;
  }

  .karaoke-ctx {
    font-size: 0.88rem;
    line-height: 1.6;
    min-height: 1.4em;
    letter-spacing: 0.01em;
  }
  .karaoke-prev { color: #1e3a3a; }
  .karaoke-next { color: #1e3a3a; }

  .karaoke-line {
    display: flex;
    flex-wrap: wrap;
    gap: 0.15em 0.45em;
    justify-content: center;
    align-items: baseline;
    min-height: 2.6em;
    animation: lineIn 0.2s ease both;
  }

  @keyframes lineIn {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .kw {
    font-size: 1.45rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    display: inline-block;
    transition: color 0.15s ease;
  }
  .kw-done {
    color: #2dd4bf;
    text-shadow: 0 0 8px rgba(45,212,191,0.35);
  }
  .kw-next {
    color: #1e3a3a;
  }

  /* Active word: a dim "base" copy with a bright "fill" copy clipped on top.
     The clip-path is updated on every animation frame for a smooth wipe. */
  .kw-now {
    position: relative;
    display: inline-block;
    color: #1e3a3a;
    transform: scale(1.06);
  }
  .kw-now .kw-base {
    color: #1e3a3a;
  }
  .kw-now .kw-fill {
    position: absolute;
    inset: 0;
    color: #ffffff;
    text-shadow:
      0 0 14px rgba(20,184,166,0.95),
      0 0 28px rgba(20,184,166,0.55),
      0 0 48px rgba(20,184,166,0.25);
    pointer-events: none;
    will-change: clip-path;
  }

  .karaoke-idle {
    font-size: 1.2rem;
    color: #1e3a3a;
    letter-spacing: 0.3em;
    animation: idlePulse 1.6s ease-in-out infinite;
  }
  @keyframes idlePulse {
    0%, 100% { opacity: 0.25; }
    50%       { opacity: 0.6; }
  }

  /* ── SRT cue list ── */
  .srt-section { grid-column: 1 / -1; }

  .srt-actions { display: flex; gap: 0.5rem; }

  .cue-list {
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    scrollbar-width: thin;
    scrollbar-color: rgba(20,184,166,0.3) transparent;
  }

  .cue-item {
    display: flex;
    gap: 0.75rem;
    align-items: baseline;
    padding: 0.45rem 0.75rem;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.12s;
    border: 1px solid transparent;
  }
  .cue-item:hover { background: rgba(255,255,255,0.04); }
  .cue-item:focus-visible { outline: 2px solid rgba(20,184,166,0.6); outline-offset: 1px; }

  .cue-time {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.72rem;
    color: #475569;
    white-space: nowrap;
    flex-shrink: 0;
    padding-top: 0.1em;
  }
  .cue-text {
    font-size: 1rem;
    line-height: 1.6;
    color: #94a3b8;
    word-break: break-word;
  }

  /* ── Error section ── */
  .error-section {
    padding: 1.25rem 1.5rem;
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    background: rgba(239,68,68,0.06);
    border-color: rgba(239,68,68,0.2);
  }
  .error-icon { font-size: 1.4rem; flex-shrink: 0; margin-top: 2px; }
  .error-title { font-size: 0.9rem; font-weight: 600; color: #fca5a5; }
  .error-body  { font-size: 0.82rem; color: #fca5a5; opacity: 0.75; margin-top: 0.2rem; word-break: break-word; }

  /* ── Empty state ── */
  .empty-state {
    text-align: center;
    padding: 3.5rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
  }
  .empty-icon { font-size: 3rem; margin-bottom: 0.25rem; }
  .empty-state p { color: #475569; font-size: 0.92rem; }
  .empty-sub {
    color: #334155 !important;
    font-size: 0.78rem !important;
    max-width: 420px;
    line-height: 1.6;
  }

  /* ── Footer ── */
  footer {
    border-top: 1px solid rgba(255,255,255,0.05);
    padding: 1rem 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.74rem;
    color: #334155;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  @media (max-width: 768px) {
    .results-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 600px) {
    .header-chips { display: none; }
    .audio-options { grid-template-columns: 1fr; }
    .action-row { flex-direction: column; }
    .btn-transcribe { width: 100%; }
  }
</style>
