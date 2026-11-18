// Debug: stream di prova verso l'host A e stampa delle righe di statistiche di Moonlight.
import { spawn } from 'node:child_process';
import path from 'node:path';
import { connect, sleep } from './lib/ipc.mjs';

const target = process.argv[2] ?? '100.70.2.107:49989';
const extra = process.argv.slice(3);
const a = await connect(undefined, { label: 'A', log: false });
await a.call('host.setAvailable', { available: true });
for (let i = 0; i < 60 && (await a.call('state.snapshot')).host.state !== 'ready'; i++) await sleep(1000);
const dir = path.resolve('target/debug/engines/moonlight');
const args = ['stream', target, 'Desktop', '--720', '--fps', '60', '--bitrate', '20000', '--display-mode', 'windowed', '--quit-after', ...extra];
const ml = spawn(path.join(dir, 'Moonlight.exe'), args, { cwd: dir });
const lines = [];
ml.stdout.on('data', (d) => lines.push(...String(d).split('\n')));
ml.stderr.on('data', (d) => lines.push(...String(d).split('\n')));
await sleep(12000);
await a.call('host.endSession');
await sleep(4000);
ml.kill();
console.log(lines.filter((l) => l.trim() && !/ULA|mDNS|gamecontroller|translation|Executing request/.test(l)).slice(-45).join('\n'));
process.exit(0);

