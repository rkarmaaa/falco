// Schema di D1, creato al primo uso di ogni istanza (idempotente): nessuna migrazione da lanciare.
const SCHEMA = [
  `CREATE TABLE IF NOT EXISTS users ( id TEXT PRIMARY KEY, code TEXT NOT NULL UNIQUE, name TEXT NOT NULL, created_at INTEGER NOT NULL, seen_at INTEGER NOT NULL )`,
  `CREATE TABLE IF NOT EXISTS requests ( id TEXT PRIMARY KEY, from_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, to_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, invite TEXT NOT NULL, created_at INTEGER NOT NULL, UNIQUE (from_id, to_id) )`,
  `CREATE INDEX IF NOT EXISTS requests_to ON requests(to_id)`,
  `CREATE TABLE IF NOT EXISTS friendships ( a TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, b TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, created_at INTEGER NOT NULL, PRIMARY KEY (a, b), CHECK (a < b) )`,
  `CREATE TABLE IF NOT EXISTS links ( code TEXT PRIMARY KEY, host_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, invite TEXT NOT NULL, expires_at INTEGER NOT NULL )`,
];

let ready: Promise<unknown> | null = null;

export function ensureSchema(db: D1Database): Promise<unknown> {
  ready ??= db.batch(SCHEMA.map((s) => db.prepare(s))).catch((e) => {
    ready = null;
    throw e;
  });
  return ready;
}
