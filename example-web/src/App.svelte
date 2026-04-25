<script>
  import { onMount } from 'svelte';
  import LoadingScreen from './lib/LoadingScreen.svelte';
  import OnboardingScreen from './lib/OnboardingScreen.svelte';
  import Phonetizer from './lib/Phonetizer.svelte';
  import AudioTranscribe from './lib/AudioTranscribe.svelte';

  // ── App-level tab ─────────────────────────────────────────────────────────
  // 'g2p' = Grapheme-to-Phoneme  |  'stt' = Speech-to-Text
  let appTab = 'g2p';

  // ── G2P (sosap) state ─────────────────────────────────────────────────────
  // phase: 'loading-wasm' | 'onboarding' | 'loading-model' | 'ready' | 'error'
  let phase = 'loading-wasm';
  let loadError = null;
  let wasmMod = null;
  let model = null;

  onMount(async () => {
    try {
      wasmMod = await import('./lib/wasm/sosap.js');
      await wasmMod.default('/wasm/sosap_bg.wasm');
      phase = 'onboarding';
    } catch (e) {
      console.error(e);
      loadError = e.message || String(e);
      phase = 'error';
    }
  });

  async function handleModelFile(file) {
    phase = 'loading-model';
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const { Model } = wasmMod;
      model = new Model(bytes, '');
      phase = 'ready';
    } catch (e) {
      console.error(e);
      loadError = `Failed to load model: ${e.message || e}`;
      phase = 'error';
    }
  }

  function resetModel() {
    model = null;
    phase = 'onboarding';
  }
</script>

<!-- ── Top-level tab bar ──────────────────────────────────────────────────── -->
<nav class="app-nav">
  <button
    class="nav-tab {appTab === 'g2p' ? 'active' : ''}"
    on:click={() => appTab = 'g2p'}
  >
    ꓢ&nbsp; G2P Phonetizer
  </button>
  <button
    class="nav-tab {appTab === 'stt' ? 'active' : ''}"
    on:click={() => appTab = 'stt'}
  >
    🎙️&nbsp; Audio Transcribe
  </button>
</nav>

<!-- ── G2P panel ─────────────────────────────────────────────────────────── -->
{#if appTab === 'g2p'}
  {#if phase === 'loading-wasm'}
    <LoadingScreen />
  {:else if phase === 'onboarding'}
    <OnboardingScreen on:file={e => handleModelFile(e.detail)} />
  {:else if phase === 'loading-model'}
    <LoadingScreen label="Loading model…" />
  {:else if phase === 'ready'}
    <Phonetizer {model} on:reset={resetModel} />
  {:else if phase === 'error'}
    <div class="error-screen">
      <div class="error-card">
        <span class="error-icon">⚠️</span>
        <h2>Something went wrong</h2>
        <p>{loadError}</p>
        <button on:click={() => { loadError = null; phase = 'onboarding'; }}>Try again</button>
      </div>
    </div>
  {/if}
{/if}

<!-- ── STT panel ──────────────────────────────────────────────────────────── -->
{#if appTab === 'stt'}
  <AudioTranscribe />
{/if}

<style>
  :global(*) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: #080c14;
    color: #e2e8f4;
    font-family: 'Inter', system-ui, sans-serif;
    min-height: 100vh;
  }

  /* ── Nav tabs ── */
  .app-nav {
    display: flex;
    gap: 2px;
    background: rgba(0,0,0,0.5);
    border-bottom: 1px solid rgba(255,255,255,0.07);
    padding: 0 1.5rem;
    position: sticky;
    top: 0;
    z-index: 200;
    backdrop-filter: blur(16px);
  }

  .nav-tab {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #475569;
    padding: 0.85rem 1.25rem;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s, border-color 0.15s;
    white-space: nowrap;
  }
  .nav-tab:hover { color: #94a3b8; }
  .nav-tab.active {
    color: #e2e8f4;
    border-bottom-color: #6366f1;
  }

  /* ── Error screen (G2P) ── */
  .error-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 1.5rem;
  }

  .error-card {
    background: rgba(239,68,68,0.08);
    border: 1px solid rgba(239,68,68,0.3);
    border-radius: 18px;
    padding: 2.5rem;
    text-align: center;
    max-width: 480px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    align-items: center;
  }

  .error-icon { font-size: 2.5rem; }
  h2 { font-size: 1.3rem; color: #fca5a5; }
  p  { color: #fca5a5; opacity: 0.75; font-size: 0.9rem; word-break: break-word; }

  button {
    margin-top: 0.5rem;
    background: rgba(239,68,68,0.15);
    border: 1px solid rgba(239,68,68,0.4);
    color: #fca5a5;
    padding: 0.55em 1.4em;
    border-radius: 10px;
    cursor: pointer;
    font-family: inherit;
    font-size: 0.88rem;
    transition: background 0.2s;
  }
  button:hover { background: rgba(239,68,68,0.25); }
</style>
