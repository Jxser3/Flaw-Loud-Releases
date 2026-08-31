use anyhow::{anyhow, Context};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, Response, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post, put},
    Router,
};
use base64::{engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD}, Engine as _};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

const SESSION_TTL_SECS: u64 = 30 * 24 * 3600;
const MAX_ATTACHMENTS: usize = 3;
const MAX_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
const MAX_JSON_BODY: usize = 110 * 1024 * 1024;

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn parse_bool_env(name: &str, default: bool) -> bool {
    env::var(name).ok().map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1"|"true"|"yes"|"on")).unwrap_or(default)
}

#[derive(Clone)]
struct Config {
    owner_username: String,
    registration_enabled: bool,
    data_dir: PathBuf,
    latest_version: String,
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    cfg: Config,
    attachment_dir: PathBuf,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad(message: impl Into<String>) -> Self { Self { status: StatusCode::BAD_REQUEST, message: message.into() } }
    fn unauthorized(message: impl Into<String>) -> Self { Self { status: StatusCode::UNAUTHORIZED, message: message.into() } }
    fn forbidden(message: impl Into<String>) -> Self { Self { status: StatusCode::FORBIDDEN, message: message.into() } }
    fn not_found(message: impl Into<String>) -> Self { Self { status: StatusCode::NOT_FOUND, message: message.into() } }
    fn conflict(message: impl Into<String>) -> Self { Self { status: StatusCode::CONFLICT, message: message.into() } }
    fn internal(message: impl Into<String>) -> Self { Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: message.into() } }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(serde_json::json!({"error": self.message}))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

fn db_lock(state: &AppState) -> ApiResult<MutexGuard<'_, Connection>> {
    state.db.lock().map_err(|_| ApiError::internal("Central Platform database lock failed."))
}

fn map_db(e: rusqlite::Error) -> ApiError {
    tracing::error!(error=%e, "database error");
    ApiError::internal("Central Platform database operation failed.")
}

fn valid_username(v: &str) -> bool {
    let n = v.chars().count();
    (3..=32).contains(&n) && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_'|'-'|'.'))
}

fn validate_password(v: &str) -> ApiResult<()> {
    if v.len() < 8 || v.len() > 128 { return Err(ApiError::bad("Password must be between 8 and 128 characters.")); }
    Ok(())
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Argon2 password hashing failed: {e}"))?;
    Ok(hashed.to_string())
}

fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash).ok().map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()).unwrap_or(false)
}

fn token_hash(token: &str) -> String { hex::encode(Sha256::digest(token.as_bytes())) }

fn new_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn valid_hwid(h: &str) -> bool { h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()) }
fn hwid_hint(h: &str) -> String { if h.len() <= 16 { h.into() } else { format!("{}…{}", &h[..8], &h[h.len()-6..]) } }

fn parse_version(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split('-').next().unwrap_or(v).split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect()
}
fn version_lt(a: &str, b: &str) -> bool {
    let mut av=parse_version(a); let mut bv=parse_version(b); let n=av.len().max(bv.len()); av.resize(n,0); bv.resize(n,0); av<bv
}

fn audit(conn: &Connection, actor: &str, action: &str, target: &str, detail: &str) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO audit(actor,action,target,detail,created_at) VALUES(?1,?2,?3,?4,?5)", params![actor,action,target,detail,now() as i64])?;
    conn.execute("DELETE FROM audit WHERE id NOT IN (SELECT id FROM audit ORDER BY id DESC LIMIT 2000)", [])?;
    Ok(())
}

fn init_db(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(r#"
PRAGMA foreign_keys=ON;
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS users(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 username TEXT NOT NULL COLLATE NOCASE UNIQUE,
 password_hash TEXT NOT NULL,
 role TEXT NOT NULL CHECK(role IN ('Owner','Moderator','User')),
 status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','blocked','banned')),
 suspended_until INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL,
 last_login INTEGER NOT NULL DEFAULT 0,
 failed_login_count INTEGER NOT NULL DEFAULT 0,
 locked_until INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS one_owner_only ON users(role) WHERE role='Owner';
CREATE TABLE IF NOT EXISTS sessions(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 token_hash TEXT NOT NULL UNIQUE,
 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 created_at INTEGER NOT NULL,
 last_seen INTEGER NOT NULL,
 hwid_hash TEXT NOT NULL,
 app_version TEXT NOT NULL,
 expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_user_idx ON sessions(user_id);
CREATE TABLE IF NOT EXISTS hardware(
 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 hwid_hash TEXT NOT NULL,
 first_seen INTEGER NOT NULL,
 last_seen INTEGER NOT NULL,
 PRIMARY KEY(user_id,hwid_hash)
);
CREATE TABLE IF NOT EXISTS hardware_bans(
 hwid_hash TEXT PRIMARY KEY,
 reason TEXT NOT NULL,
 banned_by TEXT NOT NULL,
 created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS announcements(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 title TEXT NOT NULL,
 message TEXT NOT NULL,
 priority TEXT NOT NULL,
 version TEXT NOT NULL,
 created_by TEXT NOT NULL,
 created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS announcement_reads(
 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 announcement_id INTEGER NOT NULL REFERENCES announcements(id) ON DELETE CASCADE,
 PRIMARY KEY(user_id,announcement_id)
);
CREATE TABLE IF NOT EXISTS reports(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 username TEXT NOT NULL,
 category TEXT NOT NULL,
 message TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'New',
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS report_attachments(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 report_id INTEGER NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
 name TEXT NOT NULL,
 mime TEXT NOT NULL,
 storage_path TEXT NOT NULL,
 size_bytes INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS audit(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 actor TEXT NOT NULL,
 action TEXT NOT NULL,
 target TEXT NOT NULL,
 detail TEXT NOT NULL,
 created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS release_policy(
 id INTEGER PRIMARY CHECK(id=1),
 latest_version TEXT NOT NULL,
 minimum_supported_version TEXT NOT NULL,
 grace_until INTEGER NOT NULL,
 notes TEXT NOT NULL,
 force_after_grace INTEGER NOT NULL
);
"#)?
    Ok(())
}

fn seed_release_policy(conn: &Connection, latest: &str) -> anyhow::Result<()> {
    conn.execute("INSERT OR IGNORE INTO release_policy(id,latest_version,minimum_supported_version,grace_until,notes,force_after_grace) VALUES(1,?1,'0.0.0',0,'Central Platform ready',1)", params![latest])?
    Ok(())
}

fn seed_owner(conn: &Connection, cfg: &Config) -> anyhow::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE role='Owner'", [], |r| r.get(0))?
    if count > 1 { return Err(anyhow!("database contains more than one Owner; refusing to start")); }
    if count == 1 { return Ok(()); }
    let password = env::var("FLAW_OWNER_PASSWORD").context("FLAW_OWNER_PASSWORD must be set the first time the Central Platform starts")?;
    if password.len() < 12 { return Err(anyhow!("FLAW_OWNER_PASSWORD must be at least 12 characters for the Owner account")); }
    if !valid_username(&cfg.owner_username) { return Err(anyhow!("FLAW_OWNER_USERNAME must be 3-32 characters: letters, numbers, dot, dash or underscore")); }
    let hash = hash_password(&password)?
    conn.execute("INSERT INTO users(username,password_hash,role,status,suspended_until,created_at,last_login) VALUES(?1,?2,'Owner','active',0,?3,0)", params![&cfg.owner_username,hash,now() as i64])?;
    audit(conn, "SYSTEM", "OWNER_PROVISIONED", &cfg.owner_username, "Single Owner provisioned from server environment")?
    Ok(())
}
