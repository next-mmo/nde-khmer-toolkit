import crypto from 'node:crypto';
import tls from 'node:tls';

const EDGE_TTS_BASE_URL = 'speech.platform.bing.com';
const EDGE_TTS_PATH = '/consumer/speech/synthesize/readaloud/edge/v1';
const EDGE_TTS_CLIENT_TOKEN = '6A5AA1D4EAFF4E9FB37E23D68491D6F4';
const EDGE_TTS_CHROMIUM_VERSION = '143.0.3650.75';
const EDGE_TTS_GEC_VERSION = `1-${EDGE_TTS_CHROMIUM_VERSION}`;
const EDGE_TTS_EXTENSION_ORIGIN = 'chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold';
const EDGE_TTS_TIMEOUT_MS = 20000;
const EDGE_TTS_IDLE_AFTER_AUDIO_MS = 1200;
const EDGE_VOICES = new Map([
  ['km-KH-PisethNeural', { lang: 'km-KH' }],
  ['km-KH-SreymomNeural', { lang: 'km-KH' }],
]);

export default async function handler(req, res) {
  setCorsHeaders(res);

  if (req.method === 'OPTIONS') {
    res.status(204).end();
    return;
  }

  if (req.method !== 'POST') {
    res.status(405).json({ error: 'Method not allowed' });
    return;
  }

  try {
    const body = typeof req.body === 'string' ? JSON.parse(req.body) : req.body || {};
    const text = String(body.text || '').trim();
    const voice = String(body.voice || 'km-KH-PisethNeural');
    const voiceMeta = EDGE_VOICES.get(voice);

    if (!text) {
      res.status(400).json({ error: 'Text is required' });
      return;
    }
    if (text.length > 5000) {
      res.status(413).json({ error: 'Text is too long. Maximum is 5000 characters.' });
      return;
    }
    if (!voiceMeta) {
      res.status(400).json({ error: 'Unsupported voice' });
      return;
    }

    const audio = await synthesizeEdgeTts({
      text,
      voice,
      lang: voiceMeta.lang,
      rate: normalizePercent(body.rate, '+0%'),
      volume: '+0%',
      pitch: normalizePitch(body.pitch, '+0Hz'),
    });

    res.status(200);
    res.setHeader('Content-Type', 'audio/mpeg');
    res.setHeader('Cache-Control', 'no-store');
    res.end(audio);
  } catch (error) {
    res.status(502).json({
      error: error?.message || String(error),
    });
  }
}

function setCorsHeaders(res) {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
}

async function synthesizeEdgeTts(options) {
  const audioChunks = [];
  const connectionId = createConnectionId();
  const urlPath = `${EDGE_TTS_PATH}?TrustedClientToken=${EDGE_TTS_CLIENT_TOKEN}&Sec-MS-GEC=${generateSecMsGec()}&Sec-MS-GEC-Version=${EDGE_TTS_GEC_VERSION}&ConnectionId=${connectionId}`;

  return new Promise((resolve, reject) => {
    const socket = tls.connect({
      host: EDGE_TTS_BASE_URL,
      port: 443,
      servername: EDGE_TTS_BASE_URL,
    });

    let settled = false;
    let handshakeDone = false;
    let buffer = Buffer.alloc(0);
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
      socket.end();
      if (!audioChunks.length) {
        reject(new Error('Edge TTS API did not receive audio.'));
        return;
      }
      resolve(Buffer.concat(audioChunks));
    };

    const fail = (error) => {
      if (settled) return;
      settled = true;
      clearTimers();
      socket.destroy();
      reject(error);
    };

    const armTimeout = () => {
      timeoutId = setTimeout(() => {
        if (audioChunks.length) finish();
        else fail(new Error('Edge TTS API timed out before audio arrived.'));
      }, EDGE_TTS_TIMEOUT_MS);
    };

    const armIdleFinish = () => {
      clearTimeout(idleId);
      idleId = setTimeout(finish, EDGE_TTS_IDLE_AFTER_AUDIO_MS);
    };

    socket.setNoDelay(true);
    socket.once('secureConnect', () => {
      socket.write(createWebSocketHandshake(urlPath));
      armTimeout();
    });
    socket.on('error', fail);
    socket.on('data', (chunk) => {
      try {
        buffer = Buffer.concat([buffer, chunk]);

        if (!handshakeDone) {
          const headerEnd = buffer.indexOf('\r\n\r\n');
          if (headerEnd === -1) return;
          const headerText = buffer.subarray(0, headerEnd).toString('utf8');
          const statusLine = headerText.split('\r\n')[0] || '';
          if (!/^HTTP\/1\.1 101\b/.test(statusLine)) {
            fail(new Error(`Edge TTS WebSocket rejected: ${statusLine}`));
            return;
          }
          handshakeDone = true;
          buffer = buffer.subarray(headerEnd + 4);
          socket.write(encodeClientFrame(createSpeechConfigMessage()));
          socket.write(encodeClientFrame(createSsmlMessage(options)));
        }

        let frame;
        while ((frame = readWebSocketFrame(buffer))) {
          buffer = buffer.subarray(frame.nextOffset);
          if (frame.opcode === 0x8) {
            finish();
            return;
          }
          if (frame.opcode === 0x9) {
            socket.write(encodeClientFrame(frame.payload, 0xA));
            continue;
          }
          if (frame.opcode === 0x1) {
            const message = frame.payload.toString('utf8');
            const { headers } = parseEdgeTextMessage(message);
            if (headers.Path === 'turn.end') {
              finish();
              return;
            }
          }
          if (frame.opcode === 0x2) {
            const { headers, audioData } = parseEdgeBinaryMessage(frame.payload);
            if (headers.Path === 'audio' && headers['Content-Type'] === 'audio/mpeg' && audioData.length) {
              audioChunks.push(audioData);
              armIdleFinish();
            }
          }
        }
      } catch (error) {
        fail(error);
      }
    });
    socket.on('end', () => {
      if (!settled) finish();
    });
  });
}

