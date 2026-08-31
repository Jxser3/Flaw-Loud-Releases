import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { PostgresStore } from './store.js';
import { bootstrapAdmin, createApp } from './app.js';

export async function startServer({
  env = process.env,
  store = new PostgresStore(env.DATABASE_URL),
  port = Number(env.PORT || 8080),
  host = '0.0.0.0',
  frontendDir,
} = {}) {
  await store.init();
  await bootstrapAdmin(store, env);
  const app = createApp({ store, env, frontendDir });
  const server = await new Promise((resolve, reject) => {
    const instance = app.listen(port, host, () => resolve(instance));
    instance.once('error', reject);
  });
  return { server, store };
}

const isEntryPoint = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isEntryPoint) {
  const port = Number(process.env.PORT || 8080);
  const host = '0.0.0.0';
  const { server, store } = await startServer({ port, host });
  console.log(`Flaw Loud API listening on ${host}:${port}`);

  const shutdown = () => server.close(async () => { await store.close(); process.exit(0); });
  process.on('SIGTERM', shutdown);
  process.on('SIGINT', shutdown);
}
