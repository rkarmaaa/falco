//! SQLite locale (§5, §6, §7). Migrazioni numerate con `PRAGMA user_version`.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub use rusqlite::Error as DbError;

const MIGRATIONS: &[&str] = &[
    // 1 · amici, dispositivi, registro sessioni (§5)
    r"
    CREATE TABLE friends (
      id TEXT PRIMARY KEY, tag TEXT, name TEXT, avatar_hash TEXT, accent TEXT,
      state TEXT CHECK (state IN ('pending','friend','blocked')),
      favorite INTEGER DEFAULT 0, added_at INTEGER, last_seen INTEGER
    );
    CREATE TABLE friend_devices (
      friend_id TEXT REFERENCES friends(id) ON DELETE CASCADE,
      ts_node_id TEXT PRIMARY KEY, ts_ip TEXT, hostname TEXT, os TEXT,
      can_host INTEGER, shared_in INTEGER, shared_out INTEGER
    );
    CREATE TABLE session_log (
      id INTEGER PRIMARY KEY, friend_id TEXT, role TEXT, started_at INTEGER,
      ended_at INTEGER, avg_rtt_ms REAL, path TEXT
    );
    ",
    // 2 · permessi per amico (§7) e inviti con solo l'hash del token (§6)
    r"
    CREATE TABLE friend_permissions (
      friend_id TEXT PRIMARY KEY REFERENCES friends(id) ON DELETE CASCADE,
      role TEXT NOT NULL, overrides TEXT NOT NULL DEFAULT '{}',
      cert_fingerprint TEXT, updated_at INTEGER
    );
    CREATE TABLE invites (
      token_hash TEXT PRIMARY KEY, role TEXT NOT NULL, created_at INTEGER NOT NULL,
      expires_at INTEGER NOT NULL, uses_left INTEGER NOT NULL, needs_approval INTEGER NOT NULL,
      ts_invite_id TEXT, revoked INTEGER NOT NULL DEFAULT 0
    );
    ",
    // 3 · profilo firmato degli amici, avatar per hash, valori locali (M2)
    r"
    ALTER TABLE friends ADD COLUMN profile TEXT;
    CREATE TABLE avatars (hash TEXT PRIMARY KEY, data BLOB NOT NULL);
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    ",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FriendState {
    Pending,
    Friend,
    Blocked,
}

