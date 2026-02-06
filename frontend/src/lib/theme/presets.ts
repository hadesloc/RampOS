/**
 * Theme Presets
 *
 * Default theme configurations for white-label deployments.
 */

import type { ThemeConfig, ThemeColors } from './config';

// Default RampOS theme colors (light mode)
const defaultLightColors: ThemeColors = {
  primary: { h: 226, s: 71, l: 40 },
  primaryForeground: { h: 210, s: 40, l: 98 },
  secondary: { h: 210, s: 40, l: 96 },
  secondaryForeground: { h: 222, s: 47, l: 11 },
  accent: { h: 160, s: 84, l: 39 },
  accentForeground: { h: 210, s: 40, l: 98 },
  background: { h: 210, s: 40, l: 98 },
  foreground: { h: 222, s: 47, l: 11 },
  card: { h: 0, s: 0, l: 100 },
  cardForeground: { h: 222, s: 47, l: 11 },
  muted: { h: 210, s: 40, l: 96 },
  mutedForeground: { h: 215, s: 16, l: 47 },
  destructive: { h: 0, s: 84, l: 60 },
  destructiveForeground: { h: 210, s: 40, l: 98 },
  border: { h: 214, s: 32, l: 91 },
  input: { h: 214, s: 32, l: 91 },
  ring: { h: 226, s: 71, l: 40 },
  success: { h: 160, s: 84, l: 39 },
  warning: { h: 38, s: 92, l: 50 },
  info: { h: 199, s: 89, l: 48 },
};

// Default RampOS theme colors (dark mode)
const defaultDarkColors: ThemeColors = {
  primary: { h: 226, s: 71, l: 55 },
  primaryForeground: { h: 210, s: 40, l: 98 },
  secondary: { h: 217, s: 33, l: 17 },
  secondaryForeground: { h: 210, s: 40, l: 98 },
  accent: { h: 160, s: 84, l: 39 },
  accentForeground: { h: 210, s: 40, l: 98 },
  background: { h: 222, s: 47, l: 11 },
  foreground: { h: 210, s: 40, l: 98 },
  card: { h: 222, s: 47, l: 11 },
  cardForeground: { h: 210, s: 40, l: 98 },
  muted: { h: 217, s: 33, l: 17 },
  mutedForeground: { h: 215, s: 20, l: 65 },
  destructive: { h: 0, s: 62, l: 30 },
  destructiveForeground: { h: 210, s: 40, l: 98 },
  border: { h: 217, s: 33, l: 20 },
  input: { h: 217, s: 33, l: 20 },
  ring: { h: 226, s: 71, l: 55 },
  success: { h: 160, s: 84, l: 39 },
  warning: { h: 38, s: 92, l: 50 },
  info: { h: 199, s: 89, l: 48 },
};

// Default RampOS Theme
export const defaultTheme: ThemeConfig = {
  id: 'default',
  name: 'RampOS Default',
  description: 'The default RampOS fintech theme',
  brandName: 'RampOS',
  tagline: 'Fiat-to-Crypto Infrastructure',
  colors: {
    light: defaultLightColors,
    dark: defaultDarkColors,
  },
  fonts: {
    heading: {
      family: 'Inter',
      url: 'https://fonts.googleapis.com/css2?family=Inter:wght@500;600;700&display=swap',
      weights: [500, 600, 700],
    },
    body: {
      family: 'Inter',
      url: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap',
      weights: [400, 500, 600],
    },
    mono: {
      family: 'JetBrains Mono',
      url: 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&display=swap',
      weights: [400, 500],
    },
  },
  logo: {
    light: '/logo.svg',
    dark: '/logo-dark.svg',
    favicon: '/favicon.ico',
    width: 120,
    height: 32,
  },
  borderRadius: {
    sm: '0.25rem',
    md: '0.5rem',
    lg: '0.75rem',
    xl: '1rem',
    full: '9999px',
  },
  spacing: {
    unit: 4,
  },
  shadows: {
    sm: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
    md: '0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)',
    lg: '0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)',
    xl: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)',
  },
  isDefault: true,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
};

