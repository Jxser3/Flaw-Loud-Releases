import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { PostgresStore } from '../src/store.js';
import { startServer } from '../src/server.js';

test('Postgres supports Render internal and external DATABASE_URL modes', async () => {
  const internal = new pg.Client({ connectionString: 'postgresql://user:pass@internal/db' });
  const external = new pg.Client({ connectionString: 'postgresql://user:pass@external/db?sslmode=require' });
  assert.equal(internal.connectionParameters.ssl, false);
  assert.deepEqual(external.connectionParameters.ssl, {});
  assert.throws(() => new PostgresStore('mysql://user:pass@host/db'), /postgres/);
});

test('production server starts and serves API health', async () => {
  const store = { init: async () => {}, close: async () => {} };
  const frontendDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../dist');
  const { server } = await startServer({ env: { NODE_ENV: 'production' }, store, port: 0, host: '127.0.0.1', frontendDir });
  try {
    const address = server.address();
    const response = await fetch(`http://127.0.0.1:${address.port}/api/health`);
    assert.equal(response.status, 200);
    assert.match(response.headers.get('content-type'), /application\/json/);
    assert.equal((await response.json()).status, 'healthy');
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
