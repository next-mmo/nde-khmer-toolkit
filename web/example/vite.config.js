import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import edgeTtsHandler from './api/edge-tts.js'

// https://vite.dev/config/
export default defineConfig({
  plugins: [edgeTtsApiPlugin(), svelte()],
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

function edgeTtsApiPlugin() {
  return {
    name: 'edge-tts-api-dev',
    configureServer(server) {
      server.middlewares.use('/api/edge-tts', async (req, res) => {
        try {
          req.body = await readJsonBody(req);
          await edgeTtsHandler(req, createVercelLikeResponse(res));
        } catch (error) {
          res.statusCode = 500;
          res.setHeader('Content-Type', 'application/json');
          res.end(JSON.stringify({ error: error?.message || String(error) }));
        }
      });
    },
  };
}

function readJsonBody(req) {
  if (req.method === 'GET' || req.method === 'OPTIONS') return Promise.resolve({});
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', chunk => chunks.push(chunk));
    req.on('error', reject);
    req.on('end', () => {
      const text = Buffer.concat(chunks).toString('utf8');
      if (!text) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(text));
      } catch {
        reject(new Error('Invalid JSON request body'));
      }
    });
  });
}

function createVercelLikeResponse(res) {
  return {
    status(code) {
      res.statusCode = code;
      return this;
    },
    setHeader(name, value) {
      res.setHeader(name, value);
    },
    json(value) {
      if (!res.getHeader('Content-Type')) {
        res.setHeader('Content-Type', 'application/json');
      }
      res.end(JSON.stringify(value));
    },
    end(value) {
      res.end(value);
    },
  };
}
