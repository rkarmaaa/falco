// Falco Hub su Cloudflare Workers + D1 (ADR 0017): codici amico, richieste d'amicizia,
// link di stream e chiavi monouso per la rete Falco. Non vede gli stream e non decide i permessi:
// l'invito allegato è firmato dall'host e il core lo verifica nell'handshake (§6).
import { codeOf, displayCode, normalizeCode, randomCode, validName, verify } from './auth';
import { addPage, joinPage, missingPage } from './pages';
import { ensureSchema } from './schema';
import { authKey } from './tailscale';

export interface Env {
  DB: D1Database;
  TS_TAG: string;
  DOWNLOAD_URL: string;
  TS_OAUTH_ID?: string;
  TS_OAUTH_SECRET?: string;
}

/** Validità degli inviti allegati alle richieste (come `REQUEST_VALIDITY_H` del core). */
const REQUEST_TTL_S = 168 * 3600;
const MAX_LINK_TTL_S = 7 * 86400;
const MAX_BODY = 16 * 1024;

interface UserRef {
  id: string;
  name: string;
  code: string;
}

class HttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

const json = (data: unknown, status = 200) =>
  new Response(JSON.stringify(data), { status, headers: { 'content-type': 'application/json; charset=utf-8' } });
const html = (body: string, status = 200) =>
  new Response(body, { status, headers: { 'content-type': 'text/html; charset=utf-8' } });
const now = () => Math.floor(Date.now() / 1000);
const pair = (x: string, y: string) => (x < y ? [x, y] : [y, x]);

function body<T>(raw: Uint8Array): T {
  try {
    return JSON.parse(new TextDecoder().decode(raw)) as T;
  } catch {
    throw new HttpError(400, 'richiesta non valida');
  }
}

/** Contesto di una richiesta firmata. */
interface Ctx {
  env: Env;
  me: string;
  raw: Uint8Array;
  origin: string;
}

async function user(db: D1Database, id: string): Promise<UserRef | null> {
  return db.prepare('SELECT id, name, code FROM users WHERE id = ?1').bind(id).first<UserRef>();
}

async function areFriends(db: D1Database, x: string, y: string): Promise<boolean> {
  const [a, b] = pair(x, y);
  return !!(await db.prepare('SELECT 1 FROM friendships WHERE a = ?1 AND b = ?2').bind(a, b).first());
}

async function addFriendship(db: D1Database, x: string, y: string): Promise<D1PreparedStatement> {
  const [a, b] = pair(x, y);
  return db.prepare('INSERT OR IGNORE INTO friendships (a, b, created_at) VALUES (?1, ?2, ?3)').bind(a, b, now());
}

// --- API firmate ---

async function register({ env, me, raw, origin }: Ctx): Promise<Response> {
  const name = validName(body<{ name?: unknown }>(raw).name);
  if (!name) throw new HttpError(400, 'nome non valido: da 2 a 24 caratteri');
  const code = await codeOf(me);
  if (!code) throw new HttpError(400, 'ID non valido');
  try {
    await env.DB.prepare(
      `INSERT INTO users (id, code, name, created_at, seen_at) VALUES (?1, ?2, ?3, ?4, ?4)
       ON CONFLICT(id) DO UPDATE SET name = excluded.name, seen_at = excluded.seen_at`,
    )
      .bind(me, code, name, now())
      .run();
  } catch {
    // Collisione di codice (40 bit): rarissima
    throw new HttpError(409, 'codice amico già in uso: crea un nuovo profilo');
  }
  return json({ code, friendLink: `${origin}/a/${code}` });
}

async function listRequests({ env, me }: Ctx): Promise<Response> {
  const since = now() - REQUEST_TTL_S;
  const q = (dir: 'from_id' | 'to_id', other: 'from_id' | 'to_id') =>
    env.DB.prepare(
      `SELECT r.id, r.created_at AS createdAt, u.id AS uid, u.name, u.code FROM requests r
       JOIN users u ON u.id = r.${other} WHERE r.${dir} = ?1 AND r.created_at > ?2 ORDER BY r.created_at DESC`,
    )
      .bind(me, since)
      .all<{ id: string; createdAt: number; uid: string; name: string; code: string }>();
  const shape = (rows: { results: { id: string; createdAt: number; uid: string; name: string; code: string }[] }) =>
    rows.results.map((r) => ({ id: r.id, createdAt: r.createdAt, user: { id: r.uid, name: r.name, code: r.code } }));
  const [incoming, outgoing] = await Promise.all([q('to_id', 'from_id'), q('from_id', 'to_id')]);
  return json({ incoming: shape(incoming), outgoing: shape(outgoing) });
}

