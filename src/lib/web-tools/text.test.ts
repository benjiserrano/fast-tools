import { describe, expect, it } from "vitest";

import {
  computeStats,
  convertCase,
  decodeBase64,
  decodeUrl,
  DEFAULT_LINE_OPTIONS,
  encodeBase64,
  encodeUrl,
  escapeText,
  generateLorem,
  splitWords,
  transformLines,
  unescapeText,
} from "./text";

describe("base64", () => {
  it("va y vuelve con texto ASCII", () => {
    expect(encodeBase64("hola")).toBe("aG9sYQ==");
    expect(decodeBase64("aG9sYQ==")).toBe("hola");
  });

  it("no se rompe con acentos ni emoji", () => {
    // `btoa` a secas lanzaría excepción con cualquiera de los dos.
    for (const original of ["mañana", "año 2026 ✓", "👨‍👩‍👧 familia", "日本語"]) {
      expect(decodeBase64(encodeBase64(original))).toBe(original);
    }
  });

  it("produce la variante segura para URL cuando se pide", () => {
    const original = "??>>??>>";
    const estandar = encodeBase64(original);
    const seguro = encodeBase64(original, true);

    expect(seguro).not.toContain("+");
    expect(seguro).not.toContain("/");
    expect(seguro).not.toContain("=");
    expect(decodeBase64(seguro)).toBe(original);
    expect(decodeBase64(estandar)).toBe(original);
  });

  it("tolera que falte el relleno", () => {
    // Los tokens copiados de una cabecera suelen venir sin «=».
    expect(decodeBase64("aG9sYQ")).toBe("hola");
  });

  it("falla con bytes que no son UTF-8 en vez de devolver basura", () => {
    // 0xFF no es una secuencia UTF-8 válida.
    expect(() => decodeBase64("/w==")).toThrow();
  });
});

describe("URL", () => {
  it("distingue componente de URL completa", () => {
    expect(encodeUrl("a/b?c=d", "component")).toBe("a%2Fb%3Fc%3Dd");
    expect(encodeUrl("https://x.test/a b", "full")).toBe("https://x.test/a%20b");
  });

  it("codifica el espacio como «+» en modo formulario", () => {
    expect(encodeUrl("a b", "form")).toBe("a+b");
    expect(decodeUrl("a+b", "form")).toBe("a b");
  });

  it("va y vuelve con caracteres no latinos", () => {
    const original = "búsqueda: ñandú & 日本";
    expect(decodeUrl(encodeUrl(original, "component"), "component")).toBe(original);
  });
});

describe("escapado", () => {
  it("escapa HTML sin duplicar el ampersand", () => {
    expect(escapeText("<a href=\"x\">&</a>", "html")).toBe(
      "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;",
    );
  });

  it("deshace el escapado de HTML en el orden correcto", () => {
    // «&amp;lt;» debe volver a «&lt;», no a «<».
    expect(unescapeText("&amp;lt;", "html")).toBe("&lt;");
    const original = '<a href="x">& \'fin\'</a>';
    expect(unescapeText(escapeText(original, "html"), "html")).toBe(original);
  });

  it("escapa para JSON sin las comillas exteriores", () => {
    expect(escapeText('di "hola"\n', "json")).toBe('di \\"hola\\"\\n');
    expect(unescapeText('di \\"hola\\"\\n', "json")).toBe('di "hola"\n');
  });

  it("escapa los metacaracteres de regex", () => {
    const escapado = escapeText("precio (1+2)*3 [€]", "regex");
    expect(new RegExp(escapado).test("precio (1+2)*3 [€]")).toBe(true);
    expect(unescapeText(escapado, "regex")).toBe("precio (1+2)*3 [€]");
  });

  it("entrecomilla para shell incluso con comillas dentro", () => {
    expect(escapeText("rm -rf /", "shell")).toBe("'rm -rf /'");
    // Una comilla simple se cierra, se escapa y se reabre.
    expect(escapeText("it's", "shell")).toBe(`'it'\\''s'`);
    expect(unescapeText(escapeText("it's", "shell"), "shell")).toBe("it's");
  });
});

