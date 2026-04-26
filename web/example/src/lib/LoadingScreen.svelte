<script>
  export let label = 'Loading WebAssembly…';
</script>

<div class="loading-screen">
  <div class="loader-card">
    <div class="logo-wrap">
      <div class="ring ring-1"></div>
      <div class="ring ring-2"></div>
      <div class="ring ring-3"></div>
      <span class="logo-text">ꓢ</span>
    </div>
    <h1>sosap <span class="badge">WASM</span></h1>
    <p class="stage">{label}</p>
  </div>
</div>

<style>
  @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap');

  .loading-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: radial-gradient(ellipse at 50% 0%, #0f1f3d 0%, #080c14 65%);
  }

  .loader-card {
    text-align: center;
    padding: 3rem 2.5rem;
    max-width: 380px;
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .logo-wrap {
    position: relative;
    width: 90px; height: 90px;
    display: flex; align-items: center; justify-content: center;
    margin-bottom: 0.5rem;
  }

  .ring {
    position: absolute;
    border-radius: 50%;
    border: 2px solid transparent;
    animation: spin linear infinite;
  }

  .ring-1 { width: 90px; height: 90px; border-top-color: #6366f1; border-right-color: rgba(99,102,241,.2); animation-duration: 2.4s; }
  .ring-2 { width: 64px; height: 64px; border-top-color: #818cf8; border-left-color: rgba(129,140,248,.2); animation-duration: 1.8s; animation-direction: reverse; }
  .ring-3 { width: 40px; height: 40px; border-top-color: #a5b4fc; animation-duration: 1.2s; }

  @keyframes spin { to { transform: rotate(360deg); } }

  .logo-text { font-size: 1.3rem; color: #c7d2fe; font-weight: 700; z-index: 1; }

  h1 {
    font-size: 1.8rem;
    font-weight: 700;
    color: #e0e7ff;
    letter-spacing: -0.02em;
    display: flex; align-items: center; gap: 0.5rem;
  }

  .badge {
    font-size: 0.65rem;
    font-weight: 600;
    background: linear-gradient(135deg, #6366f1, #818cf8);
    color: white;
    padding: 0.2em 0.5em;
    border-radius: 6px;
    letter-spacing: 0.06em;
  }

  .stage {
    color: #64748b;
    font-size: 0.85rem;
  }
</style>
