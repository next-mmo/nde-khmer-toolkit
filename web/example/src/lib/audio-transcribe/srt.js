export function offsetSrtTimes(srt, offset) {
  if (!offset) return srt;
  return srt.replace(/(\d{2}):(\d{2}):(\d{2}),(\d{3})/g, (_, h, m, s, ms) => {
    const t = +h * 3600 + +m * 60 + +s + +ms / 1000 + offset;
    const oh = Math.floor(t / 3600);
    const om = Math.floor((t % 3600) / 60);
    const os = Math.floor(t % 60);
    const oms = Math.round((t - Math.floor(t)) * 1000);
    return `${String(oh).padStart(2,'0')}:${String(om).padStart(2,'0')}:${String(os).padStart(2,'0')},${String(oms).padStart(3,'0')}`;
  });
}

export function segmentWords(text, locale) {
  if (!text || typeof Intl?.Segmenter === 'undefined') return text;
  const seg = new Intl.Segmenter(locale, { granularity: 'word' });
  return [...seg.segment(text)]
    .filter(s => s.isWordLike)
    .map(s => s.segment)
    .join(' ');
}

export function parseSrt(srtText) {
  return srtText.trim().split(/\n\n+/).flatMap(block => {
    const lines = block.trim().split('\n');
    const timeLine = lines.find(l => l.includes('-->'));
    if (!timeLine) return [];
    const m = timeLine.match(/(\d{2}):(\d{2}):(\d{2})[,.](\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2})[,.](\d{3})/);
    if (!m) return [];
    const toSec = (h, min, s, ms) => +h * 3600 + +min * 60 + +s + +ms / 1000;
    const text = lines.filter(l => !/^\d+$/.test(l.trim()) && !l.includes('-->')).join('\n').trim();
    return [{ start: toSec(m[1],m[2],m[3],m[4]), end: toSec(m[5],m[6],m[7],m[8]), text }];
  });
}

export function fmtTime(sec) {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  const ms = Math.round((sec % 1) * 1000);
  return `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(s).padStart(2,'0')},${String(ms).padStart(3,'0')}`;
}
