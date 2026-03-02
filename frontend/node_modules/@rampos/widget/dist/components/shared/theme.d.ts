import { WidgetTheme } from '../../types/index';
export declare function resolveTheme(theme?: WidgetTheme): Required<WidgetTheme>;
export declare function themeToCSS(theme: Required<WidgetTheme>): React.CSSProperties;
export declare function themeToCSSVars(theme: Required<WidgetTheme>): Record<string, string>;
