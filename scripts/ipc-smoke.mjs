// Test di fumo dell'IPC: si collega al core in esecuzione e chiama alcuni metodi.
// Uso: node scripts/ipc-smoke.mjs [metodo]  (con falo-core già avviato)
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';

const endpoint =
  process.platform === 'win32'
    ? String.raw`\\.\pipe\falo-core-` + (process.env.USERNAME ?? 'user')
    : path.join(os.homedir(), 'Library/Application Support/app.Falo.Falo/core.sock');

const calls = process.argv[2] ? [[process.argv[2]]] : [
  ['state.snapshot'],
  ['config.set', { key: 'network.fec_percent', value: 200 }],
  ['host.setAvailable', { available: true }],
  ['invite.create', {}],
  ['net.runDoctor'],
  ['host.setAvailable', { available: false }],
];

const socket = net.connect(endpoint);
let buffer = '';
socket.on('data', (d) => (buffer += d));
socket.on('error', (e) => {
  console.error(`Core non raggiungibile su ${endpoint}: ${e.message}`);
  process.exit(1);
});
socket.on('connect', () => {
  calls.forEach(([method, params], i) =>
    socket.write(JSON.stringify({ jsonrpc: '2.0', id: i + 1, method, params }) + '\n'),
  );
  setTimeout(() => {
    for (const line of buffer.trim().split('\n')) console.log(line);
    socket.end();
  }, 2500);
});
