// Chiama un metodo del core e stampa la risposta.
// Uso: node scripts/rpc.mjs <metodo> ['{"param":1}']
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';

const [method, raw] = process.argv.slice(2);
const endpoint =
  process.platform === 'win32'
    ? String.raw`\\.\pipe\falo-core-` + (process.env.USERNAME ?? 'user')
    : path.join(os.homedir(), 'Library/Application Support/app.Falo.Falo/core.sock');

const s = net.connect(endpoint);
let buf = '';
s.on('connect', () =>
  s.write(JSON.stringify({ jsonrpc: '2.0', id: 1, method, params: raw ? JSON.parse(raw) : undefined }) + '\n'),
);
s.on('data', (d) => {
  buf += d;
  for (const line of buf.split('\n')) {
    if (line.includes('"id":1')) {
      console.log(line);
      s.end();
    }
  }
});
s.on('error', (e) => {
  console.error(`core non raggiungibile: ${e.message}`);
  process.exitCode = 1;
});
