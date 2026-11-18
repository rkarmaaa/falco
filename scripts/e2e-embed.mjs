// Prova dello stream incorporato su un solo PC: host in loopback, sessione da sé stessi,
// la finestra di Falco resta aperta e incorpora Moonlight (ADR 0015).
// Uso: node scripts/e2e-embed.mjs [porta-base] [secondi]   (con falco-core e `npm run dev` avviati)
import { connect, sleep } from './lib/ipc.mjs';

const port = Number(process.argv[2] ?? 49989);
const seconds = Number(process.argv[3] ?? 25);
const c = await connect(undefined, { log: false });

await c.call('config.set', { key: 'network.sunshine_base_port', value: port });
const ready = c.waitFor((m) => m.method === 'host.state' && ['ready', 'error'].includes(m.params.state), 120_000);
await c.call('host.setAvailable', { available: true });
const st = await ready;
if (st.state !== 'ready') throw new Error(`host non pronto: ${st.message ?? ''}`);
console.log('host pronto');

const paired = c.waitFor((m) => m.method === 'pair.result', 120_000);
await c.call('session.pair', { host: '127.0.0.1', port });
if (!(await paired).ok) throw new Error('pairing non riuscito');
console.log('associato');

await c.call('app.openUi', {});
await sleep(4000);
const started = c.waitFor((m) => m.method === 'session.started', 30_000);
const connected = c.waitFor((m) => m.method === 'session.stats' && m.params.phase === 'connected', 30_000);
await c.call('session.join', { host: '127.0.0.1', port, app: 'Desktop' });
console.log('sessione avviata, pid', (await started).pid);
await connected;
console.log(`connesso: guardo per ${seconds} s`);
await sleep(seconds * 1000);

const ended = c.waitFor((m) => m.method === 'session.ended', 30_000);
await c.call('session.leave');
const e = await ended;
console.log('fine sessione:', e.reason);
await c.call('host.setAvailable', { available: false });
process.exit(0);
