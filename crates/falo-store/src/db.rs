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
    pub fn consume_invite(&self, token_hash: &str, now: i64) -> Result<Option<String>, DbError> {
        self.conn
            .query_row(
                "UPDATE invites SET uses_left = uses_left - 1
                 WHERE token_hash = ?1 AND uses_left > 0 AND revoked = 0 AND expires_at > ?2
                 RETURNING role",
                params![token_hash, now],
                |r| r.get(0),
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
    fn single_use_invite_first_wins() {
        let db = Db::open_in_memory().unwrap();
        db.insert_invite("h", "giocatore", 0, 100, 1).unwrap();
        assert_eq!(db.consume_invite("h", 10).unwrap().as_deref(), Some("giocatore"));
        assert_eq!(db.consume_invite("h", 10).unwrap(), None);
        db.insert_invite("old", "giocatore", 0, 100, 1).unwrap();
        assert_eq!(db.consume_invite("old", 100).unwrap(), None, "scaduto");
    }
}
