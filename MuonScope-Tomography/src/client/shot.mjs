import { chromium } from 'playwright-core';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = process.argv[2] || 'shots';
const out = path.join(__dirname, outDir);

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const PAGES = [
  { slug: '3d', url: '#/3d' },
  { slug: 'mpr', url: '#/mpr' },
  { slug: 'defects', url: '#/defects' },
  { slug: 'lab', url: '#/lab' },
  { slug: 'telemetry', url: '#/telemetry' },
];

const browser = await chromium.launch({ executablePath: CHROME, headless: true });
const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });

// Quick start guide (default route = 3D)
await page.goto('http://localhost:3000/#/3d', { waitUntil: 'networkidle' });
await page.waitForTimeout(1200);

const gb = page.getByText('Guide', { exact: true });
if (await gb.count() > 0) {
  await gb.first().click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(out, 'guide.png') });
  
  // Close the modal
  const closeBtn = page.getByText("Got it, let's explore!", { exact: true });
  if (await closeBtn.count() > 0) {
    await closeBtn.first().click();
    await page.waitForTimeout(400);
  }
}

for (const p of PAGES) {
  await page.goto('http://localhost:3000/' + p.url, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1500);
  await page.screenshot({ path: path.join(out, `page_${p.slug}.png`) });

  if (p.slug === 'mpr') {
    // Also capture bridge overpass in 2D MPR
    await page.selectOption('select', 'bridge_overpass');
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(out, 'mpr_bridge_overpass.png') });
  }
}

// Now capture each 3D preset
await page.goto('http://localhost:3000/#/3d', { waitUntil: 'networkidle' });
await page.waitForTimeout(1000);

const presets = [
  'bridge_overpass',
  'multistory_building',
  'subsurface_tunnel',
  'dam_dyke',
  'nuclear_cask',
  'astm_phantom'
];

for (const pr of presets) {
  await page.selectOption('select', pr);
  await page.waitForTimeout(1000);
  await page.screenshot({ path: path.join(out, `preset_${pr}.png`) });
}

// Capture sliced building view at Floor 2 (56%)
await page.selectOption('select', 'multistory_building');
await page.waitForTimeout(500);
const l2Btn = page.getByText('L2', { exact: true });
if (await l2Btn.count() > 0) {
  await l2Btn.first().click();
  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(out, 'building_dissected_l2.png') });
}

// Click on the 3D canvas to trigger hotspot or click first hotspot
await page.mouse.click(700, 450);
await page.waitForTimeout(600);
await page.screenshot({ path: path.join(out, 'building_hotspot_inspected.png') });

await browser.close();
console.log('done');
