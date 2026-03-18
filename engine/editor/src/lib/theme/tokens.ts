// Theme token definitions — used to generate CSS custom properties

export interface Theme {
  name: string;
  colors: Record<string, string>;
  spacing: Record<string, string>;
  fonts: Record<string, string>;
}

export const darkTheme: Theme = {
  name: 'dark',
  colors: {
    bg: '#1e1e1e',
    bgPanel: '#252525',
    bgHeader: '#2d2d2d',
    bgInput: '#333333',
    border: '#404040',
    borderFocus: '#007acc',
    text: '#cccccc',
    textMuted: '#999999',
    textDim: '#666666',
    accent: '#007acc',
    accentHover: '#1a8ad4',
    success: '#4caf50',
    warning: '#ff9800',
    error: '#f44336',
  },
  spacing: {
    xs: '4px',
    sm: '8px',
    md: '12px',
    lg: '16px',
    xl: '24px',
  },
  fonts: {
    body: "system-ui, -apple-system, 'Segoe UI', sans-serif",
    mono: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    sizeXs: '11px',
    sizeSm: '12px',
    sizeMd: '13px',
    sizeLg: '14px',
  },
};

export const lightTheme: Theme = {
  name: 'light',
  colors: {
    bg: '#f5f5f5',
    bgPanel: '#ffffff',
    bgHeader: '#e8e8e8',
    bgInput: '#ffffff',
    border: '#d0d0d0',
    borderFocus: '#007acc',
    text: '#333333',
    textMuted: '#666666',
    textDim: '#999999',
    accent: '#007acc',
    accentHover: '#005a9e',
    success: '#388e3c',
    warning: '#f57c00',
    error: '#d32f2f',
  },
  spacing: darkTheme.spacing,
  fonts: darkTheme.fonts,
};

export const themes: Record<string, Theme> = {
  dark: darkTheme,
  light: lightTheme,
};

/** Apply a theme by setting CSS custom properties on :root */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.colors)) {
    root.style.setProperty(`--color-${key}`, value);
  }
  for (const [key, value] of Object.entries(theme.spacing)) {
    root.style.setProperty(`--spacing-${key}`, value);
  }
  for (const [key, value] of Object.entries(theme.fonts)) {
    root.style.setProperty(`--font-${key}`, value);
  }
}
