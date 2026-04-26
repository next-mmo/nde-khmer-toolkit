<script>
  import { onMount, tick } from 'svelte';
  import { fmtTime, parseSrt } from './audio-transcribe/srt.js';

  const EDGE_TTS_BASE_URL = 'speech.platform.bing.com/consumer/speech/synthesize/readaloud';
  const EDGE_TTS_CLIENT_TOKEN = '6A5AA1D4EAFF4E9FB37E23D68491D6F4';
  const EDGE_TTS_FALLBACK_CHROMIUM_VERSION = '143.0.3650.75';
  const EDGE_TTS_API_URL = '/api/edge-tts';
  const TRANSFORMERS_JS_URL = 'https://cdn.jsdelivr.net/npm/@huggingface/transformers@4.1.0';
  const MMS_KHMER_MODEL = 'facebook/mms-tts-khm';
  const MMS_LOCAL_MODEL_PATH = '/models/';
  const DEFAULT_LANG = 'km-KH';
  const EDGE_VOICES = [
    {
      name: 'km-KH-PisethNeural',
      label: 'km-KH · Piseth',
      description: 'Khmer male',
      lang: DEFAULT_LANG,
    },
    {
      name: 'km-KH-SreymomNeural',
      label: 'km-KH · Sreymom',
      description: 'Khmer female',
      lang: DEFAULT_LANG,
    },
  ];
  const EDGE_TTS_BROWSER_ERROR = 'Edge TTS connection failed. Direct browser Edge TTS usually only works in Microsoft Edge because other browsers block the required WebSocket handshake. No browser voice fallback was used, so it will not read with an English system voice.';
  const EDGE_TTS_TIMEOUT_MS = 15000;
  const EDGE_TTS_IDLE_AFTER_AUDIO_MS = 1400;
  const EDGE_TTS_TIMEOUT_ERROR = 'Edge TTS did not return audio in 15 seconds. No browser voice fallback was used, so it will not read with an English system voice.';

  let supportsEdgeTts = isMicrosoftEdgeBrowser();
  let inputMode = 'text';
  let textInput = 'សួស្តី! នេះគឺជាការសាកល្បងសំឡេងខ្មែរ។';
  let srtInput = '';
  let fileName = '';
  let edgeVoiceName = 'km-KH-PisethNeural';
  let rate = 0;
  let pitch = 0;
  let status = 'idle';
  let errorMsg = '';
  let audioUrl = '';
  let audioElement;
  let srtCues = [];
  let activeCueIndex = -1;
  let activeWordFill = 0;
  let rafId = null;
  let currentUtterance = null;
  let mmsPipelinePromise = null;
  let synthRunId = 0;
  let cueStartedAt = 0;
  let cueDuration = 1;

  $: selectedEdgeVoice = EDGE_VOICES.find(v => v.name === edgeVoiceName) || EDGE_VOICES[0];
  $: edgeTtsMode = 'api';
  $: providerLabel = 'Edge TTS API';
  $: activeCue = activeCueIndex >= 0 ? srtCues[activeCueIndex] : null;
  $: canSynthesize = !!sourceText.trim()
    && status !== 'speaking'
    && status !== 'synthesizing';
  $: sourceText = getSourceText();

  onMount(() => {
    supportsEdgeTts = isMicrosoftEdgeBrowser();
    return () => {
      window.speechSynthesis?.cancel();
      stopTicker();
      cleanupAudioUrl();
    };
  });

  function isMicrosoftEdgeBrowser() {
    return typeof navigator !== 'undefined' && /\bEdg\//.test(navigator.userAgent);
  }

  function getSourceText() {
    if (inputMode === 'srt') return parseSrt(srtInput).map(c => c.text).join('\n');
    return textInput;
  }

  async function onFilePick(e) {
    const file = e.target.files?.[0];
    if (!file) return;
    fileName = file.name;
    const content = await file.text();
    if (file.name.toLowerCase().endsWith('.srt') || content.includes('-->')) {
      inputMode = 'srt';
      srtInput = content;
      srtCues = parseSrt(content);
    } else {
      inputMode = 'text';
      textInput = content;
      srtCues = [];
    }
  }

  async function synthesize() {
    if (!canSynthesize) return;
    errorMsg = '';
    window.speechSynthesis?.cancel();
    stopAudio();
    srtCues = inputMode === 'srt' ? parseSrt(srtInput) : buildLinearCues(textInput);
    if (!srtCues.length) {
      srtCues = [{ start: 0, end: estimateDuration(sourceText), text: sourceText }];
    }

    const runId = ++synthRunId;
    await synthesizeWithEdge(runId, 'api');
  }

  function stopSpeech() {
    synthRunId += 1;
    window.speechSynthesis?.cancel();
    stopAudio();
    currentUtterance = null;
    status = 'idle';
    activeCueIndex = -1;
    activeWordFill = 0;
    stopTicker();
  }

  function buildLinearCues(text) {
    const parts = text
      .split(/(?<=[។.!?])\s+|\n+/)
      .map(part => part.trim())
      .filter(Boolean);
    let cursor = 0;
    return parts.map(part => {
      const duration = estimateDuration(part);
      const cue = {
        start: cursor,
        end: cursor + duration,
        text: part,
      };
      cursor += duration;
      return cue;
    });
  }

  function estimateDuration(text) {
    const words = text.trim().split(/\s+/).filter(Boolean).length || Math.ceil(text.length / 8) || 1;
    return Math.max(1.2, words * 0.45);
  }

  async function synthesizeWithMmsWasm(runId) {
    status = 'synthesizing';
    activeCueIndex = -1;
    activeWordFill = 0;
    stopTicker();

    try {
      const synthesizer = await loadMmsPipeline();
      if (runId !== synthRunId) return;
      const output = await synthesizer(normalizeMmsText(sourceText));
      if (runId !== synthRunId) return;
      if (!output?.audio?.length || !output?.sampling_rate) {
        throw new Error('Local Khmer WASM TTS returned no audio.');
      }

      cleanupAudioUrl();
      audioUrl = URL.createObjectURL(floatAudioToWavBlob(output.audio, output.sampling_rate));
      await tick();
      currentUtterance = null;
      if (audioElement) {
        audioElement.currentTime = 0;
        try {
          await audioElement.play();
        } catch {
          status = 'idle';
        }
      }
    } catch (error) {
      if (runId !== synthRunId) return;
      errorMsg = normalizeMmsError(error);
      status = 'error';
      stopTicker();
    }
  }

  async function loadMmsPipeline() {
    mmsPipelinePromise ||= (async () => {
      const { pipeline, env } = await import(/* @vite-ignore */ TRANSFORMERS_JS_URL);
      const hasLocalModel = await hasLocalMmsModel();
      env.localModelPath = MMS_LOCAL_MODEL_PATH;
      env.allowLocalModels = hasLocalModel;
      env.allowRemoteModels = true;
      return pipeline('text-to-speech', MMS_KHMER_MODEL, {
        device: 'wasm',
        dtype: 'q8',
      });
    })();
    return mmsPipelinePromise;
  }

  async function hasLocalMmsModel() {
    try {
      const response = await fetch(`${MMS_LOCAL_MODEL_PATH}${MMS_KHMER_MODEL}/config.json`, {
        cache: 'no-store',
      });
      if (!response.ok) return false;
      const contentType = response.headers.get('content-type') || '';
      if (!contentType.includes('json')) return false;
      const config = await response.json();
      return config?.model_type === 'vits';
    } catch {
      return false;
    }
  }

  function normalizeMmsText(text) {
    return text
      .replace(/[“”"]/g, '')
      .replace(/[!?.,;:]+/g, ' ')
      .replace(/[។៕]+/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
  }

  function normalizeMmsError(error) {
    const message = error?.message || String(error);
    if (/onnx|model|could not locate|404|no such file|unexpected token/i.test(message)) {
      return 'Local Khmer WASM needs browser-compatible ONNX files at public/models/facebook/mms-tts-khm/onnx/. The app can run WASM TTS, but the public Khmer MMS checkpoint is not packaged for Transformers.js yet.';
    }
    return message;
  }

  async function synthesizeWithEdge(runId, transport = 'direct') {
    status = 'synthesizing';
    activeCueIndex = -1;
    activeWordFill = 0;
    stopTicker();

    try {
      const result = await synthesizeEdgeStream({
        text: sourceText,
        voice: selectedEdgeVoice.name,
        lang: selectedEdgeVoice.lang,
        rate: formatPercent(rate),
        volume: '+0%',
        pitch: formatPitch(pitch),
        transport,
      });
      if (runId !== synthRunId) return;
      if (!result.audio || result.audio.size === 0) {
        throw new Error('No audio received from Edge TTS.');
      }

      cleanupAudioUrl();
      audioUrl = URL.createObjectURL(result.audio);
      await tick();
      currentUtterance = null;
      if (audioElement) {
        audioElement.currentTime = 0;
        try {
          await audioElement.play();
        } catch {
          status = 'idle';
        }
      }
    } catch (error) {
      if (runId !== synthRunId) return;
      errorMsg = normalizeEdgeError(error);
      status = 'error';
      stopTicker();
    }
  }

  async function synthesizeEdgeStream(options) {
    if (options.transport === 'api') {
      return synthesizeEdgeApi(options);
    }

    const audioChunks = [];
    const wordBoundaries = [];
    const connectionId = createConnectionId();
    const secMsGec = await generateSecMsGec();
    const gecVersion = getEdgeGecVersion();
    const url = `wss://${EDGE_TTS_BASE_URL}/edge/v1?TrustedClientToken=${EDGE_TTS_CLIENT_TOKEN}&Sec-MS-GEC=${secMsGec}&Sec-MS-GEC-Version=${gecVersion}&ConnectionId=${connectionId}`;

    return new Promise((resolve, reject) => {
      const websocket = new WebSocket(url);
      websocket.binaryType = 'arraybuffer';
      let settled = false;
      let timeoutId;
      let idleId;

      const clearTimers = () => {
        clearTimeout(timeoutId);
        clearTimeout(idleId);
      };
      const finish = () => {
        if (settled) return;
        settled = true;
        clearTimers();
        websocket.close();
        if (!audioChunks.length) {
          reject(new Error(EDGE_TTS_TIMEOUT_ERROR));
          return;
        }
        resolve({
          audio: new Blob(audioChunks, { type: 'audio/mpeg' }),
          subtitle: wordBoundaries,
        });
      };
      const fail = (error) => {
        if (settled) return;
        settled = true;
        clearTimers();
        websocket.close();
        reject(error);
      };
      const armTimeout = () => {
        timeoutId = setTimeout(() => {
          if (audioChunks.length) finish();
          else fail(new Error(EDGE_TTS_TIMEOUT_ERROR));
        }, EDGE_TTS_TIMEOUT_MS);
      };
      const armIdleFinish = () => {
        clearTimeout(idleId);
        idleId = setTimeout(finish, EDGE_TTS_IDLE_AFTER_AUDIO_MS);
      };

      websocket.onopen = () => {
        websocket.send(createSpeechConfigMessage());
        websocket.send(createSsmlMessage(options));
      };
      websocket.onmessage = async (event) => {
        try {
          const data = event.data instanceof Blob ? await event.data.arrayBuffer() : event.data;
          if (typeof data === 'string') {
            const { headers, body } = parseEdgeTextMessage(data);
            if (headers.Path === 'turn.end') {
              finish();
            } else if (headers.Path === 'audio.metadata') {
              wordBoundaries.push(...parseWordBoundaries(body));
            }
            return;
          }

          const { headers, audioData } = parseEdgeBinaryMessage(new Uint8Array(data));
          if (headers.Path === 'audio' && headers['Content-Type'] === 'audio/mpeg' && audioData.length) {
            audioChunks.push(audioData);
            armIdleFinish();
          }
        } catch (error) {
          fail(error);
        }
      };
      websocket.onerror = () => fail(new Error(EDGE_TTS_BROWSER_ERROR));
      websocket.onclose = () => {
        if (!settled) finish();
      };
      armTimeout();
    });
  }

  async function synthesizeEdgeApi(options) {
    const response = await fetch(EDGE_TTS_API_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        text: options.text,
        voice: options.voice,
        rate: options.rate,
        pitch: options.pitch,
      }),
    });

    if (!response.ok) {
      const message = await readApiError(response);
      throw new Error(message || `Edge TTS API failed (${response.status})`);
    }

    return {
      audio: await response.blob(),
      subtitle: [],
    };
  }

  async function readApiError(response) {
    const contentType = response.headers.get('content-type') || '';
    if (contentType.includes('application/json')) {
      const body = await response.json().catch(() => null);
      return body?.error || '';
    }
    return response.text().catch(() => '');
  }

  function normalizeEdgeError(error) {
    const message = error?.message || String(error);
    if (message === EDGE_TTS_TIMEOUT_ERROR) return message;
    if (/websocket|network|failed|close|event/i.test(message)) return EDGE_TTS_BROWSER_ERROR;
    return message;
  }

  function createSpeechConfigMessage() {
    return `X-Timestamp:${formatEdgeDate()}\r
Content-Type:application/json; charset=utf-8\r
Path:speech.config\r
\r
{"context":{"synthesis":{"audio":{"metadataoptions":{"sentenceBoundaryEnabled":"false","wordBoundaryEnabled":"true"},"outputFormat":"audio-24khz-48kbitrate-mono-mp3"}}}}\r
`;
  }

  function createSsmlMessage({ text, voice, lang, rate, volume, pitch }) {
    const requestId = createConnectionId();
    const voiceName = toFullEdgeVoiceName(voice);
    const timestamp = `${formatEdgeDate()}Z`;
    const ssml = `<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='${lang}'><voice name='${voiceName}'><prosody pitch='${pitch}' rate='${rate}' volume='${volume}'>${escapeXml(text)}</prosody></voice></speak>`;
    return `X-RequestId:${requestId}\r
Content-Type:application/ssml+xml\r
X-Timestamp:${timestamp}\r
Path:ssml\r
\r
${ssml}`;
  }

  function toFullEdgeVoiceName(shortName) {
    const match = /^([a-z]{2,})-([A-Z]{2,})-(.+Neural)$/.exec(shortName);
    if (!match) return shortName;
    return `Microsoft Server Speech Text to Speech Voice (${match[1]}-${match[2]}, ${match[3]})`;
  }

  function parseEdgeTextMessage(message) {
    const splitIndex = message.indexOf('\r\n\r\n');
    const headerText = splitIndex >= 0 ? message.slice(0, splitIndex) : message;
    const headers = Object.fromEntries(
      headerText
        .split('\r\n')
        .map(line => line.split(/:(.*)/s).slice(0, 2))
        .filter(([key, value]) => key && value != null)
        .map(([key, value]) => [key, value.trim()])
    );
    return {
      headers,
      body: splitIndex >= 0 ? message.slice(splitIndex + 4) : '',
    };
  }

  function parseEdgeBinaryMessage(buffer) {
    if (buffer.length < 2) return { headers: {}, audioData: new Uint8Array() };
    const headerLength = (buffer[0] << 8) | buffer[1];
    const headerText = new TextDecoder().decode(buffer.slice(2, 2 + headerLength));
    const headers = Object.fromEntries(
      headerText
        .split('\r\n')
        .map(line => line.split(/:(.*)/s).slice(0, 2))
        .filter(([key, value]) => key && value != null)
        .map(([key, value]) => [key, value.trim()])
    );
    return {
      headers,
      audioData: buffer.slice(2 + headerLength),
    };
  }

  function parseWordBoundaries(body) {
    try {
      const metadata = JSON.parse(body);
      return (metadata.Metadata || [])
        .filter(item => item.Type === 'WordBoundary' && item.Data?.text?.Text)
        .map(item => ({
          offset: item.Data.Offset,
          duration: item.Data.Duration,
          text: item.Data.text.Text,
        }));
    } catch {
      return [];
    }
  }

  async function generateSecMsGec() {
    const winEpoch = 11644473600;
    let ticks = Date.now() / 1000 + winEpoch;
    ticks -= ticks % 300;
    ticks *= 10000000;
    const data = new TextEncoder().encode(`${ticks.toFixed(0)}${EDGE_TTS_CLIENT_TOKEN}`);
    const hash = await crypto.subtle.digest('SHA-256', data);
    return Array.from(new Uint8Array(hash)).map(byte => byte.toString(16).padStart(2, '0')).join('').toUpperCase();
  }

  function createConnectionId(uppercase = false) {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    const id = Array.from(bytes).map(byte => byte.toString(16).padStart(2, '0')).join('');
    return uppercase ? id.toUpperCase() : id;
  }

  function getEdgeGecVersion() {
    const fullVersion = getBrowserFullVersion() || EDGE_TTS_FALLBACK_CHROMIUM_VERSION;
    return `1-${fullVersion}`;
  }

  function getBrowserFullVersion() {
    if (typeof navigator === 'undefined') return '';
    const ua = navigator.userAgent || '';
    const edgeMatch = /\bEdg\/([\d.]+)/.exec(ua);
    if (edgeMatch?.[1]) return edgeMatch[1];
    return '';
  }

  function formatEdgeDate() {
    return new Date().toUTCString().replace('GMT', 'GMT+0000 (Coordinated Universal Time)');
  }

  function escapeXml(value) {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&apos;');
  }

  function floatAudioToWavBlob(audio, sampleRate) {
    const samples = audio instanceof Float32Array ? audio : new Float32Array(audio);
    const buffer = new ArrayBuffer(44 + samples.length * 2);
    const view = new DataView(buffer);
    writeString(view, 0, 'RIFF');
    view.setUint32(4, 36 + samples.length * 2, true);
    writeString(view, 8, 'WAVE');
    writeString(view, 12, 'fmt ');
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true);
    view.setUint16(22, 1, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * 2, true);
    view.setUint16(32, 2, true);
    view.setUint16(34, 16, true);
    writeString(view, 36, 'data');
    view.setUint32(40, samples.length * 2, true);

    let offset = 44;
    for (const sample of samples) {
      const clamped = Math.max(-1, Math.min(1, sample));
      view.setInt16(offset, clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff, true);
      offset += 2;
    }
    return new Blob([buffer], { type: 'audio/wav' });
  }

  function writeString(view, offset, value) {
    for (let i = 0; i < value.length; i += 1) {
      view.setUint8(offset + i, value.charCodeAt(i));
    }
  }

  function formatPercent(value) {
    return `${value >= 0 ? '+' : ''}${value}%`;
  }

  function formatPitch(value) {
    return `${value >= 0 ? '+' : ''}${Math.round(value * 2)}Hz`;
  }

  function stopAudio() {
    if (audioElement) {
      audioElement.pause();
      audioElement.currentTime = 0;
    }
  }

  function cleanupAudioUrl() {
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      audioUrl = '';
    }
  }

  function speakCue(index) {
    const cue = srtCues[index];
    if (!cue) {
      status = 'done';
      stopTicker();
      activeWordFill = 1;
      return;
    }

    activeCueIndex = index;
    activeWordFill = 0;
    cueStartedAt = performance.now();
    cueDuration = Math.max(0.4, cue.end - cue.start || estimateDuration(cue.text));
    status = 'speaking';

    const utterance = new SpeechSynthesisUtterance(cue.text);
    const voice = selectedVoice?.voice;
    if (voice) utterance.voice = voice;
    utterance.lang = selectedVoice?.lang || DEFAULT_LANG;
    utterance.rate = Math.max(0.5, Math.min(2, 1 + rate / 100));
    utterance.pitch = Math.max(0, Math.min(2, 1 + pitch / 100));
    utterance.onend = () => speakCue(index + 1);
    utterance.onerror = (event) => {
      errorMsg = event.error || 'Speech synthesis failed';
      status = 'error';
      stopTicker();
    };
    currentUtterance = utterance;
    startTicker();
    window.speechSynthesis.speak(utterance);
  }

  function startAudioTicker() {
    stopTicker();
    const tickAudio = () => {
      const time = audioElement?.currentTime || 0;
      const index = srtCues.findIndex(cue => time >= cue.start && time < cue.end);
      if (index >= 0) {
        const cue = srtCues[index];
        activeCueIndex = index;
        activeWordFill = Math.min(1, Math.max(0, (time - cue.start) / Math.max(0.4, cue.end - cue.start)));
      }
      rafId = requestAnimationFrame(tickAudio);
    };
    rafId = requestAnimationFrame(tickAudio);
  }

  function handleAudioPlay() {
    status = 'speaking';
    startAudioTicker();
  }

  function handleAudioPause() {
    if (status === 'speaking' && !audioElement?.ended) status = 'idle';
    stopTicker();
  }

  function handleAudioEnded() {
    status = 'done';
    activeWordFill = 1;
    stopTicker();
  }

  function startTicker() {
    stopTicker();
    const tick = () => {
      activeWordFill = Math.min(1, (performance.now() - cueStartedAt) / (cueDuration * 1000));
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
  }

  function stopTicker() {
    if (rafId != null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  function seekCue(cue, index) {
    window.speechSynthesis?.cancel();
    srtCues = srtCues.length ? srtCues : [{
      start: 0,
      end: estimateDuration(sourceText),
      text: sourceText,
    }];
    if (audioElement && audioUrl) {
      audioElement.currentTime = cue.start;
      activeCueIndex = index;
      activeWordFill = 0;
      audioElement.play().catch(() => {
        status = 'idle';
      });
      return;
    }
    activeCueIndex = index;
    activeWordFill = 0;
  }
</script>

<header>
  <div class="header-inner">
    <div class="brand">
      <span class="brand-icon">🔊</span>
      <div>
        <h1>Text → Speech</h1>
        <span class="sub">Edge TTS · SRT preview</span>
      </div>
    </div>
    <div class="header-chips">
      <span class="chip green">Provider: {providerLabel}</span>
      <span class="chip">Serverless</span>
    </div>
  </div>
</header>

<main>
  <section class="glass controls-panel">
    <div class="control-grid single">
      <label>
        <span>Voice</span>
        <select bind:value={edgeVoiceName}>
          {#each EDGE_VOICES as voice}
            <option value={voice.name}>{voice.label}</option>
          {/each}
        </select>
      </label>
    </div>

    <div class="range-grid">
      <label>
        <span>Rate {rate}%</span>
        <input type="range" min="-50" max="50" step="5" bind:value={rate} />
      </label>
      <label>
        <span>Pitch {pitch}%</span>
        <input type="range" min="-50" max="50" step="5" bind:value={pitch} />
      </label>
    </div>

    <p class="note">
      {selectedEdgeVoice.description} · {selectedEdgeVoice.name}
      · via Vercel serverless API
    </p>
  </section>

  <section class="glass editor-panel">
    <div class="mode-row">
      <button class:active={inputMode === 'text'} on:click={() => inputMode = 'text'}>Text</button>
      <button class:active={inputMode === 'srt'} on:click={() => { inputMode = 'srt'; srtCues = parseSrt(srtInput); }}>SRT</button>
      <label class="file-btn">
        File
        <input type="file" accept=".txt,.srt,text/plain,application/x-subrip" on:change={onFilePick} />
      </label>
      {#if fileName}<span class="file-name">{fileName}</span>{/if}
    </div>

    {#if inputMode === 'text'}
      <textarea bind:value={textInput} rows="8" spellcheck="false"></textarea>
    {:else}
      <textarea
        bind:value={srtInput}
        rows="10"
        spellcheck="false"
        on:input={() => srtCues = parseSrt(srtInput)}
      ></textarea>
    {/if}

    <div class="action-row">
      <button class="btn-primary" disabled={!canSynthesize} on:click={synthesize}>
        {status === 'synthesizing' ? 'Synthesizing...' : status === 'speaking' ? 'Speaking...' : 'Speak'}
      </button>
      {#if status === 'speaking' || status === 'synthesizing'}
        <button class="btn-secondary" on:click={stopSpeech}>Stop</button>
      {/if}
    </div>
  </section>

  {#if audioUrl}
    <section class="glass audio-panel">
      <audio
        bind:this={audioElement}
        src={audioUrl}
        controls
        on:play={handleAudioPlay}
        on:pause={handleAudioPause}
        on:ended={handleAudioEnded}
      ></audio>
    </section>
  {/if}

  {#if status === 'error'}
    <section class="glass error-section">
      <span class="error-icon">⚠️</span>
      <div>
        <p class="error-title">Synthesis failed</p>
        <p class="error-body">{errorMsg}</p>
      </div>
    </section>
  {/if}

  {#if srtCues.length}
    <section class="glass player-panel">
      <div class="karaoke-stage">
        <p class="karaoke-meta">SRT Player</p>
        <div class="karaoke-line">
          {#if activeCue}
            <span class="kw-now">
              <span class="kw-base">{activeCue.text}</span>
              <span
                class="kw-fill"
                style="clip-path: inset(0 {(1 - activeWordFill) * 100}% 0 0); -webkit-clip-path: inset(0 {(1 - activeWordFill) * 100}% 0 0);"
                aria-hidden="true"
              >{activeCue.text}</span>
            </span>
          {:else}
            <span class="karaoke-idle">Ready</span>
          {/if}
        </div>
      </div>

      {#if srtCues.length}
        <div class="cue-list">
          {#each srtCues as cue, index}
            <button class="cue-item" class:active={index === activeCueIndex} on:click={() => seekCue(cue, index)}>
              <span class="cue-time">{fmtTime(cue.start)}</span>
              <span class="cue-text">{cue.text}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</main>

<style>
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
    max-width: 960px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 64px;
    gap: 1rem;
  }

  .brand { display: flex; align-items: center; gap: 0.75rem; }
  .brand-icon { font-size: 1.8rem; line-height: 1; }
  h1 { font-size: 1.3rem; color: #e0fdf4; line-height: 1.1; }
  .sub { font-size: 0.7rem; color: #64748b; letter-spacing: 0.04em; }

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

  main {
    max-width: 960px;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .glass {
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 18px;
    backdrop-filter: blur(12px);
  }

  .controls-panel, .editor-panel, .player-panel, .audio-panel {
    padding: 1.25rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .audio-panel audio { width: 100%; }

  .control-grid, .range-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }

  label span, .note {
    font-size: 0.78rem;
    color: #64748b;
  }

  select, textarea {
    width: 100%;
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 10px;
    color: #e2e8f4;
    font-family: inherit;
    outline: none;
  }

  select { padding: 0.6rem 0.8rem; }
  textarea {
    min-height: 220px;
    resize: vertical;
    padding: 1rem;
    line-height: 1.7;
    font-size: 1rem;
  }

  input[type="range"] { width: 100%; accent-color: #14b8a6; }

  .mode-row, .action-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .mode-row button, .file-btn, .btn-secondary {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.12);
    color: #94a3b8;
    padding: 0.65rem 1rem;
    border-radius: 10px;
    font-size: 0.85rem;
    cursor: pointer;
    font-family: inherit;
    text-decoration: none;
  }
  .mode-row button.active {
    color: #ccfbf1;
    border-color: rgba(20,184,166,0.45);
    background: rgba(20,184,166,0.1);
  }
  .file-btn input { display: none; }
  .file-name { color: #64748b; font-size: 0.78rem; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .btn-primary {
    flex: 1;
    min-width: 190px;
    background: linear-gradient(135deg, #0d9488, #0891b2);
    border: none;
    color: white;
    padding: 0.85rem 1.5rem;
    border-radius: 12px;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    font-family: inherit;
  }
  .btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

  .karaoke-stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.65rem;
    min-height: 120px;
    padding: 1.15rem;
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
  }
  .karaoke-line {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 3.2rem;
    width: 100%;
  }
  .kw-now {
    position: relative;
    display: inline-block;
    max-width: 100%;
    color: #1e3a3a;
    font-size: 1.35rem;
    font-weight: 600;
    line-height: 1.7;
    word-break: break-word;
  }
  .kw-fill {
    position: absolute;
    inset: 0;
    color: #ffffff;
    text-shadow: 0 0 14px rgba(20,184,166,0.9);
    pointer-events: none;
  }
  .karaoke-idle { color: #475569; }

  .cue-list {
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cue-item {
    display: flex;
    gap: 0.75rem;
    align-items: baseline;
    padding: 0.45rem 0.75rem;
    border-radius: 8px;
    border: 1px solid transparent;
    background: transparent;
    color: inherit;
    cursor: pointer;
    text-align: left;
  }
  .cue-item:hover { background: rgba(255,255,255,0.04); }
  .cue-item.active { background: rgba(20,184,166,0.08); border-color: rgba(20,184,166,0.22); }
  .cue-time {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.72rem;
    color: #475569;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .cue-text { color: #94a3b8; line-height: 1.6; word-break: break-word; }

  .error-section {
    padding: 1.25rem 1.5rem;
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    background: rgba(239,68,68,0.06);
    border-color: rgba(239,68,68,0.2);
  }
  .error-title { font-size: 0.9rem; font-weight: 600; color: #fca5a5; }
  .error-body { font-size: 0.82rem; color: #fca5a5; opacity: 0.75; margin-top: 0.2rem; word-break: break-word; }

  @media (max-width: 700px) {
    .header-chips { display: none; }
    .control-grid, .range-grid { grid-template-columns: 1fr; }
  }
</style>
