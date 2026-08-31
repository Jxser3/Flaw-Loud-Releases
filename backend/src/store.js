import pg from 'pg';

const { Pool } = pg;

export class PostgresStore {
  constructor(connectionString) {
    if (!connectionString) throw new Error('DATABASE_URL is required');
    const url = new URL(connectionString);
    if (!['postgres:', 'postgresql:'].includes(url.protocol)) throw new Error('DATABASE_URL must use postgres:// or postgresql://');
    // Respect DATABASE_URL exactly. Render internal URLs need no TLS; its
    // external URL includes sslmode=require, which node-postgres honors.
    this.pool = new Pool({ connectionString });
  }
  async init() {
    await this.pool.query(`
      CREATE TABLE IF NOT EXISTS users (
        id BIGSERIAL PRIMARY KEY,
        username VARCHAR(64) NOT NULL,
        username_normalized VARCHAR(64) UNIQUE NOT NULL,
        password_hash TEXT NOT NULL,
        role VARCHAR(16) NOT NULL CHECK (role IN ('user','admin')) DEFAULT 'user',
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_login_at TIMESTAMPTZ,
        last_seen_at TIMESTAMPTZ
      );
      CREATE TABLE IF NOT EXISTS sessions (
        id BIGSERIAL PRIMARY KEY,
        user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        token_hash CHAR(64) UNIQUE NOT NULL,
        expires_at TIMESTAMPTZ NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
      );
      CREATE INDEX IF NOT EXISTS sessions_user_id_idx ON sessions(user_id);
      CREATE INDEX IF NOT EXISTS sessions_expires_at_idx ON sessions(expires_at);
    `);
  }
  async createUser({ username, passwordHash, role = 'user' }) {
    const q = await this.pool.query(
      `INSERT INTO users(username, username_normalized, password_hash, role)
       VALUES ($1, LOWER($1), $2, $3)
       RETURNING id, username, role, created_at AS "createdAt", last_login_at AS "lastLoginAt", last_seen_at AS "lastSeenAt"`,
      [username, passwordHash, role]
    );
    return q.rows[0];
  }
  async findUserByUsername(username) {
    const q = await this.pool.query('SELECT * FROM users WHERE username_normalized = LOWER($1)', [username]);
    return q.rows[0] ?? null;
  }
  async createSession({ userId, tokenHash, expiresAt }) {
    await this.pool.query('DELETE FROM sessions WHERE expires_at <= NOW()');
    await this.pool.query('INSERT INTO sessions(user_id, token_hash, expires_at) VALUES ($1,$2,$3)', [userId, tokenHash, expiresAt]);
  }
  async getSession(tokenHash) {
    const q = await this.pool.query(`
      SELECT s.id AS session_id, s.expires_at, u.*
      FROM sessions s JOIN users u ON u.id=s.user_id
      WHERE s.token_hash=$1 AND s.expires_at > NOW()`, [tokenHash]);
    return q.rows[0] ?? null;
  }
  async touchSession(sessionId, userId) {
    await this.pool.query('UPDATE sessions SET last_seen_at=NOW() WHERE id=$1', [sessionId]);
    await this.pool.query('UPDATE users SET last_seen_at=NOW() WHERE id=$1', [userId]);
  }
  async markLogin(userId) { await this.pool.query('UPDATE users SET last_login_at=NOW(), last_seen_at=NOW() WHERE id=$1', [userId]); }
  async deleteSession(tokenHash) { await this.pool.query('DELETE FROM sessions WHERE token_hash=$1', [tokenHash]); }
  async listUsers() {
    const q = await this.pool.query(`SELECT id, username, role, created_at AS "createdAt", last_login_at AS "lastLoginAt", last_seen_at AS "lastSeenAt",
      CASE WHEN last_seen_at > NOW() - INTERVAL '5 minutes' THEN 'online' ELSE 'offline' END AS status FROM users ORDER BY created_at DESC`);
    return q.rows;
  }
  async getUser(id) {
    const q = await this.pool.query(`SELECT id, username, role, created_at AS "createdAt", last_login_at AS "lastLoginAt", last_seen_at AS "lastSeenAt",
      CASE WHEN last_seen_at > NOW() - INTERVAL '5 minutes' THEN 'online' ELSE 'offline' END AS status FROM users WHERE id=$1`, [id]);
    return q.rows[0] ?? null;
  }
  async stats() {
    const q = await this.pool.query(`SELECT COUNT(*)::int AS "totalUsers", COUNT(*) FILTER (WHERE role='admin')::int AS admins,
      COUNT(*) FILTER (WHERE last_seen_at > NOW() - INTERVAL '5 minutes')::int AS "onlineUsers" FROM users`);
    return q.rows[0];
  }
  async close() { await this.pool.end(); }
}