async function sendRequest({ env, me, raw }: Ctx): Promise<Response> {
  const req = body<{ code?: string; invite?: string }>(raw);
  const code = normalizeCode(req.code ?? '');
  if (!code) throw new HttpError(400, 'codice amico non valido');
  if (!req.invite?.startsWith('falco://invite/') || req.invite.length > 4096) throw new HttpError(400, 'invito non valido');
  const to = await env.DB.prepare('SELECT id, name, code FROM users WHERE code = ?1').bind(code).first<UserRef>();
  if (!to) throw new HttpError(404, 'nessuno ha questo codice amico');
  if (to.id === me) throw new HttpError(400, 'questo è il tuo codice');
  if (await areFriends(env.DB, me, to.id)) throw new HttpError(409, `tu e ${to.name} siete già amici`);
  const reverse = await env.DB.prepare('SELECT id FROM requests WHERE from_id = ?1 AND to_id = ?2').bind(to.id, me).first<{ id: string }>();
  if (reverse) return json({ id: reverse.id, user: to, theyAsked: true });
  const id = randomCode();
  await env.DB.prepare(
    `INSERT INTO requests (id, from_id, to_id, invite, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(from_id, to_id) DO UPDATE SET id = excluded.id, invite = excluded.invite, created_at = excluded.created_at`,
  )
    .bind(id, me, to.id, req.invite, now())
    .run();
  return json({ id, user: to });
}

async function acceptRequest({ env, me }: Ctx, id: string): Promise<Response> {
  const r = await env.DB.prepare('SELECT from_id, invite FROM requests WHERE id = ?1 AND to_id = ?2')
    .bind(id, me)
    .first<{ from_id: string; invite: string }>();
  if (!r) throw new HttpError(404, 'richiesta non trovata o già gestita');
  await env.DB.batch([await addFriendship(env.DB, me, r.from_id), env.DB.prepare('DELETE FROM requests WHERE id = ?1').bind(id)]);
  return json({ invite: r.invite, user: await user(env.DB, r.from_id) });
}

async function dropRequest({ env, me }: Ctx, id: string): Promise<Response> {
  await env.DB.prepare('DELETE FROM requests WHERE id = ?1 AND (to_id = ?2 OR from_id = ?2)').bind(id, me).run();
  return json({});
}

async function unfriend({ env, me }: Ctx, other: string): Promise<Response> {
  const [a, b] = pair(me, other);
  await env.DB.prepare('DELETE FROM friendships WHERE a = ?1 AND b = ?2').bind(a, b).run();
  return json({});
}

async function createLink({ env, me, raw, origin }: Ctx): Promise<Response> {
  const req = body<{ invite?: string; expiresAt?: number }>(raw);
  if (!req.invite?.startsWith('falco://invite/') || req.invite.length > 4096) throw new HttpError(400, 'invito non valido');
  const exp = Math.min(Number(req.expiresAt) || now() + 86400, now() + MAX_LINK_TTL_S);
  const code = randomCode();
  await env.DB.prepare('INSERT INTO links (code, host_id, invite, expires_at) VALUES (?1, ?2, ?3, ?4)').bind(code, me, req.invite, exp).run();
  return json({ code, url: `${origin}/j/${code}`, expiresAt: exp });
}

async function link(db: D1Database, code: string) {
  return db
    .prepare(
      `SELECT u.id, u.name, u.code, l.invite FROM links l JOIN users u ON u.id = l.host_id
       WHERE l.code = ?1 AND l.expires_at > ?2`,
    )
    .bind(code, now())
    .first<UserRef & { invite: string }>();
}

async function redeemLink({ env, me }: Ctx, code: string): Promise<Response> {
  const l = await link(env.DB, code);
  if (!l) throw new HttpError(404, 'link scaduto o revocato');
  if (l.id === me) throw new HttpError(400, 'è il tuo link: mandalo a un amico');
  // Chi entra con un link diventa amico dell'host con il ruolo dell'invito (§6)
  await (await addFriendship(env.DB, me, l.id)).run();
  return json({ invite: l.invite, user: { id: l.id, name: l.name, code: l.code } });
}

