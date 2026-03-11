import type { HeadlessCheckoutConfig, RemoteWidgetConfig } from '../types/index';

type FetchLikeResponse = {
  ok: boolean;
  status: number;
  json: () => Promise<unknown>;
};

export type FetchLike = (
  input: string,
  init?: { headers?: Record<string, string> },
) => Promise<FetchLikeResponse>;

export async function fetchRemoteCheckoutConfig(
  remoteConfig?: RemoteWidgetConfig,
  fetcher?: FetchLike,
): Promise<Partial<HeadlessCheckoutConfig>> {
  if (!remoteConfig?.url) {
    return {};
  }

  const resolvedFetcher = fetcher ?? (globalThis.fetch as unknown as FetchLike);
  if (!resolvedFetcher) {
    throw new Error('Remote config fetcher is not available');
  }

  const response = await resolvedFetcher(remoteConfig.url, {
    headers: remoteConfig.headers,
  });
  if (!response.ok) {
    throw new Error(`Failed to fetch remote widget config: ${response.status}`);
  }

  const payload = await response.json();
  return (payload ?? {}) as Partial<HeadlessCheckoutConfig>;
}

export function mergeCheckoutConfig(
  base: HeadlessCheckoutConfig,
  remote?: Partial<HeadlessCheckoutConfig>,
): HeadlessCheckoutConfig {
  if (!remote) {
    return { ...base };
  }

  return {
    ...base,
    ...remote,
    apiKey: base.apiKey,
    theme: {
      ...base.theme,
      ...remote.theme,
    },
    themeTokens: {
      ...base.themeTokens,
      ...remote.themeTokens,
    },
    headless: {
      ...base.headless,
      ...remote.headless,
    },
    remoteConfig: base.remoteConfig ?? remote.remoteConfig,
  };
}
