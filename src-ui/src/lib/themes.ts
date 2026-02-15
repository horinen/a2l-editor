import type { ThemeName } from './types';

interface ThemeColors {
  bg: string;
  'bg-hover': string;
  'bg-selected': string;
  text: string;
  'text-muted': string;
  border: string;
  accent: string;
}

interface Theme {
  name: string;
  colors: ThemeColors;
}

export const themes: Record<ThemeName, Theme> = {
  dark: {
    name: 'Dark',
    colors: {
      bg: '#0f0f12',
      'bg-hover': '#1a1a1f',
      'bg-selected': '#1e3a5f',
      text: '#e4e4e7',
      'text-muted': '#71717a',
      border: '#27272a',
      accent: '#3b82f6',
    }
  },
  light: {
    name: 'Light',
    colors: {
      bg: '#ffffff',
      'bg-hover': '#f4f4f5',
      'bg-selected': '#dbeafe',
      text: '#18181b',
      'text-muted': '#a1a1aa',
      border: '#e4e4e7',
      accent: '#3b82f6',
    }
  },
  midnight: {
    name: 'Midnight',
    colors: {
      bg: '#000000',
      'bg-hover': '#0a0a0a',
      'bg-selected': '#0c1929',
      text: '#fafafa',
      'text-muted': '#52525b',
      border: '#18181b',
      accent: '#3b82f6',
    }
  },
  ocean: {
    name: 'Ocean',
    colors: {
      bg: '#0c1222',
      'bg-hover': '#141d32',
      'bg-selected': '#1e3a5f',
      text: '#e0f2fe',
      'text-muted': '#64748b',
      border: '#1e293b',
      accent: '#06b6d4',
    }
  }
};

export const themeNames: ThemeName[] = ['dark', 'light', 'midnight', 'ocean'];

export function applyTheme(name: ThemeName) {
  const theme = themes[name];
  if (!theme) return;

  const root = document.documentElement;
  
  // 移除所有主题类
  root.classList.remove('light', 'midnight', 'ocean');
  
  // 添加新主题类（dark 是默认，不需要添加）
  if (name !== 'dark') {
    root.classList.add(name);
  }

  // 保存到 localStorage
  localStorage.setItem('theme', name);
}

export function getSavedTheme(): ThemeName {
  const saved = localStorage.getItem('theme') as ThemeName | null;
  if (saved && themes[saved]) {
    return saved;
  }
  return 'dark';
}

export function cycleTheme(current: ThemeName): ThemeName {
  const index = themeNames.indexOf(current);
  const nextIndex = (index + 1) % themeNames.length;
  return themeNames[nextIndex];
}
