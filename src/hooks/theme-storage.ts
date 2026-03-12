import { ThemeMode, ThemePaletteInput, ThemePreset } from './theme-palette';

export const THEME_STORAGE_KEYS = {
  mode: 'duckcoding-theme',
  preset: 'duckcoding-theme-preset',
  palette: 'duckcoding-theme-palette',
} as const;

type StorageReader = (key: string) => string | null;

function isThemeMode(value: string | null): value is ThemeMode {
  return value === 'light' || value === 'dark' || value === 'system';
}

function isThemePreset(value: string | null): value is ThemePreset {
  return value === 'default' || value === 'green' || value === 'custom';
}

export function readStoredThemeMode(read: StorageReader): ThemeMode {
  const value = read(THEME_STORAGE_KEYS.mode);
  return isThemeMode(value) ? value : 'system';
}

export function readStoredThemePreset(read: StorageReader): ThemePreset {
  const value = read(THEME_STORAGE_KEYS.preset);
  return isThemePreset(value) ? value : 'default';
}

export function readStoredPalette(read: StorageReader): ThemePaletteInput | null {
  const raw = read(THEME_STORAGE_KEYS.palette);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as ThemePaletteInput;
  } catch {
    return null;
  }
}
