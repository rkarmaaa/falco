// Firma FPP (crates/falco-proto/src/fpp.rs): `METODO\npercorso\nts\nsha256(corpo)` firmato Ed25519.
// L'ID è la chiave pubblica in base32 senza padding; il codice amico deriva dal suo SHA-256.

const B32 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
export const MAX_SKEW_S = 60;

export function base32(bytes: Uint8Array): string {
  let out = '';
  let bits = 0;
  let value = 0;
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += B32[(value << (5 - bits)) & 31];
  return out;
}

export function fromBase32(s: string): Uint8Array | null {
  const out: number[] = [];
  let bits = 0;
  let value = 0;
  for (const c of s) {
    const i = B32.indexOf(c);
    if (i < 0) return null;
    value = (value << 5) | i;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 255);
      bits -= 8;
    }
  }
  return new Uint8Array(out);
}

function fromBase64Url(s: string): Uint8Array | null {
  try {
    const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((s.length + 3) % 4));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

async function sha256(data: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.digest('SHA-256', data));
}

const hex = (b: Uint8Array) => [...b].map((x) => x.toString(16).padStart(2, '0')).join('');

/** Codice amico: i primi 8 caratteri base32 dello SHA-256 della chiave (il tag senza `#` e `-`). */
export async function codeOf(id: string): Promise<string | null> {
  const key = fromBase32(id);
  if (!key || key.length !== 32) return null;
  return base32(await sha256(key)).slice(0, 8);
}

/** `#k7qm-2xpa`, `K7QM 2XPA` → `K7QM2XPA`; `null` se non è un codice. */
export function normalizeCode(s: string): string | null {
  const c = s.replace(/[^A-Za-z0-9]/g, '').toUpperCase();
  return /^[A-Z2-7]{8}$/.test(c) ? c : null;
}

export const displayCode = (c: string) => `${c.slice(0, 4)}-${c.slice(4)}`;

export function randomCode(): string {
  return base32(crypto.getRandomValues(new Uint8Array(5)));
}

/** Verifica la richiesta e restituisce l'ID del mittente, o un messaggio d'errore. */
export async function verify(
  headers: Headers,
  method: string,
  path: string,
  body: Uint8Array,
  now: number,
): Promise<{ id: string } | { error: string }> {
  const id = headers.get('x-falco-id');
  const ts = Number(headers.get('x-falco-ts'));
  const sig = headers.get('x-falco-sig');
  if (!id || !sig || !Number.isFinite(ts)) return { error: 'intestazioni di autenticazione mancanti' };
  if (Math.abs(now - ts) > MAX_SKEW_S) return { error: "orario della richiesta fuori finestra: controlla l'ora del PC" };
  const raw = fromBase32(id);
  const signature = fromBase64Url(sig);
  if (!raw || raw.length !== 32 || !signature) return { error: 'firma non valida' };
  const message = new TextEncoder().encode(`${method.toUpperCase()}\n${path}\n${ts}\n${hex(await sha256(body))}`);
  try {
    const key = await crypto.subtle.importKey('raw', raw, { name: 'Ed25519' }, false, ['verify']);
    if (await crypto.subtle.verify({ name: 'Ed25519' }, key, signature, message)) return { id };
  } catch {
    // Chiave non valida
  }
  return { error: 'firma non valida' };
}

/** Come `validate_name` del core: 2–24 caratteri, niente caratteri di controllo. */
export function validName(raw: unknown): string | null {
  if (typeof raw !== 'string') return null;
  const name = raw.trim();
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f-\u009f]/.test(name)) return null;
  const n = [...name].length;
  return n >= 2 && n <= 24 ? name : null;
}
