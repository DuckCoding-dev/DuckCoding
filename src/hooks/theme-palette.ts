export type ThemeMode = 'light' | 'dark' | 'system';

export type ThemePreset = 'default' | 'green' | 'custom';

export interface ThemeTokens {
  background: string;
  foreground: string;
  card: string;
  cardForeground: string;
  popover: string;
  popoverForeground: string;
  primary: string;
  primaryForeground: string;
  secondary: string;
  secondaryForeground: string;
  muted: string;
  mutedForeground: string;
  border: string;
  input: string;
  ring: string;
  accent: string;
  accentForeground: string;
}

export interface ThemePalette {
  light: ThemeTokens;
  dark: ThemeTokens;
}

export type ActualTheme = 'light' | 'dark';

export type ThemePaletteInput = Partial<{
  light: Partial<ThemeTokens>;
  dark: Partial<ThemeTokens>;
}>;

const CSS_VARIABLE_MAP: Record<keyof ThemeTokens, string> = {
  background: '--background',
  foreground: '--foreground',
  card: '--card',
  cardForeground: '--card-foreground',
  popover: '--popover',
  popoverForeground: '--popover-foreground',
  primary: '--primary',
  primaryForeground: '--primary-foreground',
  secondary: '--secondary',
  secondaryForeground: '--secondary-foreground',
  muted: '--muted',
  mutedForeground: '--muted-foreground',
  border: '--border',
  input: '--input',
  ring: '--ring',
  accent: '--accent',
  accentForeground: '--accent-foreground',
};

export const DEFAULT_THEME_PALETTE: ThemePalette = {
  light: {
    background: '0 0% 100%',
    foreground: '240 10% 3.9%',
    card: '0 0% 100%',
    cardForeground: '240 10% 3.9%',
    popover: '0 0% 100%',
    popoverForeground: '240 10% 3.9%',
    primary: '226 70% 55.5%',
    primaryForeground: '0 0% 98%',
    secondary: '240 4.8% 95.9%',
    secondaryForeground: '240 5.9% 10%',
    muted: '240 4.8% 95.9%',
    mutedForeground: '240 3.8% 46.1%',
    border: '240 5.9% 90%',
    input: '240 5.9% 90%',
    ring: '226 70% 55.5%',
    accent: '240 4.8% 95.9%',
    accentForeground: '240 5.9% 10%',
  },
  dark: {
    background: '240 10% 3.9%',
    foreground: '0 0% 98%',
    card: '240 10% 3.9%',
    cardForeground: '0 0% 98%',
    popover: '240 10% 3.9%',
    popoverForeground: '0 0% 98%',
    primary: '226 70% 55.5%',
    primaryForeground: '0 0% 98%',
    secondary: '240 3.7% 15.9%',
    secondaryForeground: '0 0% 98%',
    muted: '240 3.7% 15.9%',
    mutedForeground: '240 5% 64.9%',
    border: '240 3.7% 15.9%',
    input: '240 3.7% 15.9%',
    ring: '226 70% 55.5%',
    accent: '240 3.7% 15.9%',
    accentForeground: '0 0% 98%',
  },
};

export const GREEN_THEME_PALETTE: ThemePalette = {
  light: {
    ...DEFAULT_THEME_PALETTE.light,
    primary: '142 71% 45%',
    primaryForeground: '138 76% 97%',
    ring: '142 71% 45%',
    accent: '138 47% 96%',
    accentForeground: '142 72% 20%',
  },
  dark: {
    ...DEFAULT_THEME_PALETTE.dark,
    primary: '142 69% 45%',
    primaryForeground: '144 61% 10%',
    ring: '142 69% 45%',
    accent: '142 32% 18%',
    accentForeground: '138 76% 97%',
  },
};

export function mergeThemePalette(input?: ThemePaletteInput | null): ThemePalette {
  return {
    light: {
      ...DEFAULT_THEME_PALETTE.light,
      ...(input?.light ?? {}),
    },
    dark: {
      ...DEFAULT_THEME_PALETTE.dark,
      ...(input?.dark ?? {}),
    },
  };
}

export function resolveActivePalette(
  preset: ThemePreset,
  customPalette?: ThemePaletteInput | null,
): ThemePalette {
  if (preset === 'green') {
    return GREEN_THEME_PALETTE;
  }

  if (preset === 'custom') {
    return mergeThemePalette(customPalette);
  }

  return DEFAULT_THEME_PALETTE;
}

