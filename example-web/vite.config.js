import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  server: {
    headers: {
      // Required for WebAssembly streaming instantiation
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
    proxy: {
      '/google-speech-api': {
        target: 'https://www.google.com',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/google-speech-api/, '/speech-api'),
      },
    },
  },
})