function createWebSocketHandshake(path) {
  const key = crypto.randomBytes(16).toString('base64');
  return [
    `GET ${path} HTTP/1.1`,
    `Host: ${EDGE_TTS_BASE_URL}`,
    'Upgrade: websocket',
    'Connection: Upgrade',
    `Sec-WebSocket-Key: ${key}`,
    'Sec-WebSocket-Version: 13',
    `Origin: ${EDGE_TTS_EXTENSION_ORIGIN}`,
    'Pragma: no-cache',
    'Cache-Control: no-cache',
    `User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0`,
    'Accept-Encoding: gzip, deflate, br, zstd',
    'Accept-Language: en-US,en;q=0.9',
    `Cookie: muid=${crypto.randomBytes(16).toString('hex').toUpperCase()};`,
    '\r\n',
  ].join('\r\n');
}

function encodeClientFrame(data, opcode = 0x1) {
  const payload = Buffer.isBuffer(data) ? data : Buffer.from(data);
  let header;
  if (payload.length < 126) {
    header = Buffer.alloc(6);
    header[0] = 0x80 | opcode;
    header[1] = 0x80 | payload.length;
    crypto.randomBytes(4).copy(header, 2);
    maskPayload(payload, header.subarray(2, 6));
    return Buffer.concat([header, payload]);
  }
  if (payload.length <= 0xffff) {
    header = Buffer.alloc(8);
    header[0] = 0x80 | opcode;
    header[1] = 0x80 | 126;
    header.writeUInt16BE(payload.length, 2);
    crypto.randomBytes(4).copy(header, 4);
    maskPayload(payload, header.subarray(4, 8));
    return Buffer.concat([header, payload]);
  }
  header = Buffer.alloc(14);
  header[0] = 0x80 | opcode;
  header[1] = 0x80 | 127;
  header.writeBigUInt64BE(BigInt(payload.length), 2);
  crypto.randomBytes(4).copy(header, 10);
  maskPayload(payload, header.subarray(10, 14));
  return Buffer.concat([header, payload]);
}

function maskPayload(payload, mask) {
  for (let i = 0; i < payload.length; i += 1) {
    payload[i] ^= mask[i % 4];
  }
}

function readWebSocketFrame(buffer) {
  if (buffer.length < 2) return null;
  const first = buffer[0];
  const second = buffer[1];
  const opcode = first & 0x0f;
  const masked = (second & 0x80) !== 0;
  let length = second & 0x7f;
  let offset = 2;

  if (length === 126) {
    if (buffer.length < offset + 2) return null;
    length = buffer.readUInt16BE(offset);
    offset += 2;
  } else if (length === 127) {
    if (buffer.length < offset + 8) return null;
    length = Number(buffer.readBigUInt64BE(offset));
    offset += 8;
  }

  const maskOffset = offset;
  if (masked) offset += 4;
  if (buffer.length < offset + length) return null;

  const payload = Buffer.from(buffer.subarray(offset, offset + length));
  if (masked) {
    const mask = buffer.subarray(maskOffset, maskOffset + 4);
    maskPayload(payload, mask);
  }
  return {
    opcode,
    payload,
    nextOffset: offset + length,
  };
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
  const fullVoiceName = toFullEdgeVoiceName(voice);
  const ssml = `<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='${lang}'><voice name='${fullVoiceName}'><prosody pitch='${pitch}' rate='${rate}' volume='${volume}'>${escapeXml(text)}</prosody></voice></speak>`;
  return `X-RequestId:${requestId}\r
Content-Type:application/ssml+xml\r
X-Timestamp:${formatEdgeDate()}Z\r
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
  if (buffer.length < 2) return { headers: {}, audioData: Buffer.alloc(0) };
  const headerLength = (buffer[0] << 8) | buffer[1];
  const headerText = buffer.subarray(2, 2 + headerLength).toString('utf8');
  const headers = Object.fromEntries(
    headerText
      .split('\r\n')
      .map(line => line.split(/:(.*)/s).slice(0, 2))
      .filter(([key, value]) => key && value != null)
      .map(([key, value]) => [key, value.trim()])
  );
  return {
    headers,
    audioData: buffer.subarray(2 + headerLength),
  };
}

function generateSecMsGec() {
  const winEpoch = 11644473600;
  let ticks = Date.now() / 1000 + winEpoch;
  ticks -= ticks % 300;
  ticks *= 10000000;
  return crypto
    .createHash('sha256')
    .update(`${ticks.toFixed(0)}${EDGE_TTS_CLIENT_TOKEN}`)
    .digest('hex')
    .toUpperCase();
}

function createConnectionId() {
  const bytes = crypto.randomBytes(16);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  return bytes.toString('hex');
}

function formatEdgeDate() {
  return new Date().toUTCString().replace('GMT', 'GMT+0000 (Coordinated Universal Time)');
}

function escapeXml(value) {
  return String(value || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function normalizePercent(value, fallback) {
  const raw = typeof value === 'number' ? `${value >= 0 ? '+' : ''}${value}%` : String(value || fallback);
  return /^[+-]?\d+%$/.test(raw) ? raw : fallback;
}

function normalizePitch(value, fallback) {
  const raw = typeof value === 'number' ? `${value >= 0 ? '+' : ''}${value}Hz` : String(value || fallback);
  return /^[+-]?\d+Hz$/.test(raw) ? raw : fallback;
}
