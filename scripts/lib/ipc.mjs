// Client JSON-RPC minimale verso il core, per script di prova.
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';

export function endpoint(instance = process.env.FALO_INSTANCE) {
  const suffix = instance ? `-${instance}` : '';
  if (process.platform === 'win32') return String.raw`\\.\pipe\falo-core-` + (process.env.USERNAME ?? 'user') + suffix;
  const dir = instance ? `app.Falo.Falo-${instance}` : 'app.Falo.Falo';
  return path.join(os.homedir(), 'Library/Application Support', dir, 'core.sock');
}

export function connect(instance, { log = true, label = instance ?? 'a' } = {}) {
  return new Promise((resolve, reject) => {
    const socket = net.connect(endpoint(instance));
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
          const { res, rej } = pending.get(msg.id);
          pending.delete(msg.id);
          msg.error ? rej(new Error(`${msg.error.message} (${msg.error.code})`)) : res(msg.result);
        } else if (msg.method) {
          if (log && msg.method !== 'presence.changed') console.log(`  [${label}] ← ${msg.method} ${JSON.stringify(msg.params)}`);
          for (const w of [...waiters])
            if (w.test(msg)) {
              waiters.splice(waiters.indexOf(w), 1);
              w.resolve(msg.params);
            }
        }
      }
    });
    socket.on('error', reject);
    socket.on('connect', () =>
      resolve({
        call: (method, params) =>
          new Promise((res, rej) => {
            const id = nextId++;
            pending.set(id, { res, rej });
            socket.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
          }),
        waitFor: (test, ms = 60_000) =>
          new Promise((res, rej) => {
            waiters.push({ test, resolve: res });
            setTimeout(() => rej(new Error(`[${label}] timeout in attesa di un evento`)), ms);
          }),
        close: () => socket.end(),
      }),
    );
  });
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
