import { chromium } from 'playwright-core';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const browser = await chromium.launch({ executablePath: CHROME, headless: true });
const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });

const checks = [
  { url: '#/3d', expect: ['3D Visualization Console', 'Element Inspector'] },
  { url: '#/mpr', expect: ['Volumetric Slicer', 'Voxel Inspector'] },
  { url: '#/lab', expect: ['Physics & Inversion Rig', 'Computational Output & Metrics'] },
  { url: '#/telemetry', expect: ['Distributed SAM-T Mesh', 'Live Barometric Trim'] },
];

let failed = 0;
for (const c of checks) {
  await page.goto('http://localhost:3000/' + c.url, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1200);
  const body = await page.evaluate(() => document.body.innerText);
  const missing = c.expect.filter((t) => !body.includes(t));
  console.log(c.url, missing.length === 0 ? 'PASS' : 'FAIL missing=' + JSON.stringify(missing));
  if (missing.length) failed++;
}

// Verify header/nav/status shell present on any page
await page.goto('http://localhost:3000/#/3d', { waitUntil: 'networkidle' });
await page.waitForTimeout(800);
const shell = await page.evaluate(() => ({
  hasNav: Array.from(document.querySelectorAll('.nav-tab')).length,
  hasStatus: !!document.querySelector('.app-status'),
  hasBrand: !!document.querySelector('.app-brand'),
  active: document.querySelector('.nav-tab.active')?.getAttribute('href'),
}));
console.log('shell', JSON.stringify(shell));
if (shell.hasNav !== 4 || !shell.hasStatus || !shell.hasBrand) failed++;

await browser.close();
console.log(failed === 0 ? 'ALL_PASS' : 'HAS_FAILURES');
process.exit(failed === 0 ? 0 : 1);
