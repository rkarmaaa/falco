// Test end-to-end su un solo PC, solo tramite IPC (criteri M1):
// host disponibile → pairing automatico → streaming per qualche secondo → fine sessione.
// Uso: node scripts/e2e-loopback.mjs [porta-base] [secondi]   (con falo-core avviato)
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';

const port = Number(process.argv[2] ?? 47989);
const seconds = Number(process.argv[3] ?? 10);
const endpoint =
  process.platform === 'win32'
    ? String.raw`\\.\pipe\falo-core-` + (process.env.USERNAME ?? 'user')
    : path.join(os.homedir(), 'Library/Application Support/app.Falo.Falo/core.sock');

const socket = net.connect(endpoint);
const pending = new Map();
const waiters = [];
let nextId = 1;
let buffer = '';

socket.on('data', (d) => {
  buffer += d;
  let i;
  while ((i = buffer.indexOf('\n')) >= 0) {
    const msg = JSON.parse(buffer.slice(0, i));
    buffer = buffer.slice(i + 1);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
    } else if (msg.method) {
      console.log(`  ← evento ${msg.method} ${JSON.stringify(msg.params)}`);
      for (const w of [...waiters]) if (w.test(msg)) { waiters.splice(waiters.indexOf(w), 1); w.resolve(msg.params); }
    }
  }
});

const call = (method, params) =>
  new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    socket.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  });

const waitFor = (test, ms) =>
  new Promise((resolve, reject) => {
    const w = { test, resolve };
    waiters.push(w);
    setTimeout(() => reject(new Error(`timeout in attesa di un evento (${ms} ms)`)), ms);
  });

const step = (s) => console.log(`\n▶ ${s}`);

socket.on('connect', async () => {
  try {
    step(`porta base ${port}`);
    await call('config.set', { key: 'network.sunshine_base_port', value: port });

    step('host disponibile (avvio di Sunshine)');
    const ready = waitFor((m) => m.method === 'host.state' && ['ready', 'error'].includes(m.params.state), 120_000);
    await call('host.setAvailable', { available: true });
    const st = await ready;
    if (st.state !== 'ready') throw new Error(`host in errore: ${st.message}`);

    step('pairing automatico con questo PC');
    const paired = waitFor((m) => m.method === 'pair.result', 60_000);
    console.log('  PIN', (await call('session.pair', { host: '127.0.0.1', port })).pin);
    const pr = await paired;
    if (!pr.ok) throw new Error('pairing fallito');
    console.log('  client associati:', JSON.stringify(await call('host.clients')));

    step(`streaming per ${seconds} s`);
    const connected = waitFor((m) => m.method === 'session.stats' && m.params.phase === 'connected', 30_000);
    await call('session.join', { host: '127.0.0.1', port, app: 'Desktop' });
    await connected;
    await new Promise((r) => setTimeout(r, seconds * 1000));

    step('fine sessione dall\'host');
    const ended = waitFor((m) => m.method === 'session.ended', 30_000);
    await call('host.endSession');
    console.log('  sessione terminata:', JSON.stringify(await ended));

    step('host non disponibile');
    await call('host.setAvailable', { available: false });
    console.log('\n✔ M1 loopback completato');
    process.exit(0);
  } catch (e) {
    console.error(`\n✘ ${e.message}`);
    await call('host.setAvailable', { available: false }).catch(() => {});
    process.exit(1);
  }
});
socket.on('error', (e) => {
  console.error(`Core non raggiungibile: ${e.message}`);
  process.exit(1);
});
