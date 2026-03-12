import { useEffect, useState } from 'react';
import { Theme, ThemeContext } from './theme-context';
import {
  applyThemeToRoot,
  createCustomPaletteUpdate,
  resolveActivePalette,
  resolveActualTheme,
  ThemeTokens,
} from './theme-palette';
import {
  readStoredPalette,
  readStoredThemeMode,
  readStoredThemePreset,
  THEME_STORAGE_KEYS,
} from './theme-storage';

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => {
    return readStoredThemeMode((key) => localStorage.getItem(key));
  });

  const [preset, setPresetState] = useState(() =>
    readStoredThemePreset((key) => localStorage.getItem(key)),
  );

  const [customPalette, setCustomPalette] = useState(() =>
    readStoredPalette((key) => localStorage.getItem(key)),
  );

  const getSystemTheme = (): 'light' | 'dark' => {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  };

  const [systemTheme, setSystemTheme] = useState<'light' | 'dark'>(() => getSystemTheme());

  const actualTheme = resolveActualTheme(theme, systemTheme);
  const palette = resolveActivePalette(preset, customPalette);

  const setTheme = (newTheme: Theme) => {
    setThemeState(newTheme);
  };

  const setPreset = (newPreset: typeof preset) => {
    setPresetState(newPreset);
  };

  const updatePaletteToken = (mode: 'light' | 'dark', token: keyof ThemeTokens, value: string) => {
    const next = createCustomPaletteUpdate({
      preset,
      customPalette,
      mode,
      token,
      value,
    });
    setPresetState(next.preset);
    setCustomPalette(next.palette);
  };

  const resetPalette = () => {
    setPresetState('default');
    setCustomPalette(null);
  };

  useEffect(() => {
    applyThemeToRoot(window.document.documentElement, actualTheme, palette[actualTheme]);
  }, [actualTheme, palette]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      setSystemTheme(getSystemTheme());
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  useEffect(() => {
    localStorage.setItem(THEME_STORAGE_KEYS.mode, theme);
  }, [theme]);

  useEffect(() => {
    localStorage.setItem(THEME_STORAGE_KEYS.preset, preset);
  }, [preset]);

  useEffect(() => {
    if (customPalette) {
      localStorage.setItem(THEME_STORAGE_KEYS.palette, JSON.stringify(customPalette));
      return;
    }
    localStorage.removeItem(THEME_STORAGE_KEYS.palette);
  }, [customPalette]);

  return (
    <ThemeContext.Provider
      value={{
        theme,
        actualTheme,
        preset,
        palette,
        hasCustomPalette: customPalette !== null,
        setTheme,
        setPreset,
        updatePaletteToken,
        resetPalette,
      }}
    >
      {children}
    </ThemeContext.Provider>
  );
}
