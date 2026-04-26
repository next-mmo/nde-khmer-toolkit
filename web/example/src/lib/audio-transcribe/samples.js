export const DEMO_AUDIO_SAMPLES = [
  {
    id: 'sample-1',
    label: 'Sample 1',
    url: 'https://cdn.jsdelivr.net/gh/next-mmo/nde-khmer-toolkit@main/data/samples_khm_1161_1980987674.wav',
    fallbackUrl: '/demo-audio/samples_khm_1161_1980987674.wav',
    filename: 'samples_khm_1161_1980987674.wav',
    type: 'audio/wav',
  },
  {
    id: 'sample-2',
    label: 'Sample 2',
    url: 'https://cdn.jsdelivr.net/gh/next-mmo/nde-khmer-toolkit@main/data/khmer-audio-16k.wav',
    fallbackUrl: '/demo-audio/khmer-audio-16k.wav',
    filename: 'khmer-audio-16k-demo.wav',
    type: 'audio/wav',
  },
];

export async function loadDemoAudioFile(sample) {
  let response = null;
  let lastError = '';

  for (const url of [sample.fallbackUrl, sample.url]) {
    try {
      response = await fetch(url, { cache: 'force-cache' });
      if (response.ok) break;
      lastError = `${response.status}`;
    } catch (e) {
      lastError = e?.message || String(e);
    }
    response = null;
  }

  if (!response) {
    throw new Error(`Could not load ${sample.label}${lastError ? ` (${lastError})` : ''}`);
  }

  const blob = await response.blob();
  return new File([blob], sample.filename, { type: blob.type || sample.type });
}
