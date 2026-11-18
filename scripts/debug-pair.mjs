// Debug: pairing del Moonlight dei motori con l'host A, con conferma del PIN e verifica con list.
import { spawn } from 'node:child_process';
import path from 'node:path';
import { connect, sleep } from './lib/ipc.mjs';

const target = process.argv[2] ?? '100.70.2.107:49989';
const a = await connect(undefined, { label: 'A', log: false });
await a.call('host.setAvailable', { available: true });
for (let i = 0; i < 60 && (await a.call('state.snapshot')).host.state !== 'ready'; i++) await sleep(1000);
const dir = path.resolve('target/debug/engines/moonlight');
const t0 = Date.now();
const ms = () => `${Date.now() - t0} ms`;

function list() {
  return new Promise((resolve) => {
    const p = spawn(path.join(dir, 'Moonlight.exe'), ['list', target], { cwd: dir });
    let out = '';
    p.stdout.on('data', (d) => (out += d));
    setTimeout(() => {
      p.kill();
      resolve(out.split('\n').filter((l) => l.trim() && !/^\d\d:\d\d:\d\d - /.test(l)));
    }, 6000);
  });
}

const ml = spawn(path.join(dir, 'Moonlight.exe'), ['pair', target, '--pin', '1357'], { cwd: dir });
ml.on('exit', (c) => console.log('pair uscito con', c, 'a', ms()));
for (let i = 0; i < 20; i++) {
  await sleep(500);
  const p = await a.call('host.pairings');
  if (p.length) {
    await a.call('host.approvePairing', { id: p[0].id, pin: '1357', name: 'Debug' });
    console.log('PIN confermato a', ms());
    break;
  }
}
await sleep(3000);
console.log('list durante pair:', JSON.stringify(await list()), ms());
ml.kill();
await sleep(1000);
console.log('list dopo pair:', JSON.stringify(await list()), ms());
process.exit(0);
