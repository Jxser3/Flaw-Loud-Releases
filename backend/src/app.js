import path from 'node:path';
import { fileURLToPath } from 'node:url';
import express from 'express';
import cookieParser from 'cookie-parser';
import helmet from 'helmet';
import { rateLimit } from 'express-rate-limit';
import { hashPassword, verifyPassword, newSessionToken, hashToken } from './security.js';

const VERSION = '1.1.2';
const COOKIE = 'flaw_session';
const publicUser = (u) => ({ id: String(u.id), username: u.username, role: u.role, createdAt: u.createdAt ?? u.created_at, lastLoginAt: u.lastLoginAt ?? u.last_login_at, lastSeenAt: u.lastSeenAt ?? u.last_seen_at, status: u.status });
const validUsername = (v) => typeof v === 'string' && /^[A-Za-z0-9_.-]{3,32}$/.test(v);
const validPassword = (v) => typeof v === 'string' && v.length >= 10 && v.length <= 128;

export async function bootstrapAdmin(store, env = process.env) {
  const username = env.FLAW_ADMIN_USERNAME;
  const password = env.FLAW_ADMIN_PASSWORD;
  if (!username && !password) return false;
  if (!validUsername(username) || !validPassword(password)) throw new Error('FLAW_ADMIN_USERNAME or FLAW_ADMIN_PASSWORD is invalid (password must be at least 10 characters)');
  if (await store.findUserByUsername(username)) return false;
  try { await store.createUser({ username, passwordHash: await hashPassword(password), role: 'admin' }); return true; }
  catch (error) { if (error?.code === '23505') return false; throw error; }
}

export function createApp({ store, env = process.env, frontendDir } = {}) {
  if (!store) throw new Error('store is required');
  const app = express();
  if (env.TRUST_PROXY) app.set('trust proxy', Number(env.TRUST_PROXY) || 1);
  app.disable('x-powered-by');
  app.use(helmet({ contentSecurityPolicy: false }));
  app.use(express.json({ limit: '32kb' }));
  app.use(cookieParser());

  const authLimit = rateLimit({ windowMs: 15 * 60_000, limit: env.NODE_ENV === 'test' ? 1000 : 20, standardHeaders: 'draft-8', legacyHeaders: false });
  const cookieOptions = (maxAge) => ({ httpOnly: true, secure: env.NODE_ENV === 'production', sameSite: 'lax', path: '/', maxAge });
  const sessionFor = async (user, rememberMe) => {
    const token = newSessionToken();
    const maxAge = rememberMe ? Number(env.REMEMBER_ME_DAYS || 30) * 86_400_000 : Number(env.SESSION_HOURS || 12) * 3_600_000;
    await store.createSession({ userId: user.id, tokenHash: hashToken(token), expiresAt: new Date(Date.now() + maxAge) });
    return { token, maxAge };
  };
  const authenticate = async (req, res, next) => {
    try {
      const token = req.cookies[COOKIE];
      if (!token) return res.status(401).json({ error: 'Authentication required' });
      const session = await store.getSession(hashToken(token));
      if (!session) return res.status(401).json({ error: 'Session expired or invalid' });
      req.user = session;
      await store.touchSession(session.session_id, session.id);
      next();
    } catch (error) { next(error); }
  };
  const adminOnly = (req, res, next) => req.user.role === 'admin' ? next() : res.status(403).json({ error: 'Admin access required' });

  // API routes always precede static files and the SPA fallback.
  app.get('/api/health', (_req, res) => res.status(200).json({ ok: true, service: 'flaw-loud-api', status: 'healthy', version: VERSION }));
  app.post('/api/auth/register', authLimit, async (req, res, next) => {
    try {
      const { username, password } = req.body ?? {};
      if (!validUsername(username) || !validPassword(password)) return res.status(400).json({ error: 'Username must be 3-32 safe characters and password at least 10 characters' });
      const user = await store.createUser({ username, passwordHash: await hashPassword(password), role: 'user' });
      const session = await sessionFor(user, Boolean(req.body.rememberMe));
      res.cookie(COOKIE, session.token, cookieOptions(session.maxAge)).status(201).json({ user: publicUser(user) });
    } catch (error) { if (error?.code === '23505') return res.status(409).json({ error: 'Username already exists' }); next(error); }
  });
  app.post('/api/auth/login', authLimit, async (req, res, next) => {
    try {
      const user = validUsername(req.body?.username) ? await store.findUserByUsername(req.body.username) : null;
      if (!user || !validPassword(req.body?.password) || !await verifyPassword(req.body.password, user.password_hash)) return res.status(401).json({ error: 'Invalid username or password' });
      await store.markLogin(user.id);
      const session = await sessionFor(user, Boolean(req.body.rememberMe));
      res.cookie(COOKIE, session.token, cookieOptions(session.maxAge)).json({ user: publicUser(user) });
    } catch (error) { next(error); }
  });
  app.post('/api/auth/logout', async (req, res, next) => {
    try { if (req.cookies[COOKIE]) await store.deleteSession(hashToken(req.cookies[COOKIE])); res.clearCookie(COOKIE, { httpOnly: true, secure: env.NODE_ENV === 'production', sameSite: 'lax', path: '/' }).status(204).end(); }
    catch (error) { next(error); }
  });
  app.get('/api/auth/me', authenticate, (req, res) => res.json({ user: publicUser(req.user) }));
  app.get('/api/admin/users', authenticate, adminOnly, async (_req, res, next) => { try { res.json({ users: (await store.listUsers()).map(publicUser) }); } catch (e) { next(e); } });
  app.get('/api/admin/users/:id', authenticate, adminOnly, async (req, res, next) => { try { const u = await store.getUser(req.params.id); u ? res.json({ user: publicUser(u) }) : res.status(404).json({ error: 'User not found' }); } catch (e) { next(e); } });
  app.get('/api/admin/stats', authenticate, adminOnly, async (_req, res, next) => { try { res.json(await store.stats()); } catch (e) { next(e); } });
  app.use('/api', (_req, res) => res.status(404).json({ error: 'API route not found' }));

  const resolvedFrontend = frontendDir ?? path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../dist');
  app.use(express.static(resolvedFrontend, { index: false }));
  app.get(/.*/, (_req, res) => res.sendFile(path.join(resolvedFrontend, 'index.html')));
  app.use((error, _req, res, _next) => { console.error(error); res.status(500).json({ error: 'Internal server error' }); });
  return app;
}