// Modern Gradient Theme
export const modernGradientTheme: ThemeConfig = {
  ...defaultTheme,
  id: 'modern-gradient',
  name: 'Modern Gradient',
  description: 'A modern theme with gradient accents',
  brandName: 'PayFlow',
  colors: {
    light: {
      ...defaultLightColors,
      primary: { h: 262, s: 83, l: 58 },
      accent: { h: 330, s: 81, l: 60 },
      ring: { h: 262, s: 83, l: 58 },
    },
    dark: {
      ...defaultDarkColors,
      primary: { h: 262, s: 83, l: 65 },
      accent: { h: 330, s: 81, l: 65 },
      ring: { h: 262, s: 83, l: 65 },
    },
  },
  isDefault: false,
};

// Corporate Blue Theme
export const corporateBlueTheme: ThemeConfig = {
  ...defaultTheme,
  id: 'corporate-blue',
  name: 'Corporate Blue',
  description: 'Professional corporate styling',
  brandName: 'CorpPay',
  colors: {
    light: {
      ...defaultLightColors,
      primary: { h: 210, s: 100, l: 35 },
      accent: { h: 180, s: 60, l: 40 },
      ring: { h: 210, s: 100, l: 35 },
    },
    dark: {
      ...defaultDarkColors,
      primary: { h: 210, s: 100, l: 50 },
      accent: { h: 180, s: 60, l: 50 },
      ring: { h: 210, s: 100, l: 50 },
    },
  },
  isDefault: false,
};

// Emerald Fintech Theme
export const emeraldFintechTheme: ThemeConfig = {
  ...defaultTheme,
  id: 'emerald-fintech',
  name: 'Emerald Fintech',
  description: 'Green-focused fintech theme',
  brandName: 'GreenPay',
  colors: {
    light: {
      ...defaultLightColors,
      primary: { h: 160, s: 84, l: 39 },
      accent: { h: 199, s: 89, l: 48 },
      ring: { h: 160, s: 84, l: 39 },
    },
    dark: {
      ...defaultDarkColors,
      primary: { h: 160, s: 84, l: 45 },
      accent: { h: 199, s: 89, l: 55 },
      ring: { h: 160, s: 84, l: 45 },
    },
  },
  isDefault: false,
};

// Sunset Orange Theme
export const sunsetOrangeTheme: ThemeConfig = {
  ...defaultTheme,
  id: 'sunset-orange',
  name: 'Sunset Orange',
  description: 'Warm and vibrant orange theme',
  brandName: 'SunPay',
  colors: {
    light: {
      ...defaultLightColors,
      primary: { h: 25, s: 95, l: 53 },
      accent: { h: 350, s: 89, l: 60 },
      ring: { h: 25, s: 95, l: 53 },
    },
    dark: {
      ...defaultDarkColors,
      primary: { h: 25, s: 95, l: 60 },
      accent: { h: 350, s: 89, l: 65 },
      ring: { h: 25, s: 95, l: 60 },
    },
  },
  isDefault: false,
};

// Midnight Dark Theme
export const midnightDarkTheme: ThemeConfig = {
  ...defaultTheme,
  id: 'midnight-dark',
  name: 'Midnight Dark',
  description: 'Ultra-dark theme for low-light environments',
  brandName: 'DarkPay',
  colors: {
    light: defaultLightColors,
    dark: {
      ...defaultDarkColors,
      background: { h: 240, s: 10, l: 4 },
      card: { h: 240, s: 10, l: 6 },
      muted: { h: 240, s: 5, l: 12 },
      border: { h: 240, s: 5, l: 15 },
      input: { h: 240, s: 5, l: 15 },
    },
  },
  isDefault: false,
};

// All available presets
export const themePresets: ThemeConfig[] = [
  defaultTheme,
  modernGradientTheme,
  corporateBlueTheme,
  emeraldFintechTheme,
  sunsetOrangeTheme,
  midnightDarkTheme,
];

// Get preset by ID
export function getPresetById(id: string): ThemeConfig | undefined {
  return themePresets.find((preset) => preset.id === id);
}

// Get default preset
export function getDefaultPreset(): ThemeConfig {
  return defaultTheme;
}