async function revokeLink({ env, me }: Ctx, code: string): Promise<Response> {
  await env.DB.prepare('DELETE FROM links WHERE code = ?1 AND host_id = ?2').bind(code, me).run();
  return json({});
}

async function netKey({ env, me }: Ctx): Promise<Response> {
  if (!env.TS_OAUTH_ID || !env.TS_OAUTH_SECRET) throw new HttpError(503, 'la rete Falco non è attiva su questo server');
  const code = (await user(env.DB, me))?.code ?? '';
  try {
    const key = await authKey(env.TS_OAUTH_ID, env.TS_OAUTH_SECRET, env.TS_TAG, `falco ${displayCode(code)}`);
    return json({ loginServer: '', authKey: key });
  } catch (e) {
    console.error('chiave di rete', e);
    throw new HttpError(502, 'rete Falco non raggiungibile, riprova tra poco');
  }
}

type Handler = (ctx: Ctx, param: string) => Promise<Response>;

const routes: [string, RegExp, Handler, boolean][] = [
  // metodo, percorso, gestore, serve la registrazione
  ['POST', /^\/v1\/me$/, register, false],
  ['GET', /^\/v1\/requests$/, listRequests, true],
  ['POST', /^\/v1\/requests$/, sendRequest, true],
  ['POST', /^\/v1\/requests\/([A-Z2-7]{8})\/accept$/, acceptRequest, true],
  ['POST', /^\/v1\/requests\/([A-Z2-7]{8})\/decline$/, dropRequest, true],
  ['DELETE', /^\/v1\/friends\/([A-Z2-7]{52})$/, unfriend, true],
  ['POST', /^\/v1\/links$/, createLink, true],
  ['DELETE', /^\/v1\/links\/([A-Z2-7]{8})$/, revokeLink, true],
  ['POST', /^\/v1\/links\/([A-Z2-7]{8})\/redeem$/, redeemLink, true],
  ['POST', /^\/v1\/net\/key$/, netKey, true],
];

async function api(request: Request, env: Env, url: URL): Promise<Response> {
  const route = routes.find(([m, re]) => m === request.method && re.test(url.pathname));
  if (!route) throw new HttpError(404, 'percorso sconosciuto');
  const [, re, handler, needsUser] = route;
  const raw = new Uint8Array(await request.arrayBuffer());
  if (raw.length > MAX_BODY) throw new HttpError(413, 'richiesta troppo grande');
  const auth = await verify(request.headers, request.method, url.pathname, raw, now());
  if ('error' in auth) throw new HttpError(401, auth.error);
  if (needsUser && !(await user(env.DB, auth.id))) throw new HttpError(401, 'profilo non registrato sul server');
  return handler({ env, me: auth.id, raw, origin: url.origin }, re.exec(url.pathname)?.[1] ?? '');
}

async function pages(env: Env, url: URL): Promise<Response> {
  const [, kind, raw = ''] = url.pathname.split('/');
  const code = normalizeCode(raw);
  if (code && kind === 'a') {
    const u = await env.DB.prepare('SELECT name, code FROM users WHERE code = ?1').bind(code).first<UserRef>();
    if (u) return html(addPage(u.name, u.code, env.DOWNLOAD_URL));
  }
  if (code && kind === 'j') {
    const l = await link(env.DB, code);
    if (l) return html(joinPage(l.name, code, env.DOWNLOAD_URL));
  }
  return html(missingPage(env.DOWNLOAD_URL), 404);
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    try {
      if (url.pathname === '/healthz') return new Response('ok');
      await ensureSchema(env.DB);
      if (url.pathname.startsWith('/v1/')) return await api(request, env, url);
      if (request.method === 'GET') return await pages(env, url);
      throw new HttpError(404, 'percorso sconosciuto');
    } catch (e) {
      if (e instanceof HttpError) return json({ error: e.message }, e.status);
      console.error(e);
      return json({ error: 'errore interno del server' }, 500);
    }
  },

  async scheduled(_: ScheduledController, env: Env): Promise<void> {
    await ensureSchema(env.DB);
    await env.DB.batch([
      env.DB.prepare('DELETE FROM links WHERE expires_at <= ?1').bind(now()),
      env.DB.prepare('DELETE FROM requests WHERE created_at <= ?1').bind(now() - REQUEST_TTL_S),
    ]);
  },
} satisfies ExportedHandler<Env>;
