export const TARGET_SAMPLE_RATE = 16000;
export const AUDIO_EXTS = /\.(flac|wav|mp3|ogg|m4a|opus|webm|aac)$/i;

export function formatFileSize(file) {
  if (!file) return '';
  return file.size > 1e6
    ? `${(file.size / 1e6).toFixed(1)} MB`
    : `${(file.size / 1024).toFixed(0)} KB`;
}

export function inferAudioContentType(file) {
  const name = file.name.toLowerCase();
  if (name.endsWith('.flac')) return 'audio/x-flac; rate=16000';
  if (name.endsWith('.wav')) return 'audio/l16; rate=16000';
  if (name.endsWith('.mp3')) return 'audio/mpeg';
  if (name.endsWith('.ogg') || name.endsWith('.opus')) return 'audio/ogg';
  if (name.endsWith('.webm')) return 'audio/webm';
  if (name.endsWith('.m4a')) return 'audio/mp4';
  if (name.endsWith('.aac')) return 'audio/aac';
  return file.type || 'application/octet-stream';
}

async function decodeAudioData(arrayBuffer) {
  const Ctx = window.AudioContext || /** @type {any} */ (window).webkitAudioContext;
  if (!Ctx) throw new Error('This browser does not support Web Audio decoding.');
  const ctx = new Ctx();
  try {
    return await ctx.decodeAudioData(arrayBuffer);
  } finally {
    ctx.close?.();
  }
}

function sampleAt(channel, pos) {
  const i = Math.floor(pos);
  if (i < 0) return channel[0] || 0;
  if (i >= channel.length - 1) return channel[channel.length - 1] || 0;
  const frac = pos - i;
  return channel[i] * (1 - frac) + channel[i + 1] * frac;
}

function highPass(samples, sampleRate, cutoffHz) {
  const out = new Float32Array(samples.length);
  const dt = 1 / sampleRate;
  const rc = 1 / (2 * Math.PI * cutoffHz);
  const alpha = rc / (rc + dt);
  let prevY = 0;
  let prevX = samples[0] || 0;
  for (let i = 0; i < samples.length; i++) {
    const x = samples[i];
    const y = alpha * (prevY + x - prevX);
    out[i] = y;
    prevY = y;
    prevX = x;
  }
  return out;
}

function lowPass(samples, sampleRate, cutoffHz) {
  const out = new Float32Array(samples.length);
  const dt = 1 / sampleRate;
  const rc = 1 / (2 * Math.PI * cutoffHz);
  const alpha = dt / (rc + dt);
  let y = samples[0] || 0;
  for (let i = 0; i < samples.length; i++) {
    y += alpha * (samples[i] - y);
    out[i] = y;
  }
  return out;
}

function applyNoiseGate(samples, sampleRate) {
  const frame = Math.max(1, Math.floor(sampleRate * 0.02));
  const rms = [];
  let peak = 0;
  for (let i = 0; i < samples.length; i += frame) {
    let sum = 0;
    const end = Math.min(samples.length, i + frame);
    for (let j = i; j < end; j++) {
      const s = samples[j];
      sum += s * s;
      const a = Math.abs(s);
      if (a > peak) peak = a;
    }
    rms.push(Math.sqrt(sum / Math.max(1, end - i)));
  }
  if (!peak) return samples;
  const sorted = [...rms].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length * 0.5)] || 0;
  const threshold = Math.max(peak * 0.012, median * 0.8, 0.002);
  const out = new Float32Array(samples.length);
  for (let frameIndex = 0; frameIndex < rms.length; frameIndex++) {
    const gain = rms[frameIndex] < threshold ? 0.25 : 1;
    const start = frameIndex * frame;
    const end = Math.min(samples.length, start + frame);
    for (let i = start; i < end; i++) out[i] = samples[i] * gain;
  }
  return out;
}

function normalizePeak(samples) {
  let peak = 0;
  for (let i = 0; i < samples.length; i++) peak = Math.max(peak, Math.abs(samples[i]));
  if (!peak) return samples;
  const gain = Math.min(1.8, 0.92 / peak);
  if (gain <= 1) return samples;
  const out = new Float32Array(samples.length);
  for (let i = 0; i < samples.length; i++) out[i] = samples[i] * gain;
  return out;
}

