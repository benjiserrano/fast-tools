/**
 * Conversión entre espacios de color y contraste WCAG.
 *
 * Todo se hace sobre sRGB, que es lo que entienden el navegador y el CSS. OKLCH
 * pasa por OKLab: es perceptualmente uniforme, así que cambiar la luminosidad
 * no arrastra el tono, cosa que sí pasa en HSL.
 */

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export interface Hsl {
  h: number;
  s: number;
  l: number;
}

export interface Oklch {
  l: number;
  c: number;
  h: number;
}

const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

const round = (value: number, decimals = 0) => {
  const factor = 10 ** decimals;
  return Math.round(value * factor) / factor;
};

// ── HEX ───────────────────────────────────────────────────────────────────

export function parseHex(text: string): Rgb | null {
  const hex = text.trim().replace(/^#/, "");
  // Forma corta: #abc equivale a #aabbcc.
  const expanded =
    hex.length === 3
      ? hex
          .split("")
          .map((char) => char + char)
          .join("")
      : hex;

  if (!/^[0-9a-fA-F]{6}$/.test(expanded)) return null;

  return {
    r: parseInt(expanded.slice(0, 2), 16),
    g: parseInt(expanded.slice(2, 4), 16),
    b: parseInt(expanded.slice(4, 6), 16),
  };
}

export function toHex({ r, g, b }: Rgb): string {
  const part = (value: number) =>
    clamp(Math.round(value), 0, 255).toString(16).padStart(2, "0");
  return `#${part(r)}${part(g)}${part(b)}`;
}

// ── HSL ───────────────────────────────────────────────────────────────────

export function rgbToHsl({ r, g, b }: Rgb): Hsl {
  const red = r / 255;
  const green = g / 255;
  const blue = b / 255;

  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  const lightness = (max + min) / 2;

  if (delta === 0) return { h: 0, s: 0, l: lightness * 100 };

  const saturation = delta / (1 - Math.abs(2 * lightness - 1));

  let hue: number;
  if (max === red) hue = ((green - blue) / delta) % 6;
  else if (max === green) hue = (blue - red) / delta + 2;
  else hue = (red - green) / delta + 4;

  hue *= 60;
  if (hue < 0) hue += 360;

  return { h: hue, s: saturation * 100, l: lightness * 100 };
}

export function hslToRgb({ h, s, l }: Hsl): Rgb {
  const hue = ((h % 360) + 360) % 360;
  const saturation = clamp(s, 0, 100) / 100;
  const lightness = clamp(l, 0, 100) / 100;

  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const second = chroma * (1 - Math.abs(((hue / 60) % 2) - 1));
  const offset = lightness - chroma / 2;

  const [red, green, blue] = (() => {
    if (hue < 60) return [chroma, second, 0];
    if (hue < 120) return [second, chroma, 0];
    if (hue < 180) return [0, chroma, second];
    if (hue < 240) return [0, second, chroma];
    if (hue < 300) return [second, 0, chroma];
    return [chroma, 0, second];
  })();

  return {
    r: Math.round((red + offset) * 255),
    g: Math.round((green + offset) * 255),
    b: Math.round((blue + offset) * 255),
  };
}

// ── OKLCH ─────────────────────────────────────────────────────────────────

/** sRGB con corrección gamma a luz lineal. */
function toLinear(channel: number): number {
  const value = channel / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

function fromLinear(value: number): number {
  const channel =
    value <= 0.0031308 ? value * 12.92 : 1.055 * value ** (1 / 2.4) - 0.055;
  return clamp(Math.round(channel * 255), 0, 255);
}

export function rgbToOklch(rgb: Rgb): Oklch {
  const red = toLinear(rgb.r);
  const green = toLinear(rgb.g);
  const blue = toLinear(rgb.b);

  // Matrices de Björn Ottosson para OKLab.
  const long = Math.cbrt(
    0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue,
  );
  const medium = Math.cbrt(
    0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue,
  );
  const short = Math.cbrt(
    0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue,
  );

  const lightness =
    0.2104542553 * long + 0.793617785 * medium - 0.0040720468 * short;
  const a = 1.9779984951 * long - 2.428592205 * medium + 0.4505937099 * short;
  const b = 0.0259040371 * long + 0.7827717662 * medium - 0.808675766 * short;

  const chroma = Math.sqrt(a * a + b * b);
  let hue = (Math.atan2(b, a) * 180) / Math.PI;
  if (hue < 0) hue += 360;

  return { l: lightness, c: chroma, h: chroma < 1e-6 ? 0 : hue };
}

export function oklchToRgb({ l, c, h }: Oklch): Rgb {
  const radians = (h * Math.PI) / 180;
  const a = c * Math.cos(radians);
  const b = c * Math.sin(radians);

  const long = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const medium = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const short = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;

  return {
    r: fromLinear(
      4.0767416621 * long - 3.3077115913 * medium + 0.2309699292 * short,
    ),
    g: fromLinear(
      -1.2684380046 * long + 2.6097574011 * medium - 0.3413193965 * short,
    ),
    b: fromLinear(
      -0.0041960863 * long - 0.7034186147 * medium + 1.707614701 * short,
    ),
  };
}

// ── Contraste WCAG ────────────────────────────────────────────────────────

/** Luminancia relativa según la definición de la WCAG. */
export function relativeLuminance({ r, g, b }: Rgb): number {
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

export function contrastRatio(foreground: Rgb, background: Rgb): number {
  const lighter = Math.max(
    relativeLuminance(foreground),
    relativeLuminance(background),
  );
  const darker = Math.min(
    relativeLuminance(foreground),
    relativeLuminance(background),
  );
  return (lighter + 0.05) / (darker + 0.05);
}

export interface ContrastVerdict {
  ratio: number;
  /** 4.5:1 para texto normal. */
  aaNormal: boolean;
  /** 3:1 para texto grande (18 pt, o 14 pt en negrita). */
  aaLarge: boolean;
  /** 7:1, el nivel exigente. */
  aaaNormal: boolean;
  aaaLarge: boolean;
  summary: string;
}

export function judgeContrast(
  foreground: Rgb,
  background: Rgb,
): ContrastVerdict {
  const ratio = contrastRatio(foreground, background);
  const aaNormal = ratio >= 4.5;
  const aaLarge = ratio >= 3;
  const aaaNormal = ratio >= 7;

  return {
    ratio,
    aaNormal,
    aaLarge,
    aaaNormal,
    aaaLarge: aaNormal,
    summary: aaaNormal
      ? "Cumple AAA: vale para cualquier tamaño de texto."
      : aaNormal
        ? "Cumple AA: vale para texto normal."
        : aaLarge
          ? "Solo vale para texto grande (18 pt o 14 pt en negrita)."
          : "No cumple el mínimo: ilegible para mucha gente.",
  };
}

// ── Formato de salida ─────────────────────────────────────────────────────

export interface ColorNotation {
  hex: string;
  rgb: string;
  hsl: string;
  oklch: string;
}

export function describe(rgb: Rgb): ColorNotation {
  const hsl = rgbToHsl(rgb);
  const oklch = rgbToOklch(rgb);

  return {
    hex: toHex(rgb),
    rgb: `rgb(${Math.round(rgb.r)} ${Math.round(rgb.g)} ${Math.round(rgb.b)})`,
    hsl: `hsl(${round(hsl.h)} ${round(hsl.s)}% ${round(hsl.l)}%)`,
    // La luminosidad de OKLCH se expresa en porcentaje en CSS.
    oklch: `oklch(${round(oklch.l * 100, 1)}% ${round(oklch.c, 3)} ${round(oklch.h, 1)})`,
  };
}

/**
 * Acepta cualquiera de las notaciones que se escriben a mano.
 *
 * No se usa el truco de pintar en un canvas y leer el píxel: en jsdom no hay
 * canvas, y así los tests comprueban la conversión de verdad.
 */
export function parseColor(text: string): Rgb | null {
  const value = text.trim().toLowerCase();

  const hex = parseHex(value);
  if (hex) return hex;

  const numbers = (input: string) =>
    input
      .replace(/[^\d.,%\-/\s]/g, "")
      .split(/[\s,/]+/)
      .filter(Boolean);

  if (value.startsWith("rgb")) {
    const parts = numbers(value).map((part) => parseFloat(part));
    if (parts.length < 3 || parts.some(Number.isNaN)) return null;
    return {
      r: clamp(parts[0]!, 0, 255),
      g: clamp(parts[1]!, 0, 255),
      b: clamp(parts[2]!, 0, 255),
    };
  }

  if (value.startsWith("hsl")) {
    const parts = numbers(value).map((part) => parseFloat(part));
    if (parts.length < 3 || parts.some(Number.isNaN)) return null;
    return hslToRgb({ h: parts[0]!, s: parts[1]!, l: parts[2]! });
  }

  if (value.startsWith("oklch")) {
    const raw = numbers(value);
    const parts = raw.map((part) => parseFloat(part));
    if (parts.length < 3 || parts.some(Number.isNaN)) return null;
    // La luminosidad admite «62%» o «0.62».
    const lightness = raw[0]!.includes("%") ? parts[0]! / 100 : parts[0]!;
    return oklchToRgb({ l: lightness, c: parts[1]!, h: parts[2]! });
  }

  return null;
}
