// Worker vero in locale (workerd + D1 in memoria): registrazione, richiesta, accettazione, link.
import { generateKeyPairSync, createHash, sign } from 'node:crypto';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { unstable_startWorker } from 'wrangler';
import { base32, codeOf, normalizeCode, validName } from '../src/auth';

type Worker = Awaited<ReturnType<typeof unstable_startWorker>>;
let worker: Worker;

beforeAll(async () => {
  worker = await unstable_startWorker({ config: 'wrangler.toml', dev: { server: { port: 0 }, inspector: false } });
}, 60_000);

afterAll(async () => {
  await worker?.dispose();
});

class Me {
  readonly id: string;
  private readonly key;
  constructor() {
    const { publicKey, privateKey } = generateKeyPairSync('ed25519');
    this.key = privateKey;
    const raw = publicKey.export({ format: 'der', type: 'spki' }).subarray(-32);
    this.id = base32(new Uint8Array(raw));
  }

  async call(method: string, path: string, body?: unknown) {
    const bytes = body === undefined ? Buffer.alloc(0) : Buffer.from(JSON.stringify(body));
    const ts = Math.floor(Date.now() / 1000);
    const digest = createHash('sha256').update(bytes).digest('hex');
    const sig = sign(null, Buffer.from(`${method}\n${path}\n${ts}\n${digest}`), this.key).toString('base64url');
    const r = await worker.fetch(`http://hub${path}`, {
      method,
      headers: { 'x-falco-id': this.id, 'x-falco-ts': String(ts), 'x-falco-sig': sig, 'content-type': 'application/json' },
      body: method === 'GET' ? undefined : bytes,
    });
    return { status: r.status, body: (await r.json().catch(() => null)) as Record<string, any> };
  }
}

describe('codici', () => {
  it('normalizza e valida', () => {
    expect(normalizeCode('#k7qm-2xpa')).toBe('K7QM2XPA');
    expect(normalizeCode('K7QM-2XP1')).toBeNull();
    expect(validName('  Marco ')).toBe('Marco');
    expect(validName('M')).toBeNull();
  });

  it('il codice è il tag del core', async () => {
    // Stesso calcolo di FalcoId::tag(): base32(sha256(chiave))[0..8]
    const me = new Me();
    expect(await codeOf(me.id)).toMatch(/^[A-Z2-7]{8}$/);
  });
});

describe('hub', () => {
  const invite = 'falco://invite/KLUv_test';

  it('richiesta d\'amicizia', async () => {
    const [anna, bruno] = [new Me(), new Me()];
    expect((await anna.call('GET', '/v1/requests')).status).toBe(401);
    const reg = await anna.call('POST', '/v1/me', { name: 'Anna' });
    expect(reg.status).toBe(200);
    expect(reg.body.code).toBe(await codeOf(anna.id));
    await bruno.call('POST', '/v1/me', { name: 'Bruno' });
    const code = (await codeOf(bruno.id))!;

    const sent = await anna.call('POST', '/v1/requests', { code: `${code.slice(0, 4)}-${code.slice(4)}`.toLowerCase(), invite });
    expect(sent.status).toBe(200);
    const list = await bruno.call('GET', '/v1/requests');
    expect(list.body.incoming[0].user.name).toBe('Anna');
    const rid = list.body.incoming[0].id;
    expect((await anna.call('POST', `/v1/requests/${rid}/accept`)).status).toBe(404);
    const acc = await bruno.call('POST', `/v1/requests/${rid}/accept`);
    expect(acc.body).toMatchObject({ invite, user: { id: anna.id } });
    expect((await anna.call('POST', '/v1/requests', { code, invite })).status).toBe(409);
  });

  it('link di stream', async () => {
    const [host, guest] = [new Me(), new Me()];
    await host.call('POST', '/v1/me', { name: 'Host' });
    await guest.call('POST', '/v1/me', { name: 'Ospite' });
    const link = await host.call('POST', '/v1/links', { invite, expiresAt: Math.floor(Date.now() / 1000) + 3600 });
    expect(link.body.url).toBe(`http://hub/j/${link.body.code}`);
    const page = await worker.fetch(`http://hub/j/${link.body.code}`);
    expect(await page.text()).toContain(`falco://join/${link.body.code}`);
    expect((await host.call('POST', `/v1/links/${link.body.code}/redeem`)).status).toBe(400);
    const r = await guest.call('POST', `/v1/links/${link.body.code}/redeem`);
    expect(r.body).toMatchObject({ invite, user: { name: 'Host' } });
    await host.call('DELETE', `/v1/links/${link.body.code}`);
    expect((await guest.call('POST', `/v1/links/${link.body.code}/redeem`)).status).toBe(404);
  });

  it('firma sbagliata e rete spenta', async () => {
    const me = new Me();
    const r = await worker.fetch('http://hub/v1/me', { method: 'POST', headers: { 'x-falco-id': me.id, 'x-falco-ts': '1', 'x-falco-sig': 'x' } });
    expect(r.status).toBe(401);
    await me.call('POST', '/v1/me', { name: 'Io' });
    expect((await me.call('POST', '/v1/net/key')).status).toBe(503);
  });
});