export function buildCssVariables(tokens: ThemeTokens): Record<string, string> {
  return Object.entries(CSS_VARIABLE_MAP).reduce<Record<string, string>>((acc, [key, variable]) => {
    acc[variable] = tokens[key as keyof ThemeTokens];
    return acc;
  }, {});
}

export function resolveActualTheme(mode: ThemeMode, systemTheme: ActualTheme): ActualTheme {
  return mode === 'system' ? systemTheme : mode;
}

interface RootThemeTarget {
  classList: {
    add: (...tokens: string[]) => void;
    remove: (...tokens: string[]) => void;
  };
  style: {
    setProperty: (property: string, value: string) => void;
  };
}

export function applyThemeToRoot(
  root: RootThemeTarget,
  actualTheme: ActualTheme,
  tokens: ThemeTokens,
): void {
  root.classList.remove('light', 'dark');
  root.classList.add(actualTheme);

  const cssVariables = buildCssVariables(tokens);
  Object.entries(cssVariables).forEach(([key, value]) => {
    root.style.setProperty(key, value);
  });
}

function formatHue(hue: number): string {
  return String(Math.round(hue));
}

function formatPercent(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}

function clampRgbChannel(value: number): number {
  return Math.max(0, Math.min(255, Math.round(value)));
}

export function hslTokenToHex(token: string): string {
  const [hRaw, sRaw, lRaw] = token.trim().split(/\s+/);
  const hue = Number.parseFloat(hRaw);
  const saturation = Number.parseFloat(sRaw.replace('%', '')) / 100;
  const lightness = Number.parseFloat(lRaw.replace('%', '')) / 100;

  if ([hue, saturation, lightness].some((value) => Number.isNaN(value))) {
    return '#000000';
  }

  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const hueSection = hue / 60;
  const x = chroma * (1 - Math.abs((hueSection % 2) - 1));
  const match = lightness - chroma / 2;

  let red = 0;
  let green = 0;
  let blue = 0;

  if (hueSection >= 0 && hueSection < 1) {
    red = chroma;
    green = x;
  } else if (hueSection < 2) {
    red = x;
    green = chroma;
  } else if (hueSection < 3) {
    green = chroma;
    blue = x;
  } else if (hueSection < 4) {
    green = x;
    blue = chroma;
  } else if (hueSection < 5) {
    red = x;
    blue = chroma;
  } else {
    red = chroma;
    blue = x;
  }

  const toHex = (value: number) =>
    clampRgbChannel((value + match) * 255)
      .toString(16)
      .padStart(2, '0');
  return `#${toHex(red)}${toHex(green)}${toHex(blue)}`;
}

export function hexToHslToken(hex: string): string {
  const normalized = hex.replace('#', '');
  if (normalized.length !== 3 && normalized.length !== 6) {
    return DEFAULT_THEME_PALETTE.light.primary;
  }
  const expanded =
    normalized.length === 3
      ? normalized
          .split('')
          .map((part) => `${part}${part}`)
          .join('')
      : normalized;

  const red = Number.parseInt(expanded.slice(0, 2), 16) / 255;
  const green = Number.parseInt(expanded.slice(2, 4), 16) / 255;
  const blue = Number.parseInt(expanded.slice(4, 6), 16) / 255;

  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  const lightness = (max + min) / 2;

  let hue = 0;
  if (delta !== 0) {
    if (max === red) {
      hue = ((green - blue) / delta) % 6;
    } else if (max === green) {
      hue = (blue - red) / delta + 2;
    } else {
      hue = (red - green) / delta + 4;
    }
  }
  hue = Math.round((hue * 60 + 360) % 360);

  const saturation = delta === 0 ? 0 : delta / (1 - Math.abs(2 * lightness - 1));

  return `${formatHue(hue)} ${formatPercent(saturation * 100)}% ${formatPercent(lightness * 100)}%`;
}

interface CustomPaletteUpdateInput {
  preset: ThemePreset;
  customPalette?: ThemePaletteInput | null;
  mode: 'light' | 'dark';
  token: keyof ThemeTokens;
  value: string;
}

export function createCustomPaletteUpdate({
  preset,
  customPalette,
  mode,
  token,
  value,
}: CustomPaletteUpdateInput): { preset: ThemePreset; palette: ThemePalette } {
  const basePalette = resolveActivePalette(preset, customPalette);
  return {
    preset: 'custom',
    palette: {
      light: { ...basePalette.light },
      dark: { ...basePalette.dark },
      [mode]: {
        ...basePalette[mode],
        [token]: value,
      },
    },
  };
}
