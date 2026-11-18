// Chiavi monouso per la rete Falco: un'unica tailnet di Falco, i dispositivi entrano con il tag
// `tag:falco` senza account (API di Tailscale con un OAuth client di Falco, scope `auth_keys`).

const API = 'https://api.tailscale.com/api/v2';
/** Il tempo di collegarsi: poi la chiave non serve più. */
const KEY_TTL_S = 15 * 60;

async function token(id: string, secret: string): Promise<string> {
  const r = await fetch(`${API}/oauth/token`, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ client_id: id, client_secret: secret, grant_type: 'client_credentials' }),
  });
  if (!r.ok) throw new Error(`oauth ${r.status}: ${await r.text()}`);
  return ((await r.json()) as { access_token: string }).access_token;
}

export async function authKey(id: string, secret: string, tag: string, description: string): Promise<string> {
  const r = await fetch(`${API}/tailnet/-/keys`, {
    method: 'POST',
    headers: { authorization: `Bearer ${await token(id, secret)}`, 'content-type': 'application/json' },
    body: JSON.stringify({
      capabilities: { devices: { create: { reusable: false, ephemeral: false, preauthorized: true, tags: [tag] } } },
      expirySeconds: KEY_TTL_S,
      description: description.replace(/[^A-Za-z0-9 -]/g, '').slice(0, 50),
    }),
  });
  if (!r.ok) throw new Error(`keys ${r.status}: ${await r.text()}`);
  return ((await r.json()) as { key: string }).key;
}