function audioBufferToMono16k(audioBuffer, keepVocal) {
  const outLength = Math.max(1, Math.ceil(audioBuffer.duration * TARGET_SAMPLE_RATE));
  const out = new Float32Array(outLength);
  const sourceRate = audioBuffer.sampleRate;
  const channels = Array.from({ length: audioBuffer.numberOfChannels }, (_, i) => audioBuffer.getChannelData(i));

  for (let i = 0; i < outLength; i++) {
    const pos = i * sourceRate / TARGET_SAMPLE_RATE;
    if (keepVocal && channels.length >= 2) {
      out[i] = (sampleAt(channels[0], pos) + sampleAt(channels[1], pos)) * 0.5;
    } else {
      let sum = 0;
      for (const channel of channels) sum += sampleAt(channel, pos);
      out[i] = sum / channels.length;
    }
  }

  if (!keepVocal) return out;
  return normalizePeak(applyNoiseGate(lowPass(highPass(out, TARGET_SAMPLE_RATE, 90), TARGET_SAMPLE_RATE, 7400), TARGET_SAMPLE_RATE));
}

function encodeLinear16(samples) {
  const bytes = new Uint8Array(samples.length * 2);
  const view = new DataView(bytes.buffer);
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    view.setInt16(i * 2, s < 0 ? s * 0x8000 : s * 0x7fff, true);
  }
  return bytes;
}

function detectSpeechBoundsFromBuffer(audioBuffer) {
  const sr = audioBuffer.sampleRate;
  const data = audioBuffer.getChannelData(0);
  const total = data.length / sr;

  let peak = 0;
  for (let i = 0; i < data.length; i++) {
    const a = Math.abs(data[i]);
    if (a > peak) peak = a;
  }
  if (peak === 0) return { start: 0, end: total, total };

  const threshold = peak * 0.04;
  const frame = Math.max(1, Math.floor(sr * 0.02));
  let firstS = -1;
  let lastS = -1;
  for (let i = 0; i < data.length; i += frame) {
    let max = 0;
    const limit = Math.min(i + frame, data.length);
    for (let j = i; j < limit; j++) {
      const a = Math.abs(data[j]);
      if (a > max) max = a;
    }
    if (max > threshold) {
      if (firstS === -1) firstS = i;
      lastS = limit;
    }
  }
  if (firstS === -1) return { start: 0, end: total, total };
  return { start: firstS / sr, end: lastS / sr, total };
}

function getAudioDuration(file) {
  return new Promise((resolve) => {
    const url = URL.createObjectURL(file);
    const audio = new Audio(url);
    audio.onloadedmetadata = () => {
      URL.revokeObjectURL(url);
      resolve(audio.duration || 0);
    };
    audio.onerror = () => {
      URL.revokeObjectURL(url);
      resolve(0);
    };
  });
}

export async function prepareAudioForTranscription(file, options) {
  const { convertTo16k, reduceBackground, onStep = () => {} } = options;
  const fileBuffer = await file.arrayBuffer();
  let decoded = null;
  let bounds = null;

  try {
    decoded = await decodeAudioData(fileBuffer.slice(0));
    bounds = detectSpeechBoundsFromBuffer(decoded);
  } catch {
    const total = await getAudioDuration(file);
    bounds = { start: 0, end: total, total };
  }

  if (!convertTo16k && !reduceBackground) {
    return {
      bytes: new Uint8Array(fileBuffer),
      contentType: inferAudioContentType(file),
      bounds,
    };
  }

  if (!decoded) {
    throw new Error('Could not decode audio for browser-side 16 kHz conversion.');
  }

  onStep(reduceBackground ? 'Preparing 16 kHz vocal-focused audio...' : 'Converting to 16 kHz mono...');
  const samples = audioBufferToMono16k(decoded, reduceBackground);
  return {
    bytes: encodeLinear16(samples),
    contentType: 'audio/l16; rate=16000',
    bounds,
  };
}
