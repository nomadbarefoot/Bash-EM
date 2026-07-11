export type ThemeTokens = {
  "--bg": string;
  "--text": string;
  "--accent": string;
  "--border"?: string;
  "--border-lighter"?: string;
  "--subtitle"?: string;
  "--link"?: string;
};

export type ThemePreset = {
  id: string;
  name: string;
  tokens: ThemeTokens;
};

export const themePresets: ThemePreset[] = [
  {
    id: "light",
    name: "Light",
    tokens: {
      "--bg": "#f4ede3",
      "--text": "#4a3421",
      "--accent": "#ea6735",
      "--border": "#e5dac9",
      "--border-lighter": "#f0e9de",
      "--subtitle": "#626262",
      "--link": "#446464",
    },
  },
  {
    id: "dark",
    name: "Dark",
    tokens: {
      "--bg": "#202727",
      "--text": "#e0ddd8",
      "--accent": "#E46680",
      "--border": "rgba(255,255,255,0.12)",
      "--border-lighter": "#2a3232",
      "--subtitle": "#9a9590",
      "--link": "#8ab4b4",
    },
  },
];

export const defaultThemeId = "light";

export const themeFonts = {
  sans: '"Questrial", Helvetica, Arial, sans-serif',
} as const;
