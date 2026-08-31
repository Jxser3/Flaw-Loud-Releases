import { PostgresStore } from './store.js';
import { bootstrapAdmin, createApp } from './app.js';

const port = Number(process.env.PORT || 8080);
const host = '0.0.0.0';
const store = new PostgresStore(process.env.DATABASE_URL);

await store.init();
await bootstrapAdmin(store);
const server = createApp({ store }).listen(port, host, () => console.log(`Flaw Loud API listening on ${host}:${port}`));

const shutdown = () => server.close(async () => { await store.close(); process.exit(0); });
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
