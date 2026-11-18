// Debug: l'istanza B prova a entrare dal primo amico e stampa l'esito.
import { connect } from './lib/ipc.mjs';

const b = await connect('b', { label: 'B' });
const a = await connect(undefined, { label: 'A' });
const f = (await b.call('friends.list'))[0];
console.log('amico', f?.name, f?.state, 'canJoin', f?.canJoin);
const snapB = await b.call('state.snapshot');
console.log('B streaming', snapB.streaming);
const t0 = Date.now();
a.waitFor((m) => m.method === 'session.knock', 40_000).then(
  async (k) => {
    console.log('A riceve knock da', k.name, 'dopo', Date.now() - t0, 'ms');
    await a.call('host.answerKnock', { friendId: k.friendId, accept: false });
  },
  () => console.log('A: nessun knock'),
);
try {
  await b.call('session.join', { friendId: f.id });
  console.log('B: entrato');
} catch (e) {
  console.log('B errore:', e.message, 'dopo', Date.now() - t0, 'ms');
}
process.exit(0);
