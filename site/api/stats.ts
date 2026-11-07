// Active-user counts for the makers: GET /api/stats?token=<STATS_TOKEN>.
// Counts are approximate (about 1% error) because the IDs themselves are never stored.
import { day, setup, sql } from "./_db.js";

export async function GET(request: Request): Promise<Response> {
  const token = new URL(request.url).searchParams.get("token");
  if (!process.env.STATS_TOKEN || token !== process.env.STATS_TOKEN) {
    return new Response("Not found", { status: 404 });
  }
  if (!sql) return new Response("Not configured", { status: 503 });

  await setup();
  const since30 = day(29);
  const since7 = day(6);
  const [daily, windows, installs, versions] = await Promise.all([
    sql`SELECT to_char(day, 'YYYY-MM-DD') AS date, hll_cardinality(users)::int AS active
        FROM zap_active WHERE day >= ${since30} ORDER BY day DESC`,
    sql`SELECT
          (SELECT coalesce(hll_cardinality(hll_union_agg(users)), 0)::int FROM zap_active WHERE day >= ${since7}) AS last7,
          (SELECT coalesce(hll_cardinality(hll_union_agg(users)), 0)::int FROM zap_active WHERE day >= ${since30}) AS last30`,
    sql`SELECT coalesce(hll_cardinality(users), 0)::int AS total FROM zap_installs WHERE id = 1`,
    sql`SELECT version, hll_cardinality(hll_union_agg(users))::int AS active
        FROM zap_version_active WHERE day >= ${since7} GROUP BY version ORDER BY version DESC`,
  ]);
  return Response.json({
    activeToday: daily.find((d) => d.date === day())?.active ?? 0,
    activeLast7Days: windows[0].last7,
    activeLast30Days: windows[0].last30,
    installsEver: installs[0]?.total ?? 0,
    versionsLast7Days: versions,
    daily,
  });
}
