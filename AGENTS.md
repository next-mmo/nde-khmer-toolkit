# AGENTS.md

## Project Shape

- Rust workspace for Khmer text/audio tooling plus a Svelte/Vite browser demo.
- Native/API crates live in `api/` and `crates/*`; browser bindings live in `crates/kfa-wasm` and `crates/kfa-web`.
- The web demo lives in `web/example` and must stay deployable as static files.

## Browser-First Rules

- Core web features should run fully in the browser, without requiring a local or hosted app server.
- Prefer Rust compiled to WASM for Khmer normalization, number verbalization, G2P, subtitle timing, audio preparation, and future alignment logic.
- Do not add required `/api`, `/v1`, Node, filesystem, native binary, or database dependencies to `web/example`.
- Keep server crates optional. `api/`, `edge-tts-rs`, and native CLIs may exist, but the browser demo must not depend on them for its primary flows.
- If a capability cannot run locally in WASM yet, keep it isolated, label it as a temporary external/provider dependency, and preserve a browser-only fallback.
- Use `cfg(target_arch = "wasm32")` and target-specific dependencies for browser code instead of runtime platform checks when possible.
- WASM assets must be served with `application/wasm`; keep COOP/COEP headers when threads, shared memory, or streaming instantiation need them.

## Code Style

- Follow existing Rust/Svelte patterns; avoid broad refactors.
- Keep generated WASM glue and binary artifacts in the current web asset layout unless changing the build pipeline intentionally.
- Do not hand-edit generated `wasm-pack` output except as a last resort; update the source crate and regenerate.
- Keep UI changes static-host friendly: no SSR assumptions, no server-only environment variables, and no secrets in frontend code.

## Useful Commands

- Rust check: `cargo check --workspace`
- Rust tests: `cargo test --workspace`
- Web dev: `cd web/example && npm run dev`
- Web build: `cd web/example && npm run build`
