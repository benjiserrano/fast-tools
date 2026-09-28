import { describe, expect, it } from "vitest";

import {
  contrastRatio,
  describe as describeColor,
  hslToRgb,
  judgeContrast,
  oklchToRgb,
  parseColor,
  parseHex,
  rgbToHsl,
  rgbToOklch,
  toHex,
} from "./color";

const BLACK = { r: 0, g: 0, b: 0 };
const WHITE = { r: 255, g: 255, b: 255 };

describe("HEX", () => {
  it("acepta la forma larga, la corta y sin almohadilla", () => {
    expect(parseHex("#ff8800")).toEqual({ r: 255, g: 136, b: 0 });
    expect(parseHex("ff8800")).toEqual({ r: 255, g: 136, b: 0 });
    expect(parseHex("#f80")).toEqual({ r: 255, g: 136, b: 0 });
  });

  it("rechaza lo que no es hexadecimal", () => {
    expect(parseHex("#12345")).toBeNull();
    expect(parseHex("#gggggg")).toBeNull();
    expect(parseHex("rojo")).toBeNull();
  });

  it("va y vuelve", () => {
    for (const hex of ["#000000", "#ffffff", "#3366cc", "#ff8800"]) {
      expect(toHex(parseHex(hex)!)).toBe(hex);
    }
  });
});

describe("HSL", () => {
  it("convierte los colores primarios donde toca", () => {
    expect(rgbToHsl({ r: 255, g: 0, b: 0 })).toMatchObject({ h: 0, s: 100, l: 50 });
    expect(rgbToHsl({ r: 0, g: 255, b: 0 })).toMatchObject({ h: 120 });
    expect(rgbToHsl({ r: 0, g: 0, b: 255 })).toMatchObject({ h: 240 });
  });

  it("el gris no tiene tono ni saturación", () => {
    const hsl = rgbToHsl({ r: 128, g: 128, b: 128 });
    expect(hsl.s).toBe(0);
    expect(hsl.h).toBe(0);
  });

  it("va y vuelve sin desviarse", () => {
    for (const rgb of [
      { r: 255, g: 136, b: 0 },
      { r: 12, g: 200, b: 90 },
      { r: 50, g: 50, b: 200 },
    ]) {
      const recovered = hslToRgb(rgbToHsl(rgb));
      expect(Math.abs(recovered.r - rgb.r)).toBeLessThanOrEqual(1);
      expect(Math.abs(recovered.g - rgb.g)).toBeLessThanOrEqual(1);
      expect(Math.abs(recovered.b - rgb.b)).toBeLessThanOrEqual(1);
    }
  });
});

describe("OKLCH", () => {
  it("el blanco tiene luminosidad 1 y croma cero", () => {
    const oklch = rgbToOklch(WHITE);
    expect(oklch.l).toBeCloseTo(1, 2);
    expect(oklch.c).toBeCloseTo(0, 2);
  });

  it("el negro tiene luminosidad cero", () => {
    expect(rgbToOklch(BLACK).l).toBeCloseTo(0, 2);
  });

  it("va y vuelve sin desviarse", () => {
    for (const rgb of [
      { r: 255, g: 136, b: 0 },
      { r: 51, g: 102, b: 204 },
      { r: 12, g: 200, b: 90 },
      { r: 128, g: 128, b: 128 },
    ]) {
      const recovered = oklchToRgb(rgbToOklch(rgb));
      expect(Math.abs(recovered.r - rgb.r)).toBeLessThanOrEqual(2);
      expect(Math.abs(recovered.g - rgb.g)).toBeLessThanOrEqual(2);
      expect(Math.abs(recovered.b - rgb.b)).toBeLessThanOrEqual(2);
    }
  });

  it("conserva el tono mejor que HSL al aclarar", () => {
    // Es la razón por la que la herramienta ofrece OKLCH: aclarar un azul en
    // HSL lo vira hacia el violeta, y en OKLCH se queda azul. El tono percibido
    // se mide con OKLCH en ambos casos, que es la escala que corresponde a lo
    // que ve el ojo.
    const rgb = { r: 51, g: 102, b: 204 };
    const base = rgbToOklch(rgb);

    const porOklch = rgbToOklch(oklchToRgb({ ...base, l: base.l + 0.12 }));

    const hsl = rgbToHsl(rgb);
    const porHsl = rgbToOklch(hslToRgb({ ...hsl, l: hsl.l + 18 }));

    const desvioOklch = Math.abs(porOklch.h - base.h);
    const desvioHsl = Math.abs(porHsl.h - base.h);

    expect(desvioOklch).toBeLessThan(desvioHsl);
  });

  it("aclarar un color saturado hasta salirse de gama sí desplaza el tono", () => {
    // Documenta el límite real: fuera de sRGB hay que recortar los canales, y
    // el recorte cambia su proporción. No es un fallo de la conversión.
    const base = rgbToOklch({ r: 51, g: 102, b: 204 });
    const lighter = oklchToRgb({ ...base, l: base.l + 0.25 });

    const clamped = [lighter.r, lighter.g, lighter.b].some(
      (channel) => channel === 0 || channel === 255,
    );
    expect(clamped).toBe(true);
  });
});

