#!/usr/bin/env node
// Measure real HTTP traffic and keep screenshots from a browser session.
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const { chromium } = require('playwright');

async function main() {
  const [url, output] = process.argv.slice(2);
  if (!url || !output) throw new Error('Usage: node scripts/measure_web.cjs URL OUTPUT_DIRECTORY');
  await fs.mkdir(output, { recursive: true });
  const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'ambition-web-'));
  const context = await chromium.launchPersistentContext(profile, {
    viewport: { width: 1024, height: 576 },
    executablePath: process.env.AMBITION_BROWSER || undefined,
    args: process.env.AMBITION_SOFTWARE_GPU ? ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] : [],
  });
  const page = context.pages()[0] || await context.newPage();
  await page.addInitScript(() => {
    for (const name of ['instantiate', 'instantiateStreaming']) {
      const original = WebAssembly[name];
      WebAssembly[name] = async (...args) => {
        const result = await original(...args);
        const instance = result.instance || result;
        if (instance.exports.memory) window.__measuredWasmMemory = instance.exports.memory;
        return result;
      };
    }
  });
  const session = await context.newCDPSession(page);
  await session.send('Network.enable');
  const requests = new Map();
  const errors = [];
  const messages = [];
  let visit = 'cold';
  const timings = {};
  session.on('Network.responseReceived', ({ requestId, response }) => requests.set(requestId, {
    visit, url: response.url, status: response.status, mime_type: response.mimeType,
    cached: response.fromDiskCache || false,
    content_encoding: response.headers["Content-Encoding"] || response.headers["content-encoding"] || null,
  }));
  session.on('Network.loadingFinished', ({ requestId, encodedDataLength }) => {
    if (requests.has(requestId)) requests.get(requestId).transfer_bytes = encodedDataLength;
  });
  session.on('Network.loadingFailed', event => errors.push({ visit, network: event.errorText }));
  page.on('pageerror', error => errors.push({ visit, error: String(error) }));
  page.on('console', message => messages.push({ visit, type: message.type(), text: message.text() }));
  try {
    for (const name of ['cold', 'warm']) {
      visit = name;
      await page.goto(url, { waitUntil: 'commit', timeout: 180000 });
      await page.waitForFunction(() => {
        const canvas = document.getElementById('bevy');
        return canvas?.width > 0 && canvas?.height > 0;
      }, { timeout: 180000 });
      timings[name] = await page.evaluate(() => ({
        canvas_ms: performance.getEntriesByName('mary-o-canvas-start')[0]?.startTime || performance.now(),
      }));
      await page.locator('#bevy').click();
      await page.waitForTimeout(2500);
      Object.assign(timings[name], await page.evaluate(() => ({
        capture_ms: performance.now(),
        wasm_memory_bytes: window.__measuredWasmMemory?.buffer.byteLength || null,
        js_heap_bytes: performance.memory?.usedJSHeapSize || null,
      })));
      await page.screenshot({ path: path.join(output, `${name}.png`) });
      if (name === 'cold') {
        await page.keyboard.down('ArrowRight');
        await page.keyboard.down('z');
        await page.waitForTimeout(250);
        await page.keyboard.up('z');
        await page.keyboard.up('ArrowRight');
        await page.screenshot({ path: path.join(output, 'after-input.png') });
      }
    }
  } finally {
    const resources = [...requests.values()];
    const report = {
      url, user_agent: await page.evaluate(() => navigator.userAgent).catch(() => ''),
      software_gpu: !!process.env.AMBITION_SOFTWARE_GPU,
      cache: 'Fresh temporary persistent Chromium profile; both visits use the same profile',
      visits: Object.fromEntries(['cold', 'warm'].map(name => [name, {
        ...timings[name],
        transfer_bytes: resources.filter(r => r.visit === name).reduce((sum, r) => sum + (r.transfer_bytes || 0), 0),
        requests: resources.filter(r => r.visit === name).length,
        http_errors: resources.filter(r => r.visit === name && r.status >= 400).length,
      }])),
      resources, errors,
    };
    await fs.writeFile(path.join(output, 'network.json'), JSON.stringify(report, null, 2) + '\n');
    await fs.writeFile(path.join(output, 'console.json'), JSON.stringify(messages, null, 2) + '\n');
    console.log(JSON.stringify(report.visits, null, 2));
    await context.close();
    await fs.rm(profile, { recursive: true, force: true });
    if (errors.length) throw new Error(`${errors.length} browser errors; see ${output}/network.json`);
  }
}

main().catch(error => { console.error(error); process.exitCode = 1; });
