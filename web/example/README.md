# KFA Web Example

Static Svelte/Vite demo for Khmer text and audio tools. The app is designed to run in the browser, with most heavy logic compiled from Rust to WebAssembly.

## Goals

- Work from a static build; no required app server for core flows.
- Keep Khmer normalization, number handling, G2P, subtitle generation, and future alignment logic in WASM where practical.
- Treat native APIs, CLIs, and TTS services as optional companion tools, not web runtime requirements.
- Avoid backend-only dependencies in the frontend.

## Run

```sh
npm install
npm run dev
```

## Build

```sh
npm run build
npm run preview
```

The production output is `dist/` and can be deployed to static hosting. Ensure `.wasm` files are served as `application/wasm`; keep COOP/COEP headers if a WASM feature requires shared memory, workers, or streaming instantiation.

## WASM Layout

- `src/lib/wasm/` contains generated G2P glue.
- `src/lib/wasm-kfa/` contains generated KFA browser glue.
- `src/lib/wasm-transcribe/` contains generated speech transcription glue.
- `public/*.wasm` and `public/wasm/*` contain browser-loadable WASM/model assets.

Do not hand-edit generated WASM glue unless there is no source-level fix. Prefer updating the Rust crate and regenerating the browser package.
