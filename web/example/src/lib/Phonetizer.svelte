<script>
  export let model;

  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();

  // --- state ---
  let inputText = '';
  let nbest = 3;
  let results = [];        // [{ word, phonemes: string[], paths: string[] }]
  let isProcessing = false;
  let activeTab = 'single'; // 'single' | 'batch'

  // batch tab
  let batchInput = 'hello\nworld\ncomputer\nphonetics\nlinguistics\nalgorithm\nkhmer\nbeautiful';

  // demo words for quick-try
  const demos = ['hello', 'world', 'beautiful', 'algorithm', 'linguistics', 'phonetics', 'computer', 'knowledge'];

  function runSingle() {
    const trimmed = inputText.trim().toLowerCase();
    if (!trimmed) return;
    isProcessing = true;

    // Run in next tick so Svelte can render the spinner
    setTimeout(() => {
      try {
        const words = trimmed.split(/\s+/);
        const newResults = words.map(word => {
          const phonemes = model.phoneticize(word);
          const paths = model.phoneticize_nbest(word, nbest);
          return { word, phonemes, paths };
        });
        results = newResults;
      } finally {
        isProcessing = false;
      }
    }, 10);
  }

  function runBatch() {
    const words = batchInput.split('\n').map(w => w.trim().toLowerCase()).filter(Boolean);
    if (!words.length) return;
    isProcessing = true;

    setTimeout(() => {
      try {
        results = words.map(word => {
          const phonemes = model.phoneticize(word);
          const paths = model.phoneticize_nbest(word, nbest);
          return { word, phonemes, paths };
        });
      } finally {
        isProcessing = false;
      }
    }, 10);
  }

  function tryDemo(word) {
    inputText = word;
    activeTab = 'single';
    runSingle();
  }

  function clearResults() {
    results = [];
    inputText = '';
  }

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (activeTab === 'single') runSingle();
      else runBatch();
    }
  }

  // Phoneme colouring — handles both ARPAbet (AA1, EH0…) and IPA-style (ɛɛ, oo…)
  // A phoneme is "vowel" if it contains a unicode vowel letter or is a known ARPAbet vowel code.
  const ARPABET_VOWELS = new Set(['aa','ae','ah','ao','aw','ay','eh','er','ey','ih','iy','ow','oy','uh','uw']);
  const IPA_VOWEL_RE = /[aeiouæɑɒɔəɛɜɪʊʌyøœɐɵɨʉ]/i;
  function phonemeClass(ph) {
    const base = ph.replace(/[012]$/, '').toLowerCase();
    return ARPABET_VOWELS.has(base) || IPA_VOWEL_RE.test(base) ? 'vowel' : 'consonant';
  }

  function stressOf(ph) {
    const m = ph.match(/([012])$/);
    return m ? m[1] : '';
  }
</script>

<!-- ===== HEADER ===== -->
<header>
  <div class="header-inner">
    <div class="brand">
      <span class="brand-icon">ꓢ</span>
      <div>
        <h1>sosap</h1>
        <span class="sub">Grapheme-to-Phoneme · WebAssembly</span>
      </div>
    </div>
    <div class="header-chips">
      <span class="chip green">● Live</span>
      <span class="chip">Rust → WASM</span>
      <button class="chip btn-change" on:click={() => dispatch('reset')}>↩ Change model</button>
    </div>
  </div>
</header>

