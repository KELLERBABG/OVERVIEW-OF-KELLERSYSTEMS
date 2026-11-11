/**
 * Device fingerprinting for Sybil resistance.
 *
 * Combines multiple browser signals to create a hardware-backed
 * identity that is expensive for headless botnets to forge.
 */

export interface DeviceFingerprint {
  canvasHash: string;
  webglRenderer: string;
  webglVendor: string;
  hardwareConcurrency: number;
  screenResolution: string;
  colorDepth: number;
  pixelRatio: number;
  platform: string;
  audioFingerprint: string;
  fontsHash: string;
  hasWebAuthn: boolean;
}

export async function collectDeviceFingerprint(): Promise<DeviceFingerprint> {
  return {
    canvasHash: await getCanvasHash(),
    webglRenderer: getWebGLRenderer(),
    webglVendor: getWebGLVendor(),
    hardwareConcurrency: navigator.hardwareConcurrency ?? 0,
    screenResolution: `${screen.width}x${screen.height}`,
    colorDepth: screen.colorDepth,
    pixelRatio: window.devicePixelRatio ?? 1,
    platform: navigator.platform ?? 'unknown',
    audioFingerprint: await getAudioFingerprint(),
    fontsHash: await getFontsHash(),
    hasWebAuthn: typeof PublicKeyCredential !== 'undefined',
  };
}

export function fingerprintToScore(fp: DeviceFingerprint): number {
  let score = 0;
  if (fp.canvasHash && fp.canvasHash.length > 0) score += 0.2;
  if (fp.webglRenderer && fp.webglRenderer !== '') score += 0.2;
  if (fp.audioFingerprint && fp.audioFingerprint.length > 0) score += 0.15;
  if (fp.screenResolution !== '0x0') score += 0.1;
  if (fp.hardwareConcurrency > 1) score += 0.1;
  if (fp.fontsHash && fp.fontsHash.length > 0) score += 0.1;
  if (fp.hasWebAuthn) score += 0.15;
  return Math.min(1, score);
}

async function getCanvasHash(): Promise<string> {
  try {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');
    if (!ctx) return '';

    ctx.textBaseline = 'top';
    ctx.font = '14px Arial';
    ctx.fillStyle = '#f60';
    ctx.fillRect(125, 1, 62, 20);
    ctx.fillStyle = '#069';
    ctx.fillText('Fungal', 2, 15);
    ctx.fillStyle = 'rgba(102, 204, 0, 0.7)';
    ctx.fillText('Swarm', 4, 45);

    const dataUrl = canvas.toDataURL();
    return simpleHash(dataUrl);
  } catch {
    return '';
  }
}

function getWebGLRenderer(): string {
  try {
    const canvas = document.createElement('canvas');
    const gl = (canvas.getContext('webgl') || canvas.getContext('experimental-webgl')) as WebGLRenderingContext | null;
    if (!gl) return '';
    const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
    if (!debugInfo) return '';
    return (gl as any).getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) ?? '';
  } catch {
    return '';
  }
}

function getWebGLVendor(): string {
  try {
    const canvas = document.createElement('canvas');
    const gl = (canvas.getContext('webgl') || canvas.getContext('experimental-webgl')) as WebGLRenderingContext | null;
    if (!gl) return '';
    const debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
    if (!debugInfo) return '';
    return (gl as any).getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) ?? '';
  } catch {
    return '';
  }
}

async function getAudioFingerprint(): Promise<string> {
  try {
    const ctx = new AudioContext();
    const oscillator = ctx.createOscillator();
    const analyser = ctx.createAnalyser();
    const gain = ctx.createGain();
    oscillator.connect(analyser);
    if (gain) { analyser.connect(gain); }
    oscillator.frequency.value = 440;
    oscillator.type = 'triangle';
    const data = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(data);
    ctx.close();
    return simpleHash(Array.from(data).join(','));
  } catch {
    return '';
  }
}

async function getFontsHash(): Promise<string> {
  try {
    const baseFonts = ['monospace', 'sans-serif', 'serif'];
    const testFonts = [
      'Arial', 'Helvetica', 'Times New Roman', 'Courier New',
      'Verdana', 'Georgia', 'Palatino', 'Garamond',
      'Comic Sans MS', 'Trebuchet MS', 'Arial Black', 'Impact',
    ];
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx) return '';
    const results: number[] = [];

    for (const base of baseFonts) {
      ctx.font = `72px ${base}`;
      const baseWidth = ctx.measureText('mmmmmmmmmmlli').width;
      for (const font of testFonts) {
        ctx.font = `72px ${font}, ${base}`;
        const width = ctx.measureText('mmmmmmmmmmlli').width;
        results.push(width !== baseWidth ? 1 : 0);
      }
    }

    return simpleHash(results.join(''));
  } catch {
    return '';
  }
}

function simpleHash(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash |= 0;
  }
  return Math.abs(hash).toString(16);
}