impl FriendState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Friend => "friend",
            Self::Blocked => "blocked",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "friend" => Self::Friend,
            "blocked" => Self::Blocked,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRow {
    pub id: String,
    pub tag: String,
    pub name: String,
    pub avatar_hash: Option<String>,
    pub accent: String,
    pub state: FriendState,
    pub favorite: bool,
    pub added_at: i64,
    pub last_seen: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteRow {
    pub token_hash: String,
    pub role: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub uses_left: u32,
    pub needs_approval: bool,
    pub ts_invite_id: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRow {
    pub friend_id: String,
    pub ts_node_id: String,
    pub ts_ip: String,
    pub hostname: String,
    pub os: String,
    pub can_host: bool,
    pub shared_in: bool,
    pub shared_out: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub friend_id: String,
    pub role: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub avg_rtt_ms: Option<f64>,
    /// `direct` oppure `derp`.
    pub path: String,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, DbError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, DbError> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let current: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i + 1)?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> Result<usize, DbError> {
        self.conn.pragma_query_value(None, "user_version", |r| r.get(0))
    }

    pub fn upsert_friend(&self, f: &FriendRow) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO friends (id, tag, name, avatar_hash, accent, state, favorite, added_at, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET tag=?2, name=?3, avatar_hash=?4, accent=?5,
               state=?6, favorite=?7, last_seen=?9",
            params![
                f.id, f.tag, f.name, f.avatar_hash, f.accent, f.state.as_str(),
                f.favorite, f.added_at, f.last_seen
            ],
        )?;
        Ok(())
    }

    /// Amici visibili: i bloccati restano solo nelle impostazioni.
    pub fn friends(&self) -> Result<Vec<FriendRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tag, name, avatar_hash, accent, state, favorite, added_at, last_seen
             FROM friends WHERE state != 'blocked' ORDER BY favorite DESC, name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(FriendRow {
                id: r.get(0)?,
                tag: r.get(1)?,
                name: r.get(2)?,
                avatar_hash: r.get(3)?,
                accent: r.get(4)?,
                state: FriendState::parse(&r.get::<_, String>(5)?),
                favorite: r.get(6)?,
                added_at: r.get(7)?,
                last_seen: r.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn friend_count(&self) -> Result<usize, DbError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM friends WHERE state != 'blocked'", [], |r| r.get(0))
    }

    pub fn friend(&self, id: &str) -> Result<Option<FriendRow>, DbError> {
        Ok(self.friends()?.into_iter().find(|f| f.id == id))
    }

    /// Stato grezzo di un ID, anche se bloccato.
    pub fn friend_state(&self, id: &str) -> Result<Option<FriendState>, DbError> {
        self.conn
            .query_row("SELECT state FROM friends WHERE id = ?1", [id], |r| r.get::<_, String>(0))
            .optional()
            .map(|o| o.map(|s| FriendState::parse(&s)))
    }

    pub fn set_friend_state(&self, id: &str, state: FriendState) -> Result<(), DbError> {
        self.conn
            .execute("UPDATE friends SET state = ?2 WHERE id = ?1", params![id, state.as_str()])?;
        Ok(())
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<(), DbError> {
        self.conn
            .execute("UPDATE friends SET favorite = ?2 WHERE id = ?1", params![id, favorite])?;
        Ok(())
    }

    pub fn touch_friend(&self, id: &str, now: i64) -> Result<(), DbError> {
        self.conn.execute("UPDATE friends SET last_seen = ?2 WHERE id = ?1", params![id, now])?;
        Ok(())
    }

    /// Profilo firmato in cache (JSON).
    pub fn friend_profile(&self, id: &str) -> Result<Option<String>, DbError> {
        self.conn
            .query_row("SELECT profile FROM friends WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .map(Option::flatten)
    }

    pub fn set_friend_profile(&self, id: &str, json: &str) -> Result<(), DbError> {
        self.conn.execute("UPDATE friends SET profile = ?2 WHERE id = ?1", params![id, json])?;
        Ok(())
    }

    pub fn upsert_device(&self, d: &DeviceRow) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO friend_devices
               (friend_id, ts_node_id, ts_ip, hostname, os, can_host, shared_in, shared_out)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(ts_node_id) DO UPDATE SET friend_id=?1, ts_ip=?3, hostname=?4, os=?5,
               can_host=?6, shared_in=?7, shared_out=?8",
            params![
                d.friend_id,
                d.ts_node_id,
                d.ts_ip,
                d.hostname,
                d.os,
                d.can_host,
                d.shared_in,
                d.shared_out
            ],
        )?;
        Ok(())
    }

    pub fn devices(&self, friend_id: &str) -> Result<Vec<DeviceRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT friend_id, ts_node_id, ts_ip, hostname, os, can_host, shared_in, shared_out
             FROM friend_devices WHERE friend_id = ?1 ORDER BY hostname",
        )?;
        let rows = stmt.query_map([friend_id], |r| {
            Ok(DeviceRow {
                friend_id: r.get(0)?,
                ts_node_id: r.get(1)?,
                ts_ip: r.get(2)?,
                hostname: r.get(3)?,
                os: r.get(4)?,
                can_host: r.get(5)?,
                shared_in: r.get(6)?,
                shared_out: r.get(7)?,
            })
        })?;
        rows.collect()
    }

    /// Amico a cui appartiene un nodo Tailscale (doppio vincolo del §5).
    pub fn friend_by_node(&self, node_id: &str) -> Result<Option<String>, DbError> {
        self.conn
            .query_row(
                "SELECT friend_id FROM friend_devices WHERE ts_node_id = ?1",
                [node_id],
                |r| r.get(0),
            )
            .optional()
    }

    pub fn avatar(&self, hash: &str) -> Result<Option<Vec<u8>>, DbError> {
        self.conn
            .query_row("SELECT data FROM avatars WHERE hash = ?1", [hash], |r| r.get(0))
            .optional()
    }

    pub fn put_avatar(&self, hash: &str, data: &[u8]) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO avatars (hash, data) VALUES (?1, ?2)",
            params![hash, data],
        )?;
        Ok(())
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>, DbError> {
        self.conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn sessions(&self, friend_id: &str, limit: u32) -> Result<Vec<SessionRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT friend_id, role, started_at, ended_at, avg_rtt_ms, path FROM session_log
             WHERE friend_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![friend_id, limit], |r| {
            Ok(SessionRow {
                friend_id: r.get(0)?,
                role: r.get(1)?,
                started_at: r.get(2)?,
                ended_at: r.get(3)?,
                avg_rtt_ms: r.get(4)?,
                path: r.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn remove_friend(&self, id: &str) -> Result<bool, DbError> {
        Ok(self.conn.execute("DELETE FROM friends WHERE id = ?1", [id])? > 0)
    }

    pub fn log_session(&self, s: &SessionRow) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO session_log (friend_id, role, started_at, ended_at, avg_rtt_ms, path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![s.friend_id, s.role, s.started_at, s.ended_at, s.avg_rtt_ms, s.path],
        )?;
        Ok(())
    }

    pub fn clear_session_log(&self) -> Result<usize, DbError> {
        self.conn.execute("DELETE FROM session_log", [])
    }

    /// Consuma un uso dell'invito in modo atomico: vince il primo (§6).
    /// Restituisce ruolo e se serve l'approvazione dell'host.
    pub fn consume_invite(
        &self,
        token_hash: &str,
        now: i64,
    ) -> Result<Option<(String, bool)>, DbError> {
        self.conn
            .query_row(
                "UPDATE invites SET uses_left = uses_left - 1
                 WHERE token_hash = ?1 AND uses_left > 0 AND revoked = 0 AND expires_at > ?2
                 RETURNING role, needs_approval",
                params![token_hash, now],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
    }

    pub fn invites(&self, now: i64) -> Result<Vec<InviteRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT token_hash, role, created_at, expires_at, uses_left, needs_approval, ts_invite_id,
                    (revoked = 0 AND uses_left > 0 AND expires_at > ?1)
             FROM invites ORDER BY created_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([now], |r| {
            Ok(InviteRow {
                token_hash: r.get(0)?,
                role: r.get(1)?,
                created_at: r.get(2)?,
                expires_at: r.get(3)?,
                uses_left: r.get(4)?,
                needs_approval: r.get(5)?,
                ts_invite_id: r.get(6)?,
                active: r.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn set_invite_ts_id(&self, token_hash: &str, ts_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE invites SET ts_invite_id = ?2 WHERE token_hash = ?1",
            params![token_hash, ts_id],
        )?;
        Ok(())
    }

    /// Revoca: il token non vale più. Restituisce l'ID dell'invito Tailscale da cancellare.
    pub fn revoke_invite(&self, token_hash: &str) -> Result<Option<String>, DbError> {
        self.conn
            .query_row(
                "UPDATE invites SET revoked = 1 WHERE token_hash = ?1 RETURNING ts_invite_id",
                [token_hash],
                |r| r.get(0),
            )
            .optional()
            .map(Option::flatten)
    }

    /// Ruolo, override e client di Sunshine associato a un amico (§7).
    pub fn set_permissions(
        &self,
        friend_id: &str,
        role: &str,
        overrides: &str,
        now: i64,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO friend_permissions (friend_id, role, overrides, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(friend_id) DO UPDATE SET role = ?2, overrides = ?3, updated_at = ?4",
            params![friend_id, role, overrides, now],
        )?;
        Ok(())
    }

    pub fn set_client_cert(&self, friend_id: &str, client_uuid: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE friend_permissions SET cert_fingerprint = ?2 WHERE friend_id = ?1",
            params![friend_id, client_uuid],
        )?;
        Ok(())
    }

    /// (ruolo, override JSON, uuid del client Sunshine)
    pub fn permissions(
        &self,
        friend_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>, DbError> {
        self.conn
            .query_row(
                "SELECT role, overrides, cert_fingerprint FROM friend_permissions WHERE friend_id = ?1",
                [friend_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
    }

    pub fn insert_invite(
        &self,
        token_hash: &str,
        role: &str,
        created_at: i64,
        expires_at: i64,
        uses: u8,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO invites (token_hash, role, created_at, expires_at, uses_left, needs_approval)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![token_hash, role, created_at, expires_at, uses, uses > 1],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn friend(id: &str, name: &str, state: FriendState) -> FriendRow {
        FriendRow {
            id: id.into(),
            tag: "#AAAA-BBBB".into(),
            name: name.into(),
            avatar_hash: None,
            accent: "#FF6A2B".into(),
            state,
            favorite: false,
            added_at: 1,
            last_seen: None,
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("falo.db");
        assert_eq!(Db::open(&path).unwrap().schema_version().unwrap(), MIGRATIONS.len());
        assert_eq!(Db::open(&path).unwrap().schema_version().unwrap(), MIGRATIONS.len());
    }

    #[test]
    fn friends_hide_blocked_and_upsert() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_friend(&friend("a", "Luca", FriendState::Friend)).unwrap();
        db.upsert_friend(&friend("b", "Bea", FriendState::Pending)).unwrap();
        db.upsert_friend(&friend("c", "Troll", FriendState::Blocked)).unwrap();
        db.upsert_friend(&friend("a", "Luca R.", FriendState::Friend)).unwrap();
        let list = db.friends().unwrap();
        assert_eq!(list.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Bea", "Luca R."]);
        assert_eq!(db.friend_count().unwrap(), 2);
        assert!(db.remove_friend("a").unwrap());
    }

    #[test]
    fn devices_avatars_and_meta() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_friend(&friend("a", "Luca", FriendState::Friend)).unwrap();
        let dev = DeviceRow {
            friend_id: "a".into(),
            ts_node_id: "n1".into(),
            ts_ip: "100.64.0.2".into(),
            hostname: "luca-pc".into(),
            os: "windows".into(),
            can_host: true,
            shared_in: true,
            shared_out: false,
        };
        db.upsert_device(&dev).unwrap();
        assert_eq!(db.devices("a").unwrap(), [dev]);
        assert_eq!(db.friend_by_node("n1").unwrap().as_deref(), Some("a"));
        db.put_avatar("h", b"webp").unwrap();
        assert_eq!(db.avatar("h").unwrap().as_deref(), Some(&b"webp"[..]));
        db.set_meta("k", "v").unwrap();
        assert_eq!(db.meta("k").unwrap().as_deref(), Some("v"));
        db.remove_friend("a").unwrap();
        assert!(db.devices("a").unwrap().is_empty(), "cascade");
    }

    #[test]
    fn single_use_invite_first_wins() {
        let db = Db::open_in_memory().unwrap();
        db.insert_invite("h", "giocatore", 0, 100, 1).unwrap();
        assert_eq!(db.consume_invite("h", 10).unwrap(), Some(("giocatore".into(), false)));
        assert!(!db.invites(10).unwrap()[0].active, "uso singolo esaurito");
        assert_eq!(db.consume_invite("h", 10).unwrap(), None);
        db.insert_invite("old", "giocatore", 0, 100, 1).unwrap();
        assert_eq!(db.consume_invite("old", 100).unwrap(), None, "scaduto");
    }
}
