# Flaw Loud Railway backend

Set Railway's Root Directory to `/backend`. Railway automatically detects `Dockerfile`; no Build Command or Start Command override is needed. The image builds the self-contained Vite app in `frontend/`, and `npm start` launches the API server, which serves the resulting `dist`.

Required environment variables are documented in `.env.example`. Add a Railway PostgreSQL service and expose its `DATABASE_URL` to this service. The initial admin is created once from `FLAW_ADMIN_USERNAME` and `FLAW_ADMIN_PASSWORD`; changing those variables does not overwrite an existing account.

API routes are registered before static assets. Any unmatched `/api/*` request returns a JSON 404 and never reaches the SPA fallback.
