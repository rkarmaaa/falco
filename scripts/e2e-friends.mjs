// Prova end-to-end di M2 + M3 con due istanze sullo stesso PC:
//   A = istanza predefinita (host «Marco»), B = FALO_INSTANCE=b (ospite «Luca»).
// Prerequisiti: entrambi i core avviati, Tailscale collegato.
// Uso: node scripts/e2e-friends.mjs [porta-sunshine-A]
import { connect, sleep } from './lib/ipc.mjs';

const sunshinePort = Number(process.argv[2] ?? 47989);
const step = (s) => console.log(`\n▶ ${s}`);
let a, b;

async function ensureProfile(c, name) {
  const p = await c.call('profile.get');
  if (!p.exists) await c.call('profile.create', { name });
  return c.call('profile.get');
}

try {
  a = await connect(undefined, { label: 'A' });
  b = await connect('b', { label: 'B' });

  step('profili e porte');
  const pa = await ensureProfile(a, 'Marco');
  const pb = await ensureProfile(b, 'Luca');
  console.log(`  A = ${pa.name} ${pa.tag}   B = ${pb.name} ${pb.tag}`);
  await a.call('config.set', { key: 'network.sunshine_base_port', value: sunshinePort });
  await b.call('config.set', { key: 'network.presence_port', value: 47901 });
  await b.call('config.set', { key: 'network.sunshine_base_port', value: sunshinePort + 1000 });
  // Pulizia di prove precedenti
  for (const f of await a.call('friends.list')) await a.call('friend.remove', { id: f.id });
  for (const f of await b.call('friends.list')) await b.call('friend.remove', { id: f.id });
  await sleep(1000);

  step('A crea un invito (giocatore, 24 h, 1 uso)');
  const inv = await a.call('invite.create', { role: 'giocatore', validityHours: 24, uses: 1 });
  console.log(`  link (${inv.link.length} caratteri): ${inv.link.slice(0, 48)}…`);

  step('B vede l\'anteprima e accetta');
  const preview = await b.call('invite.preview', { link: inv.link });
  console.log(`  host ${preview.host.name} ${preview.host.tag}, ruolo ${preview.role}, controller ${preview.permissions.gamepads}`);
  const paired = b.waitFor((m) => m.method === 'invite.progress' && ['pairing', 'error'].includes(m.params.step) && m.params.ok !== null, 180_000);
  await b.call('invite.accept', { link: inv.link });
  const res = await paired;
  if (!res.ok) throw new Error(`accettazione fallita: ${res.message ?? res.step}`);

  step('il link a uso singolo non vale più');
  const again = b.waitFor((m) => m.method === 'invite.progress' && m.params.step === 'error', 60_000);
  await b.call('invite.accept', { link: inv.link });
  console.log(`  secondo uso rifiutato: ${(await again).message}`);

  step('amici da entrambe le parti, presenza entro 15 s');
  await a.call('friends.refresh');
  await b.call('friends.refresh');
  await sleep(4000);
  const fa = await a.call('friends.list');
  const fb = await b.call('friends.list');
  console.log(`  A vede: ${fa.map((f) => `${f.name} [${f.state}, ${f.role}]`).join(', ')}`);
  console.log(`  B vede: ${fb.map((f) => `${f.name} [${f.state}]`).join(', ')}`);
  const marco = fb.find((f) => f.name === 'Marco');
  if (!marco || !fa.some((f) => f.name === 'Luca')) throw new Error('amicizia non registrata');

  step('A disponibile → B vede Marco «in gioco» e preme Entra');
  const ready = a.waitFor((m) => m.method === 'host.state' && m.params.state === 'ready', 120_000);
  await a.call('host.setAvailable', { available: true });
  await ready;
  await b.call('friends.refresh');
  await sleep(4000);
  const state = (await b.call('friends.list')).find((f) => f.id === marco.id);
  console.log(`  stato di Marco per B: ${state.state}, posti liberi ${state.slotsFree}`);
  const connected = b.waitFor((m) => m.method === 'session.stats' && m.params.phase === 'connected', 60_000);
  await b.call('session.join', { friendId: marco.id });
  await connected;
  await sleep(6000);
  const ended = b.waitFor((m) => m.method === 'session.ended', 30_000);
  await a.call('host.endSession');
  console.log(`  sessione finita: ${JSON.stringify(await ended)}`);

  step('M4: A rende Luca «spettatore» → B deve bussare');
  const luca = (await a.call('friends.list')).find((f) => f.name === 'Luca');
  await a.call('friend.setPermissions', { id: luca.id, role: 'spettatore' });
  const perms = await a.call('friend.getPermissions', { id: luca.id });
  console.log(`  ruolo ${perms.role}, entra da solo: ${perms.effective.autoJoin}, client associato: ${perms.paired}`);
  let knocked = a.waitFor((m) => m.method === 'session.knock', 30_000);
  const connected2 = b.waitFor((m) => m.method === 'session.stats' && m.params.phase === 'connected', 90_000);
  const joining = b.call('session.join', { friendId: marco.id });
  console.log(`  A riceve la richiesta: ${(await knocked).name}`);
  await a.call('host.answerKnock', { friendId: luca.id, accept: true });
  await joining;
  await connected2;
  console.log(`  ospiti di A: ${JSON.stringify((await a.call('host.guests')).guests.map((g) => g.name))}`);
  const ended2 = b.waitFor((m) => m.method === 'session.ended', 30_000);
  await a.call('host.kick', { id: luca.id });
  await ended2;
  console.log('  espulso: sessione chiusa');
  await sleep(2000);
  console.log(`  host di A dopo l'espulsione: ${JSON.stringify((await a.call('state.snapshot')).host)}`);

  step('M4: B bussa di nuovo, A rifiuta');
  knocked = a.waitFor((m) => m.method === 'session.knock', 30_000);
  const refused = b.call('session.join', { friendId: marco.id }).then(
    () => 'entrato?!',
    (e) => e.message,
  );
  const first = await Promise.race([knocked.then(() => 'knock'), refused]);
  if (first !== 'knock') throw new Error(`B rifiutato prima di bussare: ${first}`);
  await a.call('host.answerKnock', { friendId: luca.id, accept: false });
  console.log(`  B riceve: ${await refused}`);

  step('B rimuove Marco → anche A perde Luca');
  await b.call('friend.remove', { id: marco.id });
  await sleep(1500);
  console.log(`  A vede ora: ${(await a.call('friends.list')).length} amici`);
  await a.call('host.setAvailable', { available: false });
  console.log('\n✔ M2 + M3 completati');
  process.exit(0);
} catch (e) {
  console.error(`\n✘ ${e.message}`);
  await a?.call('host.setAvailable', { available: false }).catch(() => {});
  process.exit(1);
}
