export const PALETTE_LIGHT = [
  '#D9534F', '#E08E45', '#D4A24C', '#C9B458',
  '#9CB158', '#6FA76F', '#5A9B96', '#5A93A8',
  '#5A7DA8', '#7363A8', '#9056A0', '#A85A8C'
] as const;

export const PALETTE_DARK = [
  '#E8716D', '#F0A562', '#E8B968', '#DDC872',
  '#B5C972', '#88BF88', '#74B5B0', '#74ADC2',
  '#7497C2', '#8D7DC2', '#A972B8', '#C274A4'
] as const;

export type Theme = 'light' | 'dark';

export function colorForIndex(index: number, theme: Theme): string {
  const palette = theme === 'dark' ? PALETTE_DARK : PALETTE_LIGHT;
  return palette[((index % 12) + 12) % 12];
}
