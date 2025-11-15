import { useEffect, useState } from 'react';

interface UpdatePayload {
  version: string;
  release_notes?: string;
  required?: boolean;
  update?: Record<string, string>;
}

export interface AppUpdateInfo {
  version: string;
  releaseNotes?: string;
  required?: boolean;
  downloadUrl: string;
  releasePage: string;
}

const UPDATE_ENDPOINT = 'https://mirror.duckcoding.com/api/v1/update';
const GITHUB_RELEASE = 'https://github.com/DuckCoding-dev/DuckCoding/releases';

export function useAppUpdate() {
  const [info, setInfo] = useState<AppUpdateInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    async function fetchUpdate() {
      try {
        const response = await fetch(UPDATE_ENDPOINT, { cache: 'no-store' });
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        const data: UpdatePayload = await response.json();
        if (!data.version || compareVersions(data.version, __APP_VERSION__) <= 0) {
          return;
        }

        const downloadUrl =
          selectDownloadLink(data.update) ?? `${GITHUB_RELEASE}/tag/v${data.version}`;
        if (mounted) {
          setInfo({
            version: data.version,
            releaseNotes: data.release_notes,
            required: data.required,
            downloadUrl,
            releasePage: `${GITHUB_RELEASE}/tag/v${data.version}`,
          });
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    }

    fetchUpdate();

    return () => {
      mounted = false;
    };
  }, []);

  return { info, loading, error };
}

function compareVersions(a: string, b: string): number {
  const parse = (v: string) => v.split('.').map((n) => parseInt(n, 10) || 0);
  const av = parse(a);
  const bv = parse(b);

  for (let i = 0; i < Math.max(av.length, bv.length); i += 1) {
    const diff = (av[i] || 0) - (bv[i] || 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

function selectDownloadLink(update?: Record<string, string>): string | undefined {
  if (!update) return undefined;
  const ua = navigator.userAgent.toLowerCase();

  if (ua.includes('mac')) {
    return update.macos_dmg || update.macos || update.dmg;
  }
  if (ua.includes('win')) {
    return update.windows_exe || update.windows_msi || update.windows;
  }
  if (ua.includes('linux')) {
    return update.linux_appimage || update.linux_deb || update.linux_rpm || update.linux;
  }
  return undefined;
}
