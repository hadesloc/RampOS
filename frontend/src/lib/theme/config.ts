/**
 * White-label Theme Configuration Types
 *
 * Defines the structure for customizable theme settings per tenant.
 */

export type HSLColor = {
  h: number;
  s: number;
  l: number;
};

export interface ThemeColors {
  primary: HSLColor;
  primaryForeground: HSLColor;
  secondary: HSLColor;
  secondaryForeground: HSLColor;
  accent: HSLColor;
  accentForeground: HSLColor;
  background: HSLColor;
  foreground: HSLColor;
  card: HSLColor;
  cardForeground: HSLColor;
  muted: HSLColor;
  mutedForeground: HSLColor;
  destructive: HSLColor;
  destructiveForeground: HSLColor;
  border: HSLColor;
  input: HSLColor;
  ring: HSLColor;
  success: HSLColor;
  warning: HSLColor;
  info: HSLColor;
}

export interface ThemeLogo {
  light: string;
  dark: string;
  favicon: string;
  width?: number;
  height?: number;
}

export interface ThemeFont {
  family: string;
  url?: string;
  weights?: number[];
}

export interface ThemeFonts {
  heading: ThemeFont;
  body: ThemeFont;
  mono: ThemeFont;
}

export interface ThemeBorderRadius {
  sm: string;
  md: string;
  lg: string;
  xl: string;
  full: string;
}

export interface ThemeSpacing {
  unit: number;
}

export interface ThemeShadows {
  sm: string;
  md: string;
  lg: string;
  xl: string;
}

export interface ThemeConfig {
  id: string;
  name: string;
  description?: string;

  // Branding
  brandName: string;
  tagline?: string;

  // Colors
  colors: {
    light: ThemeColors;
    dark: ThemeColors;
  };

  // Typography
  fonts: ThemeFonts;

  // Logos
  logo: ThemeLogo;

  // Layout
  borderRadius: ThemeBorderRadius;
  spacing: ThemeSpacing;
  shadows: ThemeShadows;

  // Custom CSS
  customCss?: string;

  // Metadata
  tenantId?: string;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface TenantBranding {
  tenantId: string;
  themeId: string;
  customOverrides?: Partial<ThemeConfig>;
  enabled: boolean;
}

// Helper function to convert HSL to CSS string
export function hslToString(hsl: HSLColor): string {
  return `${hsl.h} ${hsl.s}% ${hsl.l}%`;
}

// Helper function to parse CSS HSL string
export function parseHSL(value: string): HSLColor {
  const parts = value.trim().split(/\s+/);
  return {
    h: parseFloat(parts[0] || '0'),
    s: parseFloat((parts[1] || '0%').replace('%', '')),
    l: parseFloat((parts[2] || '0%').replace('%', '')),
  };
}

// Helper function to convert hex to HSL
export function hexToHSL(hex: string): HSLColor {
  hex = hex.replace('#', '');

  const r = parseInt(hex.substring(0, 2), 16) / 255;
  const g = parseInt(hex.substring(2, 4), 16) / 255;
  const b = parseInt(hex.substring(4, 6), 16) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);

    switch (max) {
      case r:
        h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
        break;
      case g:
        h = ((b - r) / d + 2) / 6;
        break;
      case b:
        h = ((r - g) / d + 4) / 6;
        break;
    }
  }

  return {
    h: Math.round(h * 360),
    s: Math.round(s * 100),
    l: Math.round(l * 100),
  };
}

// Helper function to convert HSL to hex
export function hslToHex(hsl: HSLColor): string {
  const { h, s, l } = hsl;
  const sNorm = s / 100;
  const lNorm = l / 100;

  const c = (1 - Math.abs(2 * lNorm - 1)) * sNorm;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = lNorm - c / 2;

  let r = 0, g = 0, b = 0;

  if (h >= 0 && h < 60) {
    r = c; g = x; b = 0;
  } else if (h >= 60 && h < 120) {
    r = x; g = c; b = 0;
  } else if (h >= 120 && h < 180) {
    r = 0; g = c; b = x;
  } else if (h >= 180 && h < 240) {
    r = 0; g = x; b = c;
  } else if (h >= 240 && h < 300) {
    r = x; g = 0; b = c;
  } else {
    r = c; g = 0; b = x;
  }

  const toHex = (n: number) => {
    const hex = Math.round((n + m) * 255).toString(16);
    return hex.length === 1 ? '0' + hex : hex;
  };

  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}
