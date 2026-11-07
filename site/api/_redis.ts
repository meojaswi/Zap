// Minimal Upstash Redis REST client for the usage endpoints. The Vercel Marketplace integration
// sets KV_REST_API_URL / KV_REST_API_TOKEN; a direct Upstash setup uses the UPSTASH_ names.
const url = process.env.KV_REST_API_URL ?? process.env.UPSTASH_REDIS_REST_URL;
const token = process.env.KV_REST_API_TOKEN ?? process.env.UPSTASH_REDIS_REST_TOKEN;

export function redisConfigured(): boolean {
  return Boolean(url && token);
}

/** Runs several Redis commands in one request and returns their results in order. */
export async function pipeline(commands: (string | number)[][]): Promise<unknown[]> {
  const res = await fetch(`${url}/pipeline`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
    body: JSON.stringify(commands),
  });
  if (!res.ok) throw new Error(`Redis responded ${res.status}`);
  const replies = (await res.json()) as { result?: unknown; error?: string }[];
  return replies.map((r) => {
    if (r.error) throw new Error(r.error);
    return r.result;
  });
}

/** UTC date as YYYY-MM-DD, `daysAgo` days back. */
export function day(daysAgo = 0): string {
  return new Date(Date.now() - daysAgo * 86_400_000).toISOString().slice(0, 10);
}
