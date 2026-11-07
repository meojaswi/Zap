// Active-user counts for the makers: GET /api/stats?token=<STATS_TOKEN>.
// Counts are approximate (about 1% error) because the IDs themselves are never stored.
import { day, pipeline, redisConfigured } from "./_redis.js";

export async function GET(request: Request): Promise<Response> {
  const token = new URL(request.url).searchParams.get("token");
  if (!process.env.STATS_TOKEN || token !== process.env.STATS_TOKEN) {
    return new Response("Not found", { status: 404 });
  }
  if (!redisConfigured()) return new Response("Not configured", { status: 503 });

  const days = Array.from({ length: 30 }, (_, i) => `zap:active:${day(i)}`);
  const results = await pipeline([
    ...days.map((key) => ["PFCOUNT", key]),
    ["PFCOUNT", ...days.slice(0, 7)],
    ["PFCOUNT", ...days],
    ["PFCOUNT", "zap:installs"],
  ]);
  const daily = days.map((key, i) => ({ date: key.slice("zap:active:".length), active: results[i] }));
  return Response.json({
    activeToday: results[0],
    activeLast7Days: results[30],
    activeLast30Days: results[31],
    installsEver: results[32],
    daily,
  });
}
