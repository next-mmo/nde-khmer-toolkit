<script>
  import { createEventDispatcher, onMount } from 'svelte';
  const dispatch = createEventDispatcher();

  let isDragging = false;
  let fileError = null;

  // Per-model state: { pct, loading, error, cached, cachedSize }
  let modelState = {};

  const MODELS = [
    {
      id: 'khmer',
      flag: '🇰🇭',
      label: 'Khmer G2P',
      author: 'seanghay',
      size: '~54 MB',
      example: "phoneticize('ក') → ['k']",
      url: 'https://huggingface.co/seanghay/khmer-g2p/resolve/main/model.fst',
      hfUrl: 'https://huggingface.co/seanghay/khmer-g2p',
    },
    {
      id: 'english',
      flag: '🇺🇸',
      label: 'English CMUdict',
      author: 'AdolfVonKleist',
      size: '~44 MB',
      example: "phoneticize('hello') → ARPAbet",
      url: 'https://media.githubusercontent.com/media/AdolfVonKleist/phonetisaurus-downloads/master/models/cmudict-20170708.o8.fst',
      hfUrl: 'https://github.com/AdolfVonKleist/phonetisaurus-downloads',
    },
  ];

  // ── IndexedDB helpers ──────────────────────────────────────────────────────
  const DB_NAME = 'sosap-model-cache';
  const DB_VERSION = 1;
  const STORE = 'models';

  function openDB() {
    return new Promise((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = e => e.target.result.createObjectStore(STORE);
      req.onsuccess = e => resolve(e.target.result);
      req.onerror = e => reject(e.target.error);
    });
  }

  async function dbGet(id) {
    const db = await openDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE, 'readonly');
      const req = tx.objectStore(STORE).get(id);
      req.onsuccess = e => resolve(e.target.result ?? null);
      req.onerror = e => reject(e.target.error);
    });
  }

  async function dbPut(id, value) {
    const db = await openDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      const req = tx.objectStore(STORE).put(value, id);
      req.onsuccess = () => resolve();
      req.onerror = e => reject(e.target.error);
    });
  }

  async function dbDelete(id) {
    const db = await openDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      const req = tx.objectStore(STORE).delete(id);
      req.onsuccess = () => resolve();
      req.onerror = e => reject(e.target.error);
    });
  }
  // ──────────────────────────────────────────────────────────────────────────

  // On mount: check which models are already cached
  onMount(async () => {
    for (const m of MODELS) {
      try {
        const cached = await dbGet(m.id);
        if (cached) {
          modelState = { ...modelState, [m.id]: {
            loading: false, error: null, pct: 0,
            cached: true, cachedSize: fmt(cached.byteLength),
          }};
        }
      } catch { /* IDB not available — ignore */ }
    }
  });

  function fmt(bytes) {
    return bytes > 1e6 ? (bytes / 1e6).toFixed(1) + ' MB' : (bytes / 1024).toFixed(0) + ' KB';
  }

  // Load bytes directly from IDB cache
  async function loadCached(model) {
    modelState = { ...modelState, [model.id]: { ...modelState[model.id], loading: true } };
    try {
      const bytes = await dbGet(model.id);
      const file = new File([bytes], `${model.id}.fst`, { type: 'application/octet-stream' });
      dispatch('file', file);
    } catch(e) {
      modelState = { ...modelState, [model.id]: { ...modelState[model.id], loading: false, error: e.message } };
    }
  }

  // Download from network, cache in IDB, then dispatch
  async function downloadAndLoad(model) {
    modelState = { ...modelState, [model.id]: { pct: 0, loading: true, error: null, cached: false } };
    try {
      const res = await fetch(model.url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const total = parseInt(res.headers.get('content-length') || '0', 10);
      const reader = res.body.getReader();
      const chunks = [];
      let received = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        received += value.length;
        const pct = total > 0 ? Math.round((received / total) * 100) : -1;
        modelState = { ...modelState, [model.id]: { pct, loading: true, error: null, cached: false } };
      }
      const bytes = new Uint8Array(received);
      let off = 0;
      for (const c of chunks) { bytes.set(c, off); off += c.length; }
      // Save to IDB
      try { await dbPut(model.id, bytes); } catch { /* storage quota — silently skip */ }
      const file = new File([bytes], `${model.id}.fst`, { type: 'application/octet-stream' });
      modelState = { ...modelState, [model.id]: { pct: 100, loading: false, error: null, cached: true, cachedSize: fmt(bytes.byteLength) } };
      dispatch('file', file);
    } catch (e) {
      modelState = { ...modelState, [model.id]: { pct: 0, loading: false, error: e.message, cached: false } };
    }
  }

  async function clearCache(model) {
    try { await dbDelete(model.id); } catch {}
    modelState = { ...modelState, [model.id]: { loading: false, error: null, pct: 0, cached: false } };
  }

  function validate(file) {
    if (!file) return 'No file selected.';
    if (!file.name.endsWith('.fst')) return `Expected a .fst file, got "${file.name}".`;
    if (file.size < 1024) return 'File seems too small to be a valid FST model.';
    return null;
  }

  function handleFile(file) {
    fileError = validate(file);
    if (!fileError) dispatch('file', file);
  }

  function onDrop(e) {
    e.preventDefault();
    isDragging = false;
    const file = e.dataTransfer?.files?.[0];
    handleFile(file);
  }

  function onPick(e) { handleFile(e.target.files?.[0]); }
  function onDragOver(e) { e.preventDefault(); isDragging = true; }
  function onDragLeave()  { isDragging = false; }
