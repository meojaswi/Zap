// Anonymous daily check-in from the Zap app, used only to count active users.
//
// The app sends a random install ID (not tied to the PC or the person) and its version, at most
// once a day. IDs go into HyperLogLog counters, which keep an approximate count but not the IDs
// themselves; nothing else about the request is stored.
import { day, setup, sql } from "./_db.js";

const ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const VERSION = /^\d{1,3}\.\d{1,3}\.\d{1,3}$/;

export async function POST(request: Request): Promise<Response> {
  if (!sql) return new Response("Not configured", { status: 503 });

  let body: { id?: unknown; version?: unknown };
  try {
    body = await request.json();
  } catch {
    return new Response("Bad request", { status: 400 });
  }
  const { id, version } = body;
  if (typeof id !== "string" || !ID.test(id) || typeof version !== "string" || !VERSION.test(version)) {
    return new Response("Bad request", { status: 400 });
  }

  await setup();
  const today = day();
  await sql`
    INSERT INTO zap_active (day, users) VALUES (${today}, hll_add(hll_empty(), hll_hash_text(${id})))
    ON CONFLICT (day) DO UPDATE SET users = hll_add(zap_active.users, hll_hash_text(${id}))`;
  await sql`
    INSERT INTO zap_version_active (day, version, users) VALUES (${today}, ${version}, hll_add(hll_empty(), hll_hash_text(${id})))
    ON CONFLICT (day, version) DO UPDATE SET users = hll_add(zap_version_active.users, hll_hash_text(${id}))`;
  await sql`
    INSERT INTO zap_installs (id, users) VALUES (1, hll_add(hll_empty(), hll_hash_text(${id})))
    ON CONFLICT (id) DO UPDATE SET users = hll_add(zap_installs.users, hll_hash_text(${id}))`;
  return new Response(null, { status: 204 });
}
