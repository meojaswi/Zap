import { useEffect, useState } from "react";
import { REPO, RELEASES_URL } from "./config";

export interface Release {
  version: string | null;
  downloadUrl: string;
}

interface GitHubRelease {
  tag_name: string;
  assets: { name: string; browser_download_url: string }[];
}

/** Latest version and a direct installer link; falls back to the releases page if GitHub can't be reached. */
export function useLatestRelease(): Release {
  const [release, setRelease] = useState<Release>({ version: null, downloadUrl: RELEASES_URL });

  useEffect(() => {
    const controller = new AbortController();
    fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { signal: controller.signal })
      .then((res) => (res.ok ? (res.json() as Promise<GitHubRelease>) : Promise.reject()))
      .then((data) => {
        const installer = data.assets.find((a) => a.name.endsWith("-setup.exe"));
        setRelease({
          version: data.tag_name.replace(/^v/, ""),
          downloadUrl: installer?.browser_download_url ?? RELEASES_URL,
        });
      })
      .catch(() => {});
    return () => controller.abort();
  }, []);

  return release;
}