describe("mayúsculas y minúsculas", () => {
  it("parte identificadores en palabras", () => {
    expect(splitWords("holaMundo")).toEqual(["hola", "Mundo"]);
    expect(splitWords("hola_mundo-cruel.otra")).toEqual([
      "hola",
      "mundo",
      "cruel",
      "otra",
    ]);
    // Una sigla seguida de palabra se separa donde toca.
    expect(splitWords("HTTPServerError")).toEqual(["HTTP", "Server", "Error"]);
  });

  it("convierte entre los estilos habituales", () => {
    const original = "nombre de usuario";
    expect(convertCase(original, "camel")).toBe("nombreDeUsuario");
    expect(convertCase(original, "pascal")).toBe("NombreDeUsuario");
    expect(convertCase(original, "snake")).toBe("nombre_de_usuario");
    expect(convertCase(original, "constant")).toBe("NOMBRE_DE_USUARIO");
    expect(convertCase(original, "kebab")).toBe("nombre-de-usuario");
    expect(convertCase(original, "title")).toBe("Nombre De Usuario");
    expect(convertCase(original, "sentence")).toBe("Nombre de usuario");
  });

  it("convierte cada línea por separado", () => {
    expect(convertCase("hola mundo\nadios mundo", "camel")).toBe(
      "holaMundo\nadiosMundo",
    );
  });

  it("conserva las líneas vacías", () => {
    expect(convertCase("a\n\nb", "snake")).toBe("a\n\nb");
  });
});

describe("operaciones por línea", () => {
  const options = (overrides: Partial<typeof DEFAULT_LINE_OPTIONS>) => ({
    ...DEFAULT_LINE_OPTIONS,
    ...overrides,
  });

  it("quita duplicados conservando el primero", () => {
    expect(transformLines("b\na\nb\nc\na", options({ deduplicate: true }))).toBe(
      "b\na\nc",
    );
  });

  it("ordena respetando los acentos del español", () => {
    // Por punto de código, «ñ» iría después de «z».
    expect(transformLines("zorro\nñu\nmano", options({ sort: "asc" }))).toBe(
      "mano\nñu\nzorro",
    );
  });

  it("ordena números como números y no como texto", () => {
    expect(transformLines("10\n9\n100", options({ sort: "asc" }))).toBe(
      "9\n10\n100",
    );
  });

  it("filtra y permite invertir el filtro", () => {
    const texto = "error: uno\nok: dos\nerror: tres";
    expect(transformLines(texto, options({ filter: "error" }))).toBe(
      "error: uno\nerror: tres",
    );
    expect(
      transformLines(texto, options({ filter: "error", invertFilter: true })),
    ).toBe("ok: dos");
  });

  it("ignora mayúsculas cuando se le dice", () => {
    expect(
      transformLines("Hola\nhola", options({ deduplicate: true, caseSensitive: false })),
    ).toBe("Hola");
  });

  it("numera alineando a la derecha", () => {
    const salida = transformLines(
      Array.from({ length: 10 }, (_, index) => `l${index}`).join("\n"),
      options({ number: true }),
    );
    // Con diez líneas el ancho es dos, así que la primera lleva un espacio.
    expect(salida.split("\n")[0]).toBe(" 1  l0");
    expect(salida.split("\n")[9]).toBe("10  l9");
  });

  it("aplica las operaciones en un orden útil", () => {
    // Recortar antes de quitar vacías, si no las líneas con espacios se quedan.
    expect(
      transformLines("  \na\n  ", options({ trim: true, removeEmpty: true })),
    ).toBe("a");
  });
});

describe("estadísticas", () => {
  it("cuenta lo básico", () => {
    const stats = computeStats("hola mundo\nsegunda línea");
    expect(stats.words).toBe(4);
    expect(stats.lines).toBe(2);
    expect(stats.nonEmptyLines).toBe(2);
  });

  it("distingue unidades UTF-16 de caracteres percibidos", () => {
    // Un emoji compuesto ocupa varias unidades pero se ve como uno.
    const stats = computeStats("👨‍👩‍👧");
    expect(stats.characters).toBeGreaterThan(1);
    expect(stats.graphemes).toBe(1);
  });

  it("cuenta bytes en UTF-8, no caracteres", () => {
    expect(computeStats("año").bytes).toBe(4);
    expect(computeStats("abc").bytes).toBe(3);
  });

  it("cuenta párrafos separados por línea en blanco", () => {
    expect(computeStats("uno\n\ndos\n\n\ntres").paragraphs).toBe(3);
  });

  it("un texto vacío no tiene líneas", () => {
    const stats = computeStats("");
    expect(stats.lines).toBe(0);
    expect(stats.words).toBe(0);
  });
});

describe("lorem ipsum", () => {
  it("genera la cantidad pedida de palabras", () => {
    expect(generateLorem(12, "words", false).split(" ")).toHaveLength(12);
  });

  it("genera frases terminadas en punto", () => {
    const texto = generateLorem(3, "sentences", false);
    expect(texto.match(/\./g)).toHaveLength(3);
  });

  it("separa los párrafos con línea en blanco", () => {
    expect(generateLorem(3, "paragraphs", false).split("\n\n")).toHaveLength(3);
  });

  it("puede empezar por la frase clásica", () => {
    expect(generateLorem(2, "sentences", true).startsWith("Lorem ipsum dolor sit amet")).toBe(
      true,
    );
  });
});
