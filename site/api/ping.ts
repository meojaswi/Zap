// Anonymous daily check-in from the Zap app, used only to count active users.
//
// The app sends a random install ID (not tied to the PC or the person) and its version, at most
// once a day. IDs go into HyperLogLog counters, which keep an approximate count but not the IDs
// themselves; nothing else about the request is stored.
import { day, pipeline, redisConfigured } from "./_redis.js";

const ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const VERSION = /^\d{1,3}\.\d{1,3}\.\d{1,3}$/;
const KEEP_DAYS = 400;

export async function POST(request: Request): Promise<Response> {
  if (!redisConfigured()) return new Response("Not configured", { status: 503 });

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

  const today = `zap:active:${day()}`;
  const onVersion = `zap:version:${version}:${day()}`;
  await pipeline([
    ["PFADD", today, id],
    ["EXPIRE", today, KEEP_DAYS * 86_400],
    ["PFADD", onVersion, id],
    ["EXPIRE", onVersion, KEEP_DAYS * 86_400],
    ["PFADD", "zap:installs", id],
  ]);
  return new Response(null, { status: 204 });
}
