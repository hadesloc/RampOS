"use client";

import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import { useTheme as useNextTheme } from 'next-themes';
import type { ThemeConfig, ThemeColors, HSLColor } from './config';
import { hslToString } from './config';
import { defaultTheme, getPresetById } from './presets';

interface WhiteLabelContextValue {
  theme: ThemeConfig;
  setTheme: (theme: ThemeConfig) => void;
  setThemeById: (id: string) => void;
  updateColors: (mode: 'light' | 'dark', colors: Partial<ThemeColors>) => void;
  updateLogo: (logo: Partial<ThemeConfig['logo']>) => void;
  updateBrandName: (name: string) => void;
  resetToDefault: () => void;
  isLoading: boolean;
}

const WhiteLabelContext = createContext<WhiteLabelContextValue | undefined>(undefined);

const THEME_STORAGE_KEY = 'whitelabel-theme';

// Inject CSS variables into document
function injectThemeVariables(theme: ThemeConfig, mode: 'light' | 'dark') {
  const colors = mode === 'dark' ? theme.colors.dark : theme.colors.light;
  const root = document.documentElement;

  // Color variables
  const colorVars: Record<string, HSLColor> = {
    '--background': colors.background,
    '--foreground': colors.foreground,
    '--card': colors.card,
    '--card-foreground': colors.cardForeground,
    '--popover': colors.card,
    '--popover-foreground': colors.cardForeground,
    '--primary': colors.primary,
    '--primary-foreground': colors.primaryForeground,
    '--secondary': colors.secondary,
    '--secondary-foreground': colors.secondaryForeground,
    '--muted': colors.muted,
    '--muted-foreground': colors.mutedForeground,
    '--accent': colors.accent,
    '--accent-foreground': colors.accentForeground,
    '--destructive': colors.destructive,
    '--destructive-foreground': colors.destructiveForeground,
    '--border': colors.border,
    '--input': colors.input,
    '--ring': colors.ring,
    '--success': colors.success,
    '--warning': colors.warning,
    '--info': colors.info,
  };

  Object.entries(colorVars).forEach(([varName, hsl]) => {
    root.style.setProperty(varName, hslToString(hsl));
  });

  // Border radius
  root.style.setProperty('--radius', theme.borderRadius.md);

  // Custom CSS
  if (theme.customCss) {
    let styleEl = document.getElementById('whitelabel-custom-css');
    if (!styleEl) {
      styleEl = document.createElement('style');
      styleEl.id = 'whitelabel-custom-css';
      document.head.appendChild(styleEl);
    }
    styleEl.textContent = theme.customCss;
  }
}

// Inject font links
function injectFonts(theme: ThemeConfig) {
  const fonts = [theme.fonts.heading, theme.fonts.body, theme.fonts.mono];
  const existingLinks = document.querySelectorAll('link[data-whitelabel-font]');
  existingLinks.forEach((link) => link.remove());

  const urls = new Set<string>();
  fonts.forEach((font) => {
    if (font.url) {
      urls.add(font.url);
    }
  });

  urls.forEach((url) => {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = url;
    link.setAttribute('data-whitelabel-font', 'true');
    document.head.appendChild(link);
  });

  // Apply font families
  const root = document.documentElement;
  root.style.setProperty('--font-heading', theme.fonts.heading.family);
  root.style.setProperty('--font-body', theme.fonts.body.family);
  root.style.setProperty('--font-mono', theme.fonts.mono.family);
}

// Update favicon
function updateFavicon(faviconUrl: string) {
  let link: HTMLLinkElement | null = document.querySelector("link[rel~='icon']");
  if (!link) {
    link = document.createElement('link');
    link.rel = 'icon';
    document.head.appendChild(link);
  }
  link.href = faviconUrl;
}

interface WhiteLabelProviderProps {
  children: React.ReactNode;
  initialTheme?: ThemeConfig;
  tenantId?: string;
}

export function WhiteLabelProvider({
  children,
  initialTheme,
  tenantId,
}: WhiteLabelProviderProps) {
  const [theme, setThemeState] = useState<ThemeConfig>(initialTheme || defaultTheme);
  const [isLoading, setIsLoading] = useState(true);
  const { resolvedTheme } = useNextTheme();

  // Load theme from storage or API
  useEffect(() => {
    const loadTheme = async () => {
      setIsLoading(true);
      try {
        // Try to load from localStorage first
        const stored = localStorage.getItem(THEME_STORAGE_KEY);
        if (stored) {
          const parsed = JSON.parse(stored);
          setThemeState(parsed);
        } else if (tenantId) {
          // In production, fetch from API
          // const response = await fetch(`/api/tenant/${tenantId}/theme`);
          // const data = await response.json();
          // setThemeState(data);
        }
      } catch (error) {
        console.error('Failed to load theme:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadTheme();
  }, [tenantId]);

  // Apply theme when it changes or dark/light mode changes
  useEffect(() => {
    if (!isLoading) {
      const mode = resolvedTheme === 'dark' ? 'dark' : 'light';
      injectThemeVariables(theme, mode);
      injectFonts(theme);
      updateFavicon(theme.logo.favicon);

      // Update document title
      if (theme.brandName) {
        const currentTitle = document.title;
        if (!currentTitle.includes(theme.brandName)) {
          document.title = `${theme.brandName} - Admin`;
        }
      }
    }
  }, [theme, resolvedTheme, isLoading]);

  const setTheme = useCallback((newTheme: ThemeConfig) => {
    setThemeState(newTheme);
    localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(newTheme));
  }, []);

  const setThemeById = useCallback((id: string) => {
    const preset = getPresetById(id);
    if (preset) {
      setTheme(preset);
    }
  }, [setTheme]);

  const updateColors = useCallback((mode: 'light' | 'dark', colors: Partial<ThemeColors>) => {
    setThemeState((prev) => ({
      ...prev,
      colors: {
        ...prev.colors,
        [mode]: {
          ...prev.colors[mode],
          ...colors,
        },
      },
      updatedAt: new Date().toISOString(),
    }));
  }, []);

  const updateLogo = useCallback((logo: Partial<ThemeConfig['logo']>) => {
    setThemeState((prev) => ({
      ...prev,
      logo: {
        ...prev.logo,
        ...logo,
      },
      updatedAt: new Date().toISOString(),
    }));
  }, []);

  const updateBrandName = useCallback((name: string) => {
    setThemeState((prev) => ({
      ...prev,
      brandName: name,
      updatedAt: new Date().toISOString(),
    }));
  }, []);

  const resetToDefault = useCallback(() => {
    setTheme(defaultTheme);
    localStorage.removeItem(THEME_STORAGE_KEY);
  }, [setTheme]);

  const value: WhiteLabelContextValue = {
    theme,
    setTheme,
    setThemeById,
    updateColors,
    updateLogo,
    updateBrandName,
    resetToDefault,
    isLoading,
  };

  return (
    <WhiteLabelContext.Provider value={value}>
      {children}
    </WhiteLabelContext.Provider>
  );
}

export function useWhiteLabel(): WhiteLabelContextValue {
  const context = useContext(WhiteLabelContext);
  if (!context) {
    throw new Error('useWhiteLabel must be used within a WhiteLabelProvider');
  }
  return context;
}

// Export a hook for getting just the theme config (useful for components)
export function useThemeConfig(): ThemeConfig {
  const { theme } = useWhiteLabel();
  return theme;
}

// Export a hook for getting brand info
export function useBrandInfo() {
  const { theme } = useWhiteLabel();
  return {
    brandName: theme.brandName,
    tagline: theme.tagline,
    logo: theme.logo,
  };
}
