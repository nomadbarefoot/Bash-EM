export const CATEGORY_LABELS: Record<string, string> = {
  environments: "Environments",
  "speed-paintings": "Speed Paintings",
  "plein-air": "Plein Air",
  professional: "Professional Works",
  studies: "Studies",
  sketches: "Sketches",
} as const;

export const CATEGORY_FILTER_VALUES = [
  "all",
  "environments",
  "speed-paintings",
  "studies",
  "professional",
  "sketches",
] as const;

export function getCategoryLabel(value: string): string {
  return CATEGORY_LABELS[value] ?? value;
}
