// Rewrites latest.json so each update downloads from the release's plain download link
// (github.com/.../releases/download/...) instead of api.github.com, which allows only 60
// unauthenticated requests an hour per network. The signatures are untouched: they cover the
// file, not where it's downloaded from.
//
// Usage: node direct-download-links.mjs <latest.json> <release.json from the GitHub API>
import { readFileSync, writeFileSync } from "node:fs";

const [manifestPath, releasePath] = process.argv.slice(2);
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const release = JSON.parse(readFileSync(releasePath, "utf8"));

for (const [platform, entry] of Object.entries(manifest.platforms)) {
  const asset = release.assets.find((a) => a.url === entry.url || a.browser_download_url === entry.url);
  if (!asset) {
    throw new Error(`No asset in ${release.tag_name} matches ${platform}: ${entry.url}`);
  }
  entry.url = asset.browser_download_url;
  console.log(`${platform}: ${entry.url}`);
}

writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
