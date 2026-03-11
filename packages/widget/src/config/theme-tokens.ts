import type { WidgetTheme, WidgetThemeTokens } from '../types/index';

export const DEFAULT_THEME_TOKENS: Required<WidgetThemeTokens> = {
  accentColor: '#2563eb',
  surfaceColor: '#ffffff',
  contentColor: '#1f2937',
  dangerColor: '#ef4444',
  successColor: '#10b981',
  radiusMd: '8px',
  fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
};

export function themeTokensToTheme(tokens?: WidgetThemeTokens): WidgetTheme {
  const merged = { ...DEFAULT_THEME_TOKENS, ...tokens };
  return {
    primaryColor: merged.accentColor,
    backgroundColor: merged.surfaceColor,
    textColor: merged.contentColor,
    errorColor: merged.dangerColor,
    successColor: merged.successColor,
    borderRadius: merged.radiusMd,
    fontFamily: merged.fontFamily,
  };
}

export function resolveThemeTokens(
  theme?: WidgetTheme,
  tokens?: WidgetThemeTokens,
): WidgetTheme {
  if (!tokens) {
    return theme ?? {};
  }

  return {
    ...themeTokensToTheme(tokens),
    ...theme,
  };
}
