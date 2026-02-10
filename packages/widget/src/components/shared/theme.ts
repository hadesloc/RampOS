import { WidgetTheme, DEFAULT_THEME } from '../../types/index';

export function resolveTheme(theme?: WidgetTheme): Required<WidgetTheme> {
  return {
    primaryColor: theme?.primaryColor ?? DEFAULT_THEME.primaryColor!,
    backgroundColor: theme?.backgroundColor ?? DEFAULT_THEME.backgroundColor!,
    textColor: theme?.textColor ?? DEFAULT_THEME.textColor!,
    borderRadius: theme?.borderRadius ?? DEFAULT_THEME.borderRadius!,
    fontFamily: theme?.fontFamily ?? DEFAULT_THEME.fontFamily!,
    errorColor: theme?.errorColor ?? DEFAULT_THEME.errorColor!,
    successColor: theme?.successColor ?? DEFAULT_THEME.successColor!,
  };
}

export function themeToCSS(theme: Required<WidgetTheme>): React.CSSProperties {
  return {
    fontFamily: theme.fontFamily,
    color: theme.textColor,
    backgroundColor: theme.backgroundColor,
    borderRadius: theme.borderRadius,
  };
}

export function themeToCSSVars(theme: Required<WidgetTheme>): Record<string, string> {
  return {
    '--rampos-primary-color': theme.primaryColor,
    '--rampos-background': theme.backgroundColor,
    '--rampos-text': theme.textColor,
    '--rampos-border-radius': theme.borderRadius,
    '--rampos-font-family': theme.fontFamily,
    '--rampos-error-color': theme.errorColor,
    '--rampos-success-color': theme.successColor,
  };
}