</script>

<div class="screen">
  <!-- Header -->
  <header>
    <div class="brand">
      <span class="brand-icon">ꓢ</span>
      <div>
        <h1>sosap</h1>
        <span class="sub">Grapheme-to-Phoneme · WebAssembly</span>
      </div>
    </div>
    <div class="chips">
      <span class="chip green">● WASM Ready</span>
      <span class="chip">Rust → Browser</span>
    </div>
  </header>

  <main>
    <!-- Steps indicator -->
    <div class="steps">
      <div class="step done">
        <span class="step-dot">✓</span>
        <span>Load WebAssembly</span>
      </div>
      <div class="step-line done"></div>
      <div class="step active">
        <span class="step-dot pulse">2</span>
        <span>Provide G2P Model</span>
      </div>
      <div class="step-line"></div>
      <div class="step">
        <span class="step-dot">3</span>
        <span>Phoneticize</span>
      </div>
    </div>

    <div class="content">
      <!-- Drop zone -->
      <section class="glass drop-zone {isDragging ? 'dragging' : ''}"
        role="region"
        aria-label="Drop zone for FST model file"
        on:dragover={onDragOver}
        on:dragleave={onDragLeave}
        on:drop={onDrop}
      >
        <div class="drop-icon">{isDragging ? '📂' : '📁'}</div>
        <h2>Drop your <code>.fst</code> model here</h2>
        <p>Drag & drop a Phonetisaurus G2P <code>.fst</code> file, or click to browse.</p>

        <label class="btn-pick" for="fst-file-input">
          Browse file…
          <input id="fst-file-input" type="file" accept=".fst" on:change={onPick} />
        </label>

        {#if fileError}
          <p class="file-error">⚠ {fileError}</p>
        {/if}
      </section>

      <!-- Info panel -->
      <aside class="info-panel">
        <div class="info-card glass">
          <h3>What is a .fst model?</h3>
          <p>
            sosap uses a <strong>Phonetisaurus OpenFST model</strong> — a weighted finite-state
            transducer trained on a pronunciation dictionary. The model file encodes how
            to map graphemes (letters) → phonemes (sounds).
          </p>
          <div class="example-block">
            <span class="ex-label">Example output</span>
            <code>model.phoneticize("hello")</code>
            <code class="out">→ ['h', 'ɛɛ', 'l', 'oo']</code>
          </div>
        </div>

        <!-- Download model cards -->
        <div class="info-card glass">
          <h3>Download a model</h3>
          <div class="model-cards">
            {#each MODELS as m}
              {@const ms = modelState[m.id]}
              <div class="model-card {ms?.loading ? 'is-loading' : ''} {ms?.cached ? 'is-cached' : ''}">
                <div class="model-card-top">
                  <span class="model-flag">{m.flag}</span>
                  <div class="model-meta">
                    <span class="model-name">
                      {m.label}
                      {#if ms?.cached}<span class="cached-badge">⚡ cached</span>{/if}
                    </span>
                    <span class="model-author">by {m.author} · {ms?.cached ? ms.cachedSize : m.size}</span>
                  </div>
                  <a href={m.hfUrl} target="_blank" rel="noreferrer" class="model-ext">↗</a>
                </div>
                <code class="model-example">{m.example}</code>

                {#if ms?.loading}
                  <div class="dl-progress">
                    <div class="dl-bar-wrap">
                      <div class="dl-bar" style="width: {ms.pct >= 0 ? ms.pct : 30}%"
                        class:indeterminate={ms.pct < 0}></div>
                    </div>
                    <span class="dl-pct">{ms.pct >= 0 ? ms.pct + '%' : '…'}</span>
                  </div>
                {:else if ms?.error}
                  <p class="dl-error">⚠ {ms.error}</p>
                  <button class="btn-dl" on:click={() => downloadAndLoad(m)}>Retry ↺</button>
                {:else if ms?.cached}
                  <div class="cached-actions">
                    <button class="btn-dl btn-cached" on:click={() => loadCached(m)}>
                      ⚡ Load (cached)
                    </button>
                    <button class="btn-clear-cache" title="Remove from cache" on:click={() => clearCache(m)}>
                      ✕
                    </button>
                  </div>
                {:else}
                  <button class="btn-dl" on:click={() => downloadAndLoad(m)}>
                    ⬇ Download &amp; Load
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <div class="info-card glass privacy-note">
          <span class="lock">🔒</span>
          <p>Models stream directly into WebAssembly memory — nothing is sent to a server or saved to disk.</p>
          </div>
      </aside>
    </div>
  </main>
</div>

<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;600&display=swap');

  .screen {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: radial-gradient(ellipse at 30% 0%, #0f1f3d 0%, #080c14 60%);
  }

  /* === Header === */
  header {
    background: rgba(8,12,20,0.9);
    backdrop-filter: blur(16px);
    border-bottom: 1px solid rgba(99,102,241,0.12);
    padding: 0 1.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 64px;
    gap: 1rem;
  }

  .brand { display: flex; align-items: center; gap: 0.75rem; }

  .brand-icon {
    font-size: 1.8rem;
    font-weight: 900;
    background: linear-gradient(135deg, #6366f1, #a78bfa);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  h1 { font-size: 1.3rem; font-weight: 700; color: #e0e7ff; letter-spacing: -0.02em; line-height: 1.1; }
  .sub { font-size: 0.7rem; color: #475569; letter-spacing: 0.04em; }

  .chips { display: flex; gap: 0.5rem; }
  .chip {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 999px;
    padding: 0.25em 0.75em;
    font-size: 0.72rem;
    color: #94a3b8;
  }
  .chip.green { color: #4ade80; border-color: rgba(74,222,128,0.3); background: rgba(74,222,128,0.07); }

  /* === Main === */
  main {
    flex: 1;
    max-width: 1000px;
    width: 100%;
    margin: 0 auto;
    padding: 2.5rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  /* === Steps === */
  .steps {
    display: flex;
    align-items: center;
    gap: 0;
  }

  .step {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.82rem;
    color: #334155;
    white-space: nowrap;
  }

  .step.done  { color: #4ade80; }
  .step.active { color: #a5b4fc; }

  .step-dot {
    width: 26px; height: 26px;
    border-radius: 50%;
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    display: flex; align-items: center; justify-content: center;
    font-size: 0.72rem;
    font-weight: 700;
    flex-shrink: 0;
  }

  .step.done  .step-dot { background: rgba(74,222,128,0.15); border-color: rgba(74,222,128,0.4); color: #4ade80; }
  .step.active .step-dot { background: rgba(99,102,241,0.15); border-color: rgba(99,102,241,0.5); color: #818cf8; }

  @keyframes pulse-ring {
    0%   { box-shadow: 0 0 0 0 rgba(99,102,241,0.5); }
    70%  { box-shadow: 0 0 0 6px rgba(99,102,241,0); }
    100% { box-shadow: 0 0 0 0 rgba(99,102,241,0); }
  }
  .pulse { animation: pulse-ring 1.6s ease-out infinite; }

  .step-line {
    flex: 1;
    height: 1px;
    background: rgba(255,255,255,0.07);
    margin: 0 0.5rem;
    min-width: 24px;
  }
  .step-line.done { background: rgba(74,222,128,0.3); }

  /* === Content layout === */
  .content {
    display: grid;
    grid-template-columns: 1fr 380px;
    gap: 1.5rem;
    align-items: start;
  }

  @media (max-width: 700px) {
    .content { grid-template-columns: 1fr; }
  }

  /* === Glass === */
  .glass {
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 18px;
    backdrop-filter: blur(12px);
  }

  /* === Drop zone === */
  .drop-zone {
    padding: 3rem 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    text-align: center;
    cursor: default;
    transition: border-color 0.2s, background 0.2s;
    min-height: 320px;
    justify-content: center;
  }

  .drop-zone.dragging {
    border-color: rgba(99,102,241,0.7);
    background: rgba(99,102,241,0.07);
    box-shadow: 0 0 0 4px rgba(99,102,241,0.12), inset 0 0 40px rgba(99,102,241,0.06);
  }

  .drop-icon { font-size: 3rem; transition: transform 0.2s; }
  .drop-zone.dragging .drop-icon { transform: scale(1.15); }

  h2 { font-size: 1.25rem; font-weight: 600; color: #c7d2fe; }
  h2 code, p code {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.9em;
    background: rgba(99,102,241,0.15);
    padding: 0.1em 0.4em;
    border-radius: 5px;
    color: #a5b4fc;
  }

  .drop-zone > p { color: #64748b; font-size: 0.88rem; max-width: 340px; line-height: 1.6; }

  .btn-pick {
    background: linear-gradient(135deg, #4f46e5, #7c3aed);
    color: white;
    padding: 0.75em 2em;
    border-radius: 12px;
    font-size: 0.92rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s, transform 0.15s;
    box-shadow: 0 4px 16px rgba(99,102,241,0.3);
    display: inline-block;
  }
  .btn-pick:hover { opacity: 0.9; transform: translateY(-1px); }
  .btn-pick input { display: none; }

  .file-error {
    color: #fca5a5;
    font-size: 0.82rem;
    background: rgba(239,68,68,0.1);
    border: 1px solid rgba(239,68,68,0.25);
    padding: 0.5em 1em;
    border-radius: 8px;
  }

  /* === Info panel === */
  .info-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .info-card {
    padding: 1.25rem 1.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  h3 { font-size: 0.9rem; font-weight: 600; color: #94a3b8; letter-spacing: 0.02em; }

  .info-card > p { font-size: 0.82rem; color: #64748b; line-height: 1.65; }
  .info-card strong { color: #94a3b8; font-weight: 600; }

  /* === Code example === */
  .example-block {
    background: rgba(0,0,0,0.3);
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 10px;
    padding: 0.9rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .ex-label { font-size: 0.68rem; color: #334155; text-transform: uppercase; letter-spacing: 0.07em; margin-bottom: 0.2rem; }

  .example-block code {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.8rem;
    color: #94a3b8;
  }
  .example-block code.out { color: #6ee7b7; }

  /* === Cached badge === */
  .cached-badge {
    display: inline-flex;
    align-items: center;
    font-size: 0.62rem;
    font-weight: 600;
    background: rgba(74,222,128,0.12);
    color: #4ade80;
    border: 1px solid rgba(74,222,128,0.3);
    padding: 0.1em 0.45em;
    border-radius: 999px;
    margin-left: 0.4rem;
    letter-spacing: 0.04em;
    vertical-align: middle;
  }

  .model-card.is-cached {
    border-color: rgba(74,222,128,0.2);
  }

  /* cached-actions: "Load" button + clear ✕ side by side */
  .cached-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .btn-cached {
    flex: 1;
    background: linear-gradient(135deg, rgba(16,185,129,0.18), rgba(5,150,105,0.18)) !important;
    border-color: rgba(16,185,129,0.45) !important;
    color: #6ee7b7 !important;
  }
  .btn-cached:hover {
    background: linear-gradient(135deg, rgba(16,185,129,0.3), rgba(5,150,105,0.3)) !important;
    box-shadow: 0 4px 14px rgba(16,185,129,0.2) !important;
  }

  .btn-clear-cache {
    flex-shrink: 0;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.1);
    color: #475569;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
    line-height: 1;
  }
  .btn-clear-cache:hover {
    background: rgba(239,68,68,0.1);
    color: #fca5a5;
    border-color: rgba(239,68,68,0.3);
  }

  /* === Privacy note === */
  .privacy-note {
    flex-direction: row;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem 1.2rem;
  }
  .lock { font-size: 1.1rem; flex-shrink: 0; margin-top: 1px; }
  .privacy-note p { font-size: 0.78rem; color: #475569; line-height: 1.55; }
</style>
