import type { HeadlessCheckoutConfig } from '../types/index';
import {
  fetchRemoteCheckoutConfig,
  mergeCheckoutConfig,
  type FetchLike,
} from '../config/remote-config';
import { resolveThemeTokens } from '../config/theme-tokens';

export function buildHeadlessCheckoutConfig(
  base: HeadlessCheckoutConfig,
  remote?: Partial<HeadlessCheckoutConfig>,
): HeadlessCheckoutConfig {
  const merged = mergeCheckoutConfig(base, remote);
  return {
    ...merged,
    theme: resolveThemeTokens(merged.theme, merged.themeTokens),
  };
}

export async function resolveHeadlessCheckoutConfig(
  base: HeadlessCheckoutConfig,
  fetcher?: FetchLike,
): Promise<HeadlessCheckoutConfig> {
  const remote = await fetchRemoteCheckoutConfig(base.remoteConfig, fetcher);
  return buildHeadlessCheckoutConfig(base, remote);
}
