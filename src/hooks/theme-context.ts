import { createContext } from 'react';
import { ActualTheme, ThemeMode, ThemePalette, ThemePreset, ThemeTokens } from './theme-palette';

export type Theme = ThemeMode;

export interface ThemeContextType {
  theme: Theme;
  actualTheme: ActualTheme; // 实际应用的主题（考虑系统设置）
  preset: ThemePreset;
  palette: ThemePalette;
  hasCustomPalette: boolean;
  setTheme: (theme: Theme) => void;
  setPreset: (preset: ThemePreset) => void;
  updatePaletteToken: (mode: ActualTheme, token: keyof ThemeTokens, value: string) => void;
  resetPalette: () => void;
}

export const ThemeContext = createContext<ThemeContextType | undefined>(undefined);