<!-- ===== MAIN ===== -->
<main>
  <!-- ---- Controls Panel ---- -->
  <section class="controls-panel glass">
    <!-- Tab switcher -->
    <div class="tabs">
      <button class="tab {activeTab === 'single' ? 'active' : ''}" on:click={() => activeTab = 'single'}>
        Single / Multi-word
      </button>
      <button class="tab {activeTab === 'batch' ? 'active' : ''}" on:click={() => activeTab = 'batch'}>
        Batch (line-by-line)
      </button>
    </div>

    {#if activeTab === 'single'}
      <div class="input-row">
        <div class="input-wrap">
          <span class="input-icon">🔤</span>
          <input
            id="word-input"
            type="text"
            placeholder="Type a word or phrase…"
            bind:value={inputText}
            on:keydown={handleKeydown}
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        <button class="btn-run" on:click={runSingle} disabled={isProcessing || !inputText.trim()}>
          {#if isProcessing}
            <span class="spin">⟳</span>
          {:else}
            Phoneticize →
          {/if}
        </button>
      </div>
    {:else}
      <div class="batch-row">
        <textarea
          id="batch-input"
          rows="6"
          placeholder="One word per line…"
          bind:value={batchInput}
          on:keydown={handleKeydown}
          spellcheck="false"
        ></textarea>
        <button class="btn-run" on:click={runBatch} disabled={isProcessing}>
          {#if isProcessing}
            <span class="spin">⟳</span> Processing…
          {:else}
            Run Batch →
          {/if}
        </button>
      </div>
    {/if}

    <!-- N-best slider -->
    <div class="slider-row">
      <label for="nbest-slider">N-best paths: <strong>{nbest}</strong></label>
      <input id="nbest-slider" type="range" min="1" max="8" bind:value={nbest} />
    </div>

    <!-- Quick-try demos -->
    <div class="demo-row">
      <span class="demo-label">Quick try:</span>
      {#each demos as word}
        <button class="demo-chip" on:click={() => tryDemo(word)}>{word}</button>
      {/each}
    </div>
  </section>

  <!-- ---- Results ---- -->
  {#if results.length > 0}
    <section class="results-section">
      <div class="results-header">
        <h2>{results.length} result{results.length !== 1 ? 's' : ''}</h2>
        <button class="btn-clear" on:click={clearResults}>Clear ✕</button>
      </div>

      <div class="results-grid">
        {#each results as r (r.word)}
          <article class="result-card glass">
            <!-- Word title -->
            <div class="card-word">{r.word}</div>

            <!-- Top-1 phoneme display -->
            <div class="phoneme-row top1">
              {#if r.phonemes.length > 0}
                {#each r.phonemes as ph}
                  <span class="phoneme {phonemeClass(ph)}">
                    {ph.replace(/[012]$/, '')}
                    {#if stressOf(ph) === '1'}<sup class="stress primary">1</sup>
                    {:else if stressOf(ph) === '2'}<sup class="stress secondary">2</sup>
                    {/if}
                  </span>
                {/each}
              {:else}
                <span class="no-result">— no phonemes found —</span>
              {/if}
            </div>

            <!-- N-best alternatives -->
            {#if r.paths.length > 1}
              <div class="nbest-section">
                <p class="nbest-title">N-best ({r.paths.length})</p>
                {#each r.paths as path, i}
                  <div class="nbest-row {i === 0 ? 'best' : ''}">
                    <span class="nbest-rank">#{i + 1}</span>
                    <span class="nbest-phones">{path}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </article>
        {/each}
      </div>
    </section>
  {:else if !isProcessing}
    <!-- Empty state -->
    <div class="empty-state">
      <div class="empty-icon">🎙️</div>
      <p>Enter a word above to see its phonetic transcription</p>
      <p class="empty-sub">Your G2P model is loaded and running entirely in your browser via WebAssembly — no data leaves your device.</p>
    </div>
  {/if}
</main>

<!-- ===== FOOTER ===== -->
<footer>
  <span>sosap · MIT License · <a href="https://github.com/seanghay/sosap" target="_blank" rel="noreferrer">GitHub ↗</a></span>
  <span>WASM: ~470 KB · Model runs in-browser</span>
</footer>

<style>
  /* ===== Font ===== */
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;600&display=swap');

  /* ===== Layout ===== */
  header {
    background: rgba(10,14,26,0.9);
    backdrop-filter: blur(16px);
    border-bottom: 1px solid rgba(99,102,241,0.15);
    position: sticky;
    top: 0;
    z-index: 100;
    padding: 0 1.5rem;
  }

  .header-inner {
    max-width: 1000px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    height: 64px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .brand-icon {
    font-size: 1.8rem;
    line-height: 1;
    background: linear-gradient(135deg, #6366f1 0%, #a78bfa 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    font-weight: 900;
  }

  h1 {
    font-size: 1.35rem;
    font-weight: 700;
    color: #e0e7ff;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }

  .sub {
    font-size: 0.7rem;
    color: #475569;
    letter-spacing: 0.04em;
  }

  .header-chips {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .chip {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 999px;
    padding: 0.25em 0.75em;
    font-size: 0.72rem;
    color: #94a3b8;
    letter-spacing: 0.03em;
    white-space: nowrap;
  }

  .btn-change {
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s, color 0.15s;
  }
  .btn-change:hover {
    background: rgba(99,102,241,0.15);
    color: #a5b4fc;
    border-color: rgba(99,102,241,0.4);
  }

  .chip.green {
    color: #4ade80;
    border-color: rgba(74,222,128,0.3);
    background: rgba(74,222,128,0.07);
  }

  main {
    max-width: 1000px;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  /* ===== Glass card ===== */
  .glass {
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 18px;
    backdrop-filter: blur(12px);
  }

  /* ===== Controls Panel ===== */
  .controls-panel {
    padding: 1.75rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .tabs {
    display: flex;
    gap: 0.4rem;
    background: rgba(0,0,0,0.3);
    border-radius: 10px;
    padding: 4px;
    width: fit-content;
  }

  .tab {
    background: transparent;
    border: none;
    color: #64748b;
    padding: 0.45em 1.1em;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    font-family: inherit;
  }

  .tab:hover { color: #cbd5e1; }
  .tab.active {
    background: linear-gradient(135deg, #4f46e5, #7c3aed);
    color: white;
    box-shadow: 0 2px 8px rgba(99,102,241,0.4);
  }

  .input-row {
    display: flex;
    gap: 0.75rem;
    align-items: stretch;
  }

  .input-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .input-icon {
    position: absolute;
    left: 1rem;
    font-size: 1rem;
    pointer-events: none;
  }

  input[type="text"] {
    width: 100%;
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 12px;
    padding: 0.85rem 1rem 0.85rem 2.75rem;
    color: #e2e8f4;
    font-size: 1rem;
    font-family: 'Inter', inherit;
    outline: none;
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  input[type="text"]:focus {
    border-color: rgba(99,102,241,0.6);
    box-shadow: 0 0 0 3px rgba(99,102,241,0.15);
  }

  input[type="text"]::placeholder { color: #334155; }

  textarea {
    flex: 1;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 12px;
    padding: 0.85rem 1rem;
    color: #e2e8f4;
    font-size: 0.92rem;
    font-family: 'JetBrains Mono', monospace;
    resize: vertical;
    outline: none;
    width: 100%;
    transition: border-color 0.2s;
    line-height: 1.7;
  }

  textarea:focus {
    border-color: rgba(99,102,241,0.6);
    box-shadow: 0 0 0 3px rgba(99,102,241,0.15);
  }

  .batch-row {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .btn-run {
    background: linear-gradient(135deg, #4f46e5, #7c3aed);
    border: none;
    color: white;
    padding: 0.85rem 1.75rem;
    border-radius: 12px;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    font-family: inherit;
    transition: opacity 0.2s, transform 0.15s, box-shadow 0.2s;
    box-shadow: 0 4px 16px rgba(99,102,241,0.3);
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .btn-run:hover:not(:disabled) {
    opacity: 0.9;
    transform: translateY(-1px);
    box-shadow: 0 6px 20px rgba(99,102,241,0.45);
  }

  .btn-run:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    transform: none;
  }

  .spin {
    display: inline-block;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  /* ===== Slider ===== */
  .slider-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .slider-row label {
    font-size: 0.83rem;
    color: #64748b;
    min-width: 140px;
  }

  .slider-row strong { color: #a5b4fc; }

  input[type="range"] {
    flex: 1;
    accent-color: #6366f1;
    height: 4px;
    min-width: 120px;
  }

  /* ===== Demo chips ===== */
  .demo-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .demo-label {
    font-size: 0.8rem;
    color: #475569;
  }

  .demo-chip {
    background: rgba(99,102,241,0.1);
    border: 1px solid rgba(99,102,241,0.25);
    border-radius: 999px;
    padding: 0.3em 0.85em;
    color: #818cf8;
    font-size: 0.8rem;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s, transform 0.15s;
  }

  .demo-chip:hover {
    background: rgba(99,102,241,0.2);
    transform: translateY(-1px);
  }

  /* ===== Results ===== */
  .results-section { display: flex; flex-direction: column; gap: 1.25rem; }

  .results-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.25rem;
  }

  .results-header h2 {
    font-size: 1rem;
    color: #64748b;
    font-weight: 500;
  }

  .btn-clear {
    background: none;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    color: #475569;
    padding: 0.35em 0.9em;
    font-size: 0.8rem;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s;
  }

  .btn-clear:hover { color: #ef4444; border-color: rgba(239,68,68,0.4); }

  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1rem;
  }

  /* ===== Result Card ===== */
  .result-card {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    animation: fadeUp 0.3s ease both;
    transition: transform 0.2s, border-color 0.2s;
  }

  .result-card:hover {
    transform: translateY(-2px);
    border-color: rgba(99,102,241,0.3);
  }

  @keyframes fadeUp {
    from { opacity: 0; transform: translateY(10px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .card-word {
    font-size: 1.4rem;
    font-weight: 700;
    color: #e2e8f4;
    letter-spacing: -0.01em;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    padding-bottom: 0.75rem;
  }

  /* ===== Phoneme badges ===== */
  .phoneme-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
  }

  .phoneme {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.9rem;
    font-weight: 600;
    padding: 0.3em 0.55em;
    border-radius: 7px;
    position: relative;
    transition: transform 0.15s;
  }

  .phoneme:hover { transform: scale(1.08); }

  .phoneme.vowel {
    background: rgba(99,102,241,0.18);
    color: #a5b4fc;
    border: 1px solid rgba(99,102,241,0.3);
  }

  .phoneme.consonant {
    background: rgba(16,185,129,0.12);
    color: #6ee7b7;
    border: 1px solid rgba(16,185,129,0.25);
  }

  sup.stress {
    font-size: 0.6em;
    margin-left: 1px;
    font-weight: 700;
  }
  sup.primary { color: #f59e0b; }
  sup.secondary { color: #94a3b8; }

  .no-result {
    color: #334155;
    font-size: 0.85rem;
    font-style: italic;
  }

  /* ===== N-best ===== */
  .nbest-section {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .nbest-title {
    font-size: 0.72rem;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    margin-bottom: 0.2rem;
  }

  .nbest-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    border-radius: 8px;
    background: rgba(255,255,255,0.02);
    transition: background 0.15s;
  }

  .nbest-row:hover { background: rgba(255,255,255,0.05); }

  .nbest-row.best { background: rgba(99,102,241,0.08); }

  .nbest-rank {
    font-size: 0.7rem;
    color: #475569;
    font-family: 'JetBrains Mono', monospace;
    min-width: 1.8em;
  }

  .nbest-row.best .nbest-rank { color: #818cf8; }

  .nbest-phones {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.82rem;
    color: #94a3b8;
    word-break: break-all;
  }

  .nbest-row.best .nbest-phones { color: #c7d2fe; }

  /* ===== Empty state ===== */
  .empty-state {
    text-align: center;
    padding: 5rem 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
  }

  .empty-icon { font-size: 3rem; margin-bottom: 0.25rem; }

  .empty-state p {
    color: #475569;
    font-size: 0.95rem;
  }

  .empty-sub {
    color: #334155 !important;
    font-size: 0.8rem !important;
    max-width: 480px;
    line-height: 1.6;
  }

  /* ===== Footer ===== */
  footer {
    border-top: 1px solid rgba(255,255,255,0.05);
    padding: 1rem 1.5rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: #334155;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  footer a { color: #6366f1; text-decoration: none; }
  footer a:hover { text-decoration: underline; }

  /* ===== Responsive ===== */
  @media (max-width: 600px) {
    .header-chips { display: none; }
    .results-grid { grid-template-columns: 1fr; }
    .input-row { flex-direction: column; }
  }
</style>
