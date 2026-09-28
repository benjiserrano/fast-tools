/**
 * Transformaciones de texto puras que se ejecutan en el WebView.
 *
 * Aquí solo entra lo que responde al instante y no toca disco: el viaje de ida
 * y vuelta por IPC se notaría al teclear. Todo lo demás vive en Rust.
 */

// ── Base64 ────────────────────────────────────────────────────────────────

/**
 * `btoa` solo acepta bytes, así que el texto se codifica a UTF-8 antes. Sin
 * esto, cualquier carácter fuera de Latin-1 —una eñe, un emoji— lanza una
 * excepción en vez de codificarse.
 */
export function encodeBase64(text: string, urlSafe = false): string {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  const encoded = btoa(binary);
  return urlSafe
    ? encoded.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "")
    : encoded;
}

export function decodeBase64(encoded: string): string {
  // Acepta las dos variantes y tolera que falte el relleno, que es lo habitual
  // en tokens copiados de una cabecera HTTP.
  const normalized = encoded.trim().replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized.padEnd(
    normalized.length + ((4 - (normalized.length % 4)) % 4),
    "=",
  );

  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

// ── URL ───────────────────────────────────────────────────────────────────

export type UrlMode = "component" | "full" | "form";

export function encodeUrl(text: string, mode: UrlMode): string {
  switch (mode) {
    case "component":
      return encodeURIComponent(text);
    case "full":
      return encodeURI(text);
    case "form":
      // application/x-www-form-urlencoded codifica el espacio como «+».
      return encodeURIComponent(text).replace(/%20/g, "+");
  }
}

export function decodeUrl(text: string, mode: UrlMode): string {
  const prepared = mode === "form" ? text.replace(/\+/g, " ") : text;
  return mode === "full" ? decodeURI(prepared) : decodeURIComponent(prepared);
}

// ── Escapado ──────────────────────────────────────────────────────────────

export type EscapeTarget = "html" | "json" | "regex" | "shell";

const HTML_ENTITIES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

export function escapeText(text: string, target: EscapeTarget): string {
  switch (target) {
    case "html":
      // El «&» va primero o se escaparían dos veces las entidades ya escritas.
      return text.replace(/[&<>"']/g, (char) => HTML_ENTITIES[char] ?? char);
    case "json":
      // Se quitan las comillas exteriores que añade JSON.stringify: lo que se
      // quiere es el contenido escapado, listo para pegar dentro de una cadena.
      return JSON.stringify(text).slice(1, -1);
    case "regex":
      return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    case "shell":
      // Comillas simples de POSIX: dentro no se interpreta nada. La propia
      // comilla simple se cierra, se escapa y se vuelve a abrir.
      return `'${text.replace(/'/g, `'\\''`)}'`;
  }
}

export function unescapeText(text: string, target: EscapeTarget): string {
  switch (target) {
    case "html": {
      const reversed = Object.entries(HTML_ENTITIES).map(
        ([char, entity]) => [entity, char] as const,
      );
      let result = text;
      // El «&amp;» se deshace el último, por la misma razón inversa.
      for (const [entity, char] of reversed.reverse()) {
        result = result.split(entity).join(char);
      }
      return result;
    }
    case "json":
      return JSON.parse(`"${text}"`) as string;
    case "regex":
      return text.replace(/\\([.*+?^${}()|[\]\\])/g, "$1");
    case "shell": {
      const trimmed = text.trim();
      if (trimmed.startsWith("'") && trimmed.endsWith("'")) {
        return trimmed.slice(1, -1).split(`'\\''`).join("'");
      }
      return text;
    }
  }
}

// ── Mayúsculas y minúsculas ───────────────────────────────────────────────

export type CaseStyle =
  | "camel"
  | "pascal"
  | "snake"
  | "constant"
  | "kebab"
  | "title"
  | "sentence"
  | "lower"
  | "upper";

export const CASE_LABELS: Record<CaseStyle, string> = {
  camel: "camelCase",
  pascal: "PascalCase",
  snake: "snake_case",
  constant: "CONSTANT_CASE",
  kebab: "kebab-case",
  title: "Título",
  sentence: "Frase",
  lower: "minúsculas",
  upper: "MAYÚSCULAS",
};

/**
 * Parte un identificador en palabras, entendiendo los tres estilos habituales:
 * separadores explícitos, cambio de minúscula a mayúscula, y el final de una
 * sigla seguida de palabra (`HTTPServer` → `HTTP` + `Server`).
 */
export function splitWords(text: string): string[] {
  return text
    .replace(/([a-z0-9])([A-ZÁÉÍÓÚÑ])/g, "$1 $2")
    .replace(/([A-ZÁÉÍÓÚÑ]+)([A-ZÁÉÍÓÚÑ][a-z])/g, "$1 $2")
    .split(/[\s_\-.]+/)
    .filter((word) => word.length > 0);
}

export function convertCase(text: string, style: CaseStyle): string {
  if (style === "lower") return text.toLowerCase();
  if (style === "upper") return text.toUpperCase();

  // Se procesa línea a línea: una lista de identificadores es el caso normal,
  // y unirlos todos en uno solo no sería útil.
  return text
    .split("\n")
    .map((line) => convertLine(line, style))
    .join("\n");
}

function convertLine(line: string, style: CaseStyle): string {
  const words = splitWords(line);
  if (words.length === 0) return line;

  const lower = words.map((word) => word.toLowerCase());
  const capitalize = (word: string) =>
    word.charAt(0).toUpperCase() + word.slice(1);

  switch (style) {
    case "camel":
      return lower.map((word, index) => (index === 0 ? word : capitalize(word))).join("");
    case "pascal":
      return lower.map(capitalize).join("");
    case "snake":
      return lower.join("_");
    case "constant":
      return lower.join("_").toUpperCase();
    case "kebab":
      return lower.join("-");
    case "title":
      return lower.map(capitalize).join(" ");
    case "sentence":
      return capitalize(lower.join(" "));
    default:
      return line;
  }
}

// ── Operaciones por línea ─────────────────────────────────────────────────

export interface LineOptions {
  trim: boolean;
  removeEmpty: boolean;
  deduplicate: boolean;
  sort: "none" | "asc" | "desc" | "length";
  reverse: boolean;
  number: boolean;
  /** Deja solo las líneas que contienen este texto; vacío no filtra. */
  filter: string;
  /** Invierte el filtro: deja las que NO lo contienen. */
  invertFilter: boolean;
  caseSensitive: boolean;
}

export const DEFAULT_LINE_OPTIONS: LineOptions = {
  trim: false,
  removeEmpty: false,
  deduplicate: false,
  sort: "none",
  reverse: false,
  number: false,
  filter: "",
  invertFilter: false,
  caseSensitive: true,
};

export function transformLines(text: string, options: LineOptions): string {
  let lines = text.split("\n");

  if (options.trim) lines = lines.map((line) => line.trim());

  if (options.filter) {
    const needle = options.caseSensitive
      ? options.filter
      : options.filter.toLowerCase();
    lines = lines.filter((line) => {
      const haystack = options.caseSensitive ? line : line.toLowerCase();
      const contains = haystack.includes(needle);
      return options.invertFilter ? !contains : contains;
    });
  }

  if (options.removeEmpty) lines = lines.filter((line) => line.trim() !== "");

  if (options.deduplicate) {
    const seen = new Set<string>();
    lines = lines.filter((line) => {
      const key = options.caseSensitive ? line : line.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  if (options.sort !== "none") {
    // `localeCompare` ordena la eñe y los acentos donde el usuario espera
    // verlos, a diferencia de comparar por punto de código.
    const collator = new Intl.Collator("es", {
      sensitivity: options.caseSensitive ? "variant" : "base",
      numeric: true,
    });
    lines = [...lines].sort((a, b) => {
      if (options.sort === "length") return a.length - b.length;
      const order = collator.compare(a, b);
      return options.sort === "desc" ? -order : order;
    });
  }

  if (options.reverse) lines = [...lines].reverse();

  if (options.number) {
    const width = String(lines.length).length;
    lines = lines.map((line, index) => `${String(index + 1).padStart(width)}  ${line}`);
  }

  return lines.join("\n");
}

// ── Estadísticas ──────────────────────────────────────────────────────────

export interface TextStats {
  characters: number;
  charactersNoSpaces: number;
  /** Caracteres percibidos: un emoji compuesto cuenta como uno. */
  graphemes: number;
  words: number;
  lines: number;
  nonEmptyLines: number;
  paragraphs: number;
  bytes: number;
  /** Minutos de lectura a 200 palabras por minuto. */
  readingMinutes: number;
}

export function computeStats(text: string): TextStats {
  const words = text.split(/\s+/).filter((word) => word.length > 0);
  const lines = text.split("\n");

  return {
    // `length` cuenta unidades UTF-16, así que un emoji cuenta dos. Se da
    // también el recuento por grafemas, que es lo que el usuario ve.
    characters: text.length,
    charactersNoSpaces: text.replace(/\s/g, "").length,
    graphemes: countGraphemes(text),
    words: words.length,
    lines: text === "" ? 0 : lines.length,
    nonEmptyLines: lines.filter((line) => line.trim() !== "").length,
    paragraphs: text.split(/\n\s*\n/).filter((block) => block.trim() !== "").length,
    bytes: new TextEncoder().encode(text).length,
    readingMinutes: Math.ceil(words.length / 200),
  };
}

function countGraphemes(text: string): number {
  if (typeof Intl.Segmenter === "function") {
    return [...new Intl.Segmenter("es", { granularity: "grapheme" }).segment(text)]
      .length;
  }
  // Sin `Intl.Segmenter`, los puntos de código son mejor aproximación que
  // las unidades UTF-16.
  return [...text].length;
}

// ── Lorem ipsum ───────────────────────────────────────────────────────────

const LOREM_WORDS =
  `lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor
   incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud
   exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis aute
   irure in reprehenderit voluptate velit esse cillum eu fugiat nulla pariatur
   excepteur sint occaecat cupidatat non proident sunt culpa qui officia deserunt
   mollit anim id est laborum`
    .split(/\s+/)
    .filter(Boolean);

export type LoremUnit = "words" | "sentences" | "paragraphs";

export function generateLorem(
  count: number,
  unit: LoremUnit,
  startClassic: boolean,
): string {
  const pick = () => LOREM_WORDS[Math.floor(Math.random() * LOREM_WORDS.length)]!;

  const sentence = () => {
    const length = 6 + Math.floor(Math.random() * 10);
    const words = Array.from({ length }, pick);
    const text = words.join(" ");
    return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
  };

  let output: string;
  switch (unit) {
    case "words":
      output = Array.from({ length: count }, pick).join(" ");
      break;
    case "sentences":
      output = Array.from({ length: count }, sentence).join(" ");
      break;
    case "paragraphs":
      output = Array.from({ length: count }, () =>
        Array.from({ length: 3 + Math.floor(Math.random() * 3) }, sentence).join(" "),
      ).join("\n\n");
      break;
  }

  if (startClassic && unit !== "words") {
    output = output.replace(/^[^.]*\./, "Lorem ipsum dolor sit amet, consectetur adipiscing elit.");
  }
  return output;
}
