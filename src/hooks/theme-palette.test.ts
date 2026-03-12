import { describe, expect, it } from 'vitest';
import {
  applyThemeToRoot,
  buildCssVariables,
  createCustomPaletteUpdate,
  DEFAULT_THEME_PALETTE,
  GREEN_THEME_PALETTE,
  hexToHslToken,
  hslTokenToHex,
  mergeThemePalette,
  resolveActualTheme,
  resolveActivePalette,
} from './theme-palette';
import {
  readStoredPalette,
  readStoredThemeMode,
  readStoredThemePreset,
  THEME_STORAGE_KEYS,
} from './theme-storage';

describe('theme palette helpers', () => {
  it('returns default palette when preset is default', () => {
    const palette = resolveActivePalette('default', null);

    expect(palette).toEqual(DEFAULT_THEME_PALETTE);
  });

  it('returns built-in green palette when preset is green', () => {
    const palette = resolveActivePalette('green', null);

    expect(palette).toEqual(GREEN_THEME_PALETTE);
  });

  it('merges custom palette values over defaults', () => {
    const palette = mergeThemePalette({
      light: {
        primary: '142 71% 45%',
      },
      dark: {
        background: '152 40% 8%',
      },
    });

    expect(palette.light.primary).toBe('142 71% 45%');
    expect(palette.light.background).toBe(DEFAULT_THEME_PALETTE.light.background);
    expect(palette.dark.background).toBe('152 40% 8%');
    expect(palette.dark.primary).toBe(DEFAULT_THEME_PALETTE.dark.primary);
  });

  it('maps palette tokens to CSS custom properties', () => {
    const cssVariables = buildCssVariables(DEFAULT_THEME_PALETTE.light);

    expect(cssVariables).toMatchObject({
      '--background': DEFAULT_THEME_PALETTE.light.background,
      '--foreground': DEFAULT_THEME_PALETTE.light.foreground,
      '--card-foreground': DEFAULT_THEME_PALETTE.light.cardForeground,
      '--primary': DEFAULT_THEME_PALETTE.light.primary,
      '--primary-foreground': DEFAULT_THEME_PALETTE.light.primaryForeground,
      '--accent': DEFAULT_THEME_PALETTE.light.accent,
      '--accent-foreground': DEFAULT_THEME_PALETTE.light.accentForeground,
      '--border': DEFAULT_THEME_PALETTE.light.border,
      '--input': DEFAULT_THEME_PALETTE.light.input,
      '--ring': DEFAULT_THEME_PALETTE.light.ring,
    });
  });

  it('resolves actual theme from system mode and system preference', () => {
    expect(resolveActualTheme('system', 'dark')).toBe('dark');
    expect(resolveActualTheme('system', 'light')).toBe('light');
    expect(resolveActualTheme('dark', 'light')).toBe('dark');
  });

  it('reads stored theme settings with safe fallbacks', () => {
    const storage = new Map<string, string>([
      [THEME_STORAGE_KEYS.mode, 'dark'],
      [THEME_STORAGE_KEYS.preset, 'green'],
      [
        THEME_STORAGE_KEYS.palette,
        JSON.stringify({
          light: { primary: '142 71% 45%' },
        }),
      ],
    ]);

    const read = (key: string) => storage.get(key) ?? null;

    expect(readStoredThemeMode(read)).toBe('dark');
    expect(readStoredThemePreset(read)).toBe('green');
    expect(readStoredPalette(read)).toEqual({
      light: { primary: '142 71% 45%' },
    });
  });

  it('applies theme class and CSS variables to the root target', () => {
    const classes = new Set<string>(['light']);
    const styles = new Map<string, string>();

    const root = {
      classList: {
        add: (...values: string[]) => values.forEach((value) => classes.add(value)),
        remove: (...values: string[]) => values.forEach((value) => classes.delete(value)),
      },
      style: {
        setProperty: (key: string, value: string) => {
          styles.set(key, value);
        },
      },
    };

    applyThemeToRoot(root, 'dark', GREEN_THEME_PALETTE.dark);

    expect(classes.has('light')).toBe(false);
    expect(classes.has('dark')).toBe(true);
    expect(styles.get('--primary')).toBe(GREEN_THEME_PALETTE.dark.primary);
    expect(styles.get('--accent-foreground')).toBe(GREEN_THEME_PALETTE.dark.accentForeground);
  });

  it('converts theme tokens between HSL and hex for color inputs', () => {
    expect(hslTokenToHex('226 70% 55.5%')).toBe('#3e63dd');
    expect(hexToHslToken('#16a34a')).toBe('142 76.2% 36.3%');
  });

  it('promotes the active preset palette to custom before editing a token', () => {
    const result = createCustomPaletteUpdate({
      preset: 'green',
      customPalette: null,
      mode: 'dark',
      token: 'background',
      value: '155 30% 12%',
    });

    expect(result.preset).toBe('custom');
    expect(result.palette.dark.background).toBe('155 30% 12%');
    expect(result.palette.dark.primary).toBe(GREEN_THEME_PALETTE.dark.primary);
    expect(result.palette.light.primary).toBe(GREEN_THEME_PALETTE.light.primary);
  });
});
