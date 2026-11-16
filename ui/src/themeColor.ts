export type Hsv = { h: number; s: number; v: number };

export function hexToRgb(hex: string): [number, number, number] | null {
  const match = /^#?([0-9a-f]{6})$/i.exec(hex);
  if (!match) return null;
  const value = Number.parseInt(match[1]!, 16);
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}

export function rgbToHex(red: number, green: number, blue: number): string {
  const channel = (value: number) =>
    Math.max(0, Math.min(255, Math.round(value)))
      .toString(16)
      .padStart(2, "0");
  return `#${channel(red)}${channel(green)}${channel(blue)}`;
}

/** HSV with h in degrees and s/v in 0..1. Null for anything but a six-digit hex colour. */
export function hexToHsv(hex: string): Hsv | null {
  const rgb = hexToRgb(hex);
  if (!rgb) return null;
  const [r, g, b] = rgb.map((value) => value / 255) as [number, number, number];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  let h = 0;
  if (delta > 0) {
    if (max === r) h = 60 * (((g - b) / delta) % 6);
    else if (max === g) h = 60 * ((b - r) / delta + 2);
    else h = 60 * ((r - g) / delta + 4);
  }
  if (h < 0) h += 360;
  return { h, s: max === 0 ? 0 : delta / max, v: max };
}

export function hsvToRgb(h: number, s: number, v: number): [number, number, number] {
  const hue = ((h % 360) + 360) % 360;
  const saturation = Math.max(0, Math.min(1, s));
  const value = Math.max(0, Math.min(1, v));
  const chroma = value * saturation;
  const section = hue / 60;
  const x = chroma * (1 - Math.abs((section % 2) - 1));
  let red: number;
  let green: number;
  let blue: number;
  if (section < 1) [red, green, blue] = [chroma, x, 0];
  else if (section < 2) [red, green, blue] = [x, chroma, 0];
  else if (section < 3) [red, green, blue] = [0, chroma, x];
  else if (section < 4) [red, green, blue] = [0, x, chroma];
  else if (section < 5) [red, green, blue] = [x, 0, chroma];
  else [red, green, blue] = [chroma, 0, x];
  const match = value - chroma;
  return [Math.round((red + match) * 255), Math.round((green + match) * 255), Math.round((blue + match) * 255)];
}

export function hsvToHex(h: number, s: number, v: number): string {
  const [red, green, blue] = hsvToRgb(h, s, v);
  return rgbToHex(red, green, blue);
}