describe("contraste WCAG", () => {
  it("blanco sobre negro da el máximo de 21:1", () => {
    expect(contrastRatio(WHITE, BLACK)).toBeCloseTo(21, 1);
  });

  it("un color consigo mismo da 1:1", () => {
    expect(contrastRatio(BLACK, BLACK)).toBeCloseTo(1, 5);
  });

  it("el orden de los colores no cambia el resultado", () => {
    const a = { r: 51, g: 102, b: 204 };
    expect(contrastRatio(a, WHITE)).toBeCloseTo(contrastRatio(WHITE, a), 5);
  });

  it("aplica los umbrales de la norma", () => {
    const maximo = judgeContrast(WHITE, BLACK);
    expect(maximo.aaNormal).toBe(true);
    expect(maximo.aaaNormal).toBe(true);
    expect(maximo.summary).toContain("AAA");

    // Gris medio sobre blanco: no llega ni al mínimo.
    const pobre = judgeContrast({ r: 170, g: 170, b: 170 }, WHITE);
    expect(pobre.aaNormal).toBe(false);
    expect(pobre.summary).toContain("No cumple");
  });
});

describe("notación", () => {
  it("produce las cuatro formas en sintaxis CSS válida", () => {
    const notation = describeColor({ r: 255, g: 136, b: 0 });

    expect(notation.hex).toBe("#ff8800");
    expect(notation.rgb).toBe("rgb(255 136 0)");
    expect(notation.hsl).toMatch(/^hsl\(\d+ \d+% \d+%\)$/);
    expect(notation.oklch).toMatch(/^oklch\([\d.]+% [\d.]+ [\d.]+\)$/);
  });

  it("interpreta cualquiera de las notaciones escritas a mano", () => {
    const esperado = { r: 255, g: 136, b: 0 };

    expect(parseColor("#ff8800")).toEqual(esperado);
    expect(parseColor("rgb(255 136 0)")).toEqual(esperado);
    expect(parseColor("rgb(255, 136, 0)")).toEqual(esperado);

    const desdeHsl = parseColor("hsl(32 100% 50%)")!;
    expect(Math.abs(desdeHsl.r - 255)).toBeLessThanOrEqual(2);

    const desdeOklch = parseColor("oklch(75.5% 0.18 62)")!;
    expect(desdeOklch).not.toBeNull();
    expect(desdeOklch.r).toBeGreaterThan(200);
  });

  it("devuelve nulo en vez de inventarse un color", () => {
    expect(parseColor("no es un color")).toBeNull();
    expect(parseColor("rgb(1 2)")).toBeNull();
    expect(parseColor("")).toBeNull();
  });

  it("la luminosidad de OKLCH admite porcentaje y fracción", () => {
    const conPorcentaje = parseColor("oklch(50% 0.1 180)");
    const conFraccion = parseColor("oklch(0.5 0.1 180)");

    expect(conPorcentaje).toEqual(conFraccion);
  });
});
