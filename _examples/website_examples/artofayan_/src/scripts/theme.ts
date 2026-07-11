/**
 * Client-side theme switcher. Apply preset to :root, persist choice, support custom presets.
 * Built-in presets are in src/config/theme.ts; we inject a serialized list from the layout.
 */

export type ThemePresetPayload = { id: string; name: string; tokens: Record<string, string> };

const STORAGE_KEY = "artofayan-theme";

function applyTokens(tokens: Record<string, string>): void {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(tokens)) {
    root.style.setProperty(key, value);
  }
}

export function applyTheme(preset: ThemePresetPayload): void {
  applyTokens(preset.tokens);
}

export function getSavedThemeId(): string | null {
  if (typeof localStorage === "undefined") return null;
  return localStorage.getItem(STORAGE_KEY);
}

export function saveTheme(id: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    // ignore
  }
}

/** Presets: built-in list + optional custom from localStorage. */
export function getPresets(builtIn: ThemePresetPayload[]): ThemePresetPayload[] {
  const custom = getCustomPreset();
  return custom ? [...builtIn, custom] : builtIn;
}

export function applyThemeById(id: string, presets: ThemePresetPayload[]): boolean {
  const preset = presets.find((p) => p.id === id);
  if (!preset) return false;
  applyTheme(preset);
  saveTheme(id);
  return true;
}

/** Initialize: apply saved or default theme. Call with presets from config. */
export function initTheme(
  builtInPresets: ThemePresetPayload[],
  defaultId: string
): void {
  const presets = getPresets(builtInPresets);
  const saved = getSavedThemeId();
  const id = saved && presets.some((p) => p.id === saved) ? saved : defaultId;
  applyThemeById(id, presets);
}

/** Custom preset stored in localStorage (single slot). */
const CUSTOM_PRESET_KEY = "artofayan-theme-custom";

export function getCustomPreset(): ThemePresetPayload | null {
  try {
    const raw = localStorage.getItem(CUSTOM_PRESET_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as ThemePresetPayload;
  } catch {
    return null;
  }
}

export function saveCustomPreset(preset: ThemePresetPayload): void {
  try {
    localStorage.setItem(CUSTOM_PRESET_KEY, JSON.stringify(preset));
  } catch {
    // ignore
  }
}

export function clearCustomPreset(): void {
  try {
    localStorage.removeItem(CUSTOM_PRESET_KEY);
  } catch {
    // ignore
  }
}
