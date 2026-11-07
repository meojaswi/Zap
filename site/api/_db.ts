// Neon Postgres for the usage endpoints. The Vercel Marketplace integration sets DATABASE_URL.
//
// Counts live in HyperLogLog columns (the hll extension): they keep an approximate number of
// distinct IDs, about 1% error, but not the IDs themselves.
import { neon } from "@neondatabase/serverless";

const url = process.env.DATABASE_URL ?? process.env.POSTGRES_URL;

export const sql = url ? neon(url) : null;

let ready: Promise<unknown> | null = null;

/** Creates the extension and tables on first use in each function instance. */
export function setup(): Promise<unknown> {
  if (!sql) throw new Error("DATABASE_URL is not set");
  ready ??= (async () => {
    await sql`CREATE EXTENSION IF NOT EXISTS hll`;
    await sql`CREATE TABLE IF NOT EXISTS zap_active (day date PRIMARY KEY, users hll NOT NULL)`;
    await sql`CREATE TABLE IF NOT EXISTS zap_version_active (day date, version text, users hll NOT NULL, PRIMARY KEY (day, version))`;
    await sql`CREATE TABLE IF NOT EXISTS zap_installs (id int PRIMARY KEY, users hll NOT NULL)`;
  })().catch((e) => {
    ready = null;
    throw e;
  });
  return ready;
}

/** UTC date as YYYY-MM-DD, `daysAgo` days back. */
export function day(daysAgo = 0): string {
  return new Date(Date.now() - daysAgo * 86_400_000).toISOString().slice(0, 10);
}
