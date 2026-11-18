// Prova di latenza su un solo PC: sessioni in loopback con parametri diversi, statistiche di Moonlight.
// Uso: node scripts/latency-bench.mjs [porta-base] [secondi-per-sessione]   (con falco-core avviato)
// La rete in loopback è ~0 ms: la prova misura soprattutto codifica, decodifica e rendering.
import { connect, sleep } from './lib/ipc.mjs';

const port = Number(process.argv[2] ?? 47989);
const seconds = Number(process.argv[3] ?? 15);
const c = await connect(undefined, { log: false });

const base = { 'stream.resolution': '1920x1080', 'stream.fps': 60, 'stream.vsync': false, 'stream.display_mode': 'windowed' };
const runs = [
  { name: 'HEVC, frame pacing spento', set: { 'stream.codec': 'hevc', 'stream.frame_pacing': false } },
  { name: 'HEVC, frame pacing acceso', set: { 'stream.codec': 'hevc', 'stream.frame_pacing': true } },
  { name: 'H.264, frame pacing spento', set: { 'stream.codec': 'h264', 'stream.frame_pacing': false } },
  { name: 'HEVC, V-Sync acceso', set: { 'stream.codec': 'hevc', 'stream.frame_pacing': false, 'stream.vsync': true } },
];

async function setAll(obj) {
  for (const [key, value] of Object.entries(obj)) await c.call('config.set', { key, value });
}

const before = (await c.call('config.get')).config;
await c.call('config.set', { key: 'network.sunshine_base_port', value: port });
const ready = c.waitFor((m) => m.method === 'host.state' && ['ready', 'error'].includes(m.params.state), 120_000);
await c.call('host.setAvailable', { available: true });
if ((await ready).state !== 'ready') throw new Error('host non pronto');

// Pairing con sé stesso se serve
const paired = c.waitFor((m) => m.method === 'pair.result', 120_000);
await c.call('session.pair', { host: '127.0.0.1', port });
if (!(await paired).ok) throw new Error('pairing non riuscito');

const results = [];
for (const run of runs) {
  await setAll({ ...base, ...run.set });
  const ended = c.waitFor((m) => m.method === 'session.ended', 60_000 + seconds * 1000);
  const connected = c.waitFor((m) => m.method === 'session.stats' && m.params.phase === 'connected', 30_000);
  await c.call('session.join', { host: '127.0.0.1', port, app: 'Desktop' });
  await connected;
  await sleep(seconds * 1000);
  await c.call('host.endSession');
  const e = await ended;
  results.push({ name: run.name, ...e.stats, total: e.latencyMs });
  await sleep(3000);
}

const f = (v) => (v === undefined || v === null ? '—' : Number(v).toFixed(2));
console.log('\nconfigurazione                 host   rete  decod  coda  render  TOTALE  fps   persi%');
for (const r of results) {
  console.log(
    `${r.name.padEnd(30)} ${f(r.hostLatencyMs).padStart(5)} ${f(r.networkLatencyMs).padStart(5)} ${f(r.decodeMs).padStart(6)} ${f(r.queueMs).padStart(5)} ${f(r.renderMs).padStart(7)} ${f(r.total).padStart(7)} ${f(r.renderFps).padStart(5)} ${f((r.droppedNetworkPct ?? 0) + (r.droppedJitterPct ?? 0)).padStart(7)}`,
  );
}

// Ripristina le impostazioni dell'utente
for (const k of ['resolution', 'fps', 'vsync', 'display_mode', 'codec', 'frame_pacing'])
  await c.call('config.set', { key: `stream.${k}`, value: before.stream[k] });
await c.call('host.setAvailable', { available: false });
process.exit(0);
