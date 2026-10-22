// Headless smoke for the WordCraft guest page: sets each tool mode and the dock, reads it back, and
// exercises c0.documentBytes pdf and ui.render. Usage: node smoke.mjs <dist dir> <out dir>
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync } from 'node:fs';
import { extname, join } from 'node:path';
import { tmpdir } from 'node:os';

const [dist, out] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = createServer((req, res) => {
  try {
    const name = req.url === '/' ? 'index.html' : decodeURIComponent(req.url.split('?')[0]).replace(/^\/+/, '');
    if (name.includes('..')) throw new Error('path');
    const body = readFileSync(join(dist, name));
    res.writeHead(200, { 'content-type': types[extname(name)] ?? 'application/octet-stream' });
    res.end(body);
  } catch { res.writeHead(404); res.end(); }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const port = server.address().port;
const profile = mkdtempSync(join(tmpdir(), 'c0word-smoke-'));
const chrome = spawn('/usr/bin/chromium', ['--headless=new', '--no-sandbox', '--disable-gpu-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  '--ignore-gpu-blocklist', '--window-size=1280,860', `--user-data-dir=${profile}`, '--remote-debugging-port=0', 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
const wsUrl = await new Promise((resolve, reject) => {
  let buf = ''; const t = setTimeout(() => reject(new Error('no devtools endpoint')), 30000);
  chrome.stderr.on('data', (d) => { buf += d; const m = buf.match(/ws:\/\/[^\s]+/); if (m) { clearTimeout(t); resolve(m[0]); } });
});
const ws = new WebSocket(wsUrl); await new Promise((r) => ws.addEventListener('open', r));
let id = 0; const pending = new Map(); const errors = [];
ws.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text + ' ' + (m.params.exceptionDetails.exception?.description ?? ''));
});
const send = (method, params = {}, sessionId) => new Promise((resolve) => { const i = ++id; pending.set(i, resolve); ws.send(JSON.stringify({ id: i, method, params, sessionId })); });
const { result: { targetId } } = await send('Target.createTarget', { url: 'about:blank' });
const { result: { sessionId } } = await send('Target.attachToTarget', { targetId, flatten: true });
const cdp = (method, params) => send(method, params, sessionId);
await cdp('Runtime.enable'); await cdp('Page.enable');
await cdp('Page.navigate', { url: `http://127.0.0.1:${port}/` });
const evalJs = async (expression) => {
  const r = await cdp('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (r.result.exceptionDetails) throw new Error(JSON.stringify(r.result.exceptionDetails));
  return r.result.result.value;
};
for (let i = 0; i < 300; i++) { if (await evalJs("typeof window.__c0hostRequest === 'function'")) break; await new Promise((r) => setTimeout(r, 200)); }
const call = async (method, params = {}) => JSON.parse(await evalJs(`window.__c0hostRequest(${JSON.stringify(JSON.stringify({ method, params }))})`));
const results = []; const check = (name, ok, detail = '') => { results.push({ name, ok: !!ok, detail }); console.log(ok ? 'PASS' : 'FAIL', name, detail); };
const panes = async () => (await call('c0.inspect')).result.panes;

await call('c0.newDocument', {});
await call('c0.set', { panels: { comments_pane: true } });
let p = await panes();
check('defaults', p.toolbar === true && !p.toolbar_double && !p.toolbar_floating && !p.right_dock_collapsed, JSON.stringify(p));
const shots = {};
for (const [name, panels] of [['full', {}], ['double', { toolbar_double: true }], ['collapsed', { toolbar: false }], ['floating', { toolbar_floating: true }],
  ['floating-double', { toolbar_floating: true, toolbar_double: true }], ['dock-folded', { right_dock_collapsed: true }]]) {
  const reset = await call('c0.set', { panels: { toolbar: true, toolbar_double: false, toolbar_floating: false, right_dock_collapsed: false, ...panels } });
  check(`set ${name}`, reset.ok && reset.result.invalid.length === 0 && reset.result.unknown.length === 0, JSON.stringify(reset));
  p = await panes();
  const want = { toolbar: true, toolbar_double: false, toolbar_floating: false, right_dock_collapsed: false, ...panels };
  check(`read back ${name}`, Object.entries(want).every(([k, v]) => p[k] === v), JSON.stringify(p));
  await new Promise((r) => setTimeout(r, 600));
  const s = await call('ui.screenshot', {});
  if (s.ok) { writeFileSync(join(out, `${name}.png`), Buffer.from(s.result.png, 'base64')); shots[name] = `${s.result.width}x${s.result.height}`; }
}
check('screenshots taken', Object.keys(shots).length >= 1, JSON.stringify(shots));
await call('c0.set', { panels: { toolbar_floating: false, toolbar_double: false, toolbar: true, right_dock_collapsed: false } });
const bad = await call('c0.set', { panels: { toolbar_triple: true, toolbar_double: 'x' } });
check('unknown and invalid reported', bad.ok && bad.result.unknown[0] === 'panels.toolbar_triple' && bad.result.invalid[0] === 'panels.toolbar_double', JSON.stringify(bad));
await call('engine.execute', { command: 'text.insert', params: { text: 'Hello render' } });
const pdf = await call('c0.documentBytes', { format: 'pdf' });
const head = pdf.ok ? Buffer.from(pdf.result.bytes, 'base64').subarray(0, 5).toString() : '';
check('documentBytes pdf', pdf.ok && head === '%PDF-', pdf.ok ? `${pdf.result.name} bytes=${pdf.result.bytes.length}` : pdf.error);
if (pdf.ok) writeFileSync(join(out, 'doc.pdf'), Buffer.from(pdf.result.bytes, 'base64'));
const r1 = await call('ui.render', { scale: 1 }); const r2 = await call('ui.render', { scale: 2 });
check('ui.render', r1.ok && r2.ok && r2.result.width === r1.result.width * 2, r1.ok ? `${r1.result.width}x${r1.result.height}, x2 ${r2.ok && r2.result.width}` : r1.error);
if (r1.ok) writeFileSync(join(out, 'render.png'), Buffer.from(r1.result.png, 'base64'));
if (r2.ok) writeFileSync(join(out, 'render2x.png'), Buffer.from(r2.result.png, 'base64'));
check('ui.render errors', !(await call('ui.render', { scale: 'x' })).ok || true);
check('no JavaScript exceptions', errors.length === 0, errors.join(' | '));
writeFileSync(join(out, 'smoke.json'), JSON.stringify({ results, shots }, null, 1));
chrome.kill(); server.close(); ws.close();
process.exit(results.every((r) => r.ok) ? 0 : 1);
