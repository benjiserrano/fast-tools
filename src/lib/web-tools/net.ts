/** Inspección de URL, marcas de tiempo y referencia de códigos HTTP. */

// ── URL ───────────────────────────────────────────────────────────────────

export interface UrlParts {
  protocol: string;
  username: string;
  password: string;
  host: string;
  port: string;
  path: string;
  query: Array<[string, string]>;
  hash: string;
  origin: string;
}

export interface UrlAnalysis {
  parts: UrlParts | null;
  error: string | null;
  warnings: string[];
}

export function analyzeUrl(input: string): UrlAnalysis {
  const text = input.trim();
  if (!text) return { parts: null, error: null, warnings: [] };

  let url: URL;
  try {
    url = new URL(text);
  } catch {
    return {
      parts: null,
      // El fallo más común es olvidar el esquema, así que se dice directamente.
      error: text.includes("://")
        ? "No se reconoce esa dirección."
        : "Falta el esquema. Prueba con https:// delante.",
      warnings: [],
    };
  }

  const warnings: string[] = [];
  if (url.protocol === "http:") {
    warnings.push("Va sin cifrar: cualquiera en la red puede leer el tráfico.");
  }
  if (url.username || url.password) {
    warnings.push(
      "Lleva credenciales en la propia dirección. Quedan en el historial y en " +
        "los registros del servidor, y muchos navegadores ya las ignoran.",
    );
  }
  // `new URL()` convierte los dominios internacionalizados a Punycode, así que
  // el host ya no tiene caracteres no ASCII: lo que queda es el prefijo
  // «xn--». Es la señal de un dominio que en pantalla puede imitar a otro,
  // como «раypal» con una «р» cirílica.
  if (url.hostname.split(".").some((label) => label.startsWith("xn--"))) {
    warnings.push(
      `El dominio usa caracteres no latinos y el navegador lo traduce a «${url.hostname}». ` +
        "Puede imitar visualmente a otro dominio; comprueba a dónde lleva de verdad.",
    );
  }

  return {
    parts: {
      protocol: url.protocol.replace(":", ""),
      username: url.username,
      password: url.password,
      host: url.hostname,
      port: url.port,
      path: url.pathname,
      query: [...url.searchParams.entries()],
      hash: url.hash.replace("#", ""),
      origin: url.origin,
    },
    error: null,
    warnings,
  };
}

/** Reconstruye una dirección a partir de sus partes. */
export function buildUrl(parts: UrlParts): string {
  const url = new URL(`${parts.protocol || "https"}://${parts.host || "ejemplo.test"}`);
  if (parts.port) url.port = parts.port;
  if (parts.username) url.username = parts.username;
  if (parts.password) url.password = parts.password;
  url.pathname = parts.path || "/";
  for (const [key, value] of parts.query) {
    if (key) url.searchParams.append(key, value);
  }
  if (parts.hash) url.hash = parts.hash;
  return url.toString();
}

// ── Marcas de tiempo ──────────────────────────────────────────────────────

export interface TimestampView {
  unixSeconds: number;
  unixMillis: number;
  iso: string;
  utc: string;
  local: string;
  relative: string;
  weekday: string;
  /** Nombre de la zona horaria del sistema. */
  timeZone: string;
}

/**
 * Interpreta casi cualquier forma de escribir un instante.
 *
 * Los números se tratan como Unix. Distinguir segundos de milisegundos por el
 * número de dígitos es una heurística, pero acierta para cualquier fecha entre
 * 1973 y 5138, que cubre todo lo que alguien va a pegar aquí.
 */
export function parseTimestamp(input: string): Date | null {
  const text = input.trim();
  if (!text) return null;

  if (/^-?\d+$/.test(text)) {
    const value = Number(text);
    if (!Number.isFinite(value)) return null;
    const asDate = Math.abs(value) > 1e11 ? new Date(value) : new Date(value * 1000);
    return Number.isNaN(asDate.getTime()) ? null : asDate;
  }

  const parsed = new Date(text);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

export function describeTimestamp(date: Date): TimestampView {
  const millis = date.getTime();
  return {
    unixSeconds: Math.floor(millis / 1000),
    unixMillis: millis,
    iso: date.toISOString(),
    utc: date.toUTCString(),
    local: date.toLocaleString("es", { dateStyle: "full", timeStyle: "medium" }),
    relative: relativeTime(millis - Date.now()),
    weekday: date.toLocaleDateString("es", { weekday: "long" }),
    timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  };
}

function relativeTime(deltaMillis: number): string {
  const formatter = new Intl.RelativeTimeFormat("es", { numeric: "auto" });
  const seconds = Math.round(deltaMillis / 1000);
  const units: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["year", 31_536_000],
    ["month", 2_592_000],
    ["day", 86_400],
    ["hour", 3_600],
    ["minute", 60],
  ];

  for (const [unit, size] of units) {
    if (Math.abs(seconds) >= size) {
      return formatter.format(Math.round(seconds / size), unit);
    }
  }
  return formatter.format(seconds, "second");
}

// ── Códigos de estado HTTP ────────────────────────────────────────────────

export interface StatusCode {
  code: number;
  name: string;
  meaning: string;
}

export const HTTP_STATUS: StatusCode[] = [
  { code: 100, name: "Continue", meaning: "Sigue enviando el cuerpo de la petición." },
  { code: 101, name: "Switching Protocols", meaning: "Cambio de protocolo aceptado, por ejemplo a WebSocket." },

  { code: 200, name: "OK", meaning: "Todo bien." },
  { code: 201, name: "Created", meaning: "Se ha creado el recurso. La cabecera Location dice dónde." },
  { code: 202, name: "Accepted", meaning: "Aceptado para procesar después. Aún no se ha hecho." },
  { code: 204, name: "No Content", meaning: "Correcto y sin cuerpo que devolver." },
  { code: 206, name: "Partial Content", meaning: "Trozo del recurso, en respuesta a una petición con rango." },

  { code: 301, name: "Moved Permanently", meaning: "Cambió de sitio para siempre. Actualiza el enlace." },
  { code: 302, name: "Found", meaning: "Está en otro sitio ahora mismo. El original sigue siendo válido." },
  { code: 304, name: "Not Modified", meaning: "No ha cambiado desde tu copia en caché. No se envía cuerpo." },
  { code: 307, name: "Temporary Redirect", meaning: "Como el 302 pero conservando el método." },
  { code: 308, name: "Permanent Redirect", meaning: "Como el 301 pero conservando el método." },

  { code: 400, name: "Bad Request", meaning: "La petición está mal formada. El error es del cliente." },
  { code: 401, name: "Unauthorized", meaning: "Falta autenticarse. Mal nombre: significa «no identificado»." },
  { code: 403, name: "Forbidden", meaning: "Identificado pero sin permiso. Autenticarse otra vez no ayuda." },
  { code: 404, name: "Not Found", meaning: "No existe, o no se quiere admitir que existe." },
  { code: 405, name: "Method Not Allowed", meaning: "El recurso existe pero no acepta ese método." },
  { code: 409, name: "Conflict", meaning: "Choca con el estado actual, por ejemplo una edición simultánea." },
  { code: 410, name: "Gone", meaning: "Existió y se eliminó a propósito. No volverá." },
  { code: 413, name: "Payload Too Large", meaning: "El cuerpo enviado supera el límite del servidor." },
  { code: 415, name: "Unsupported Media Type", meaning: "El Content-Type enviado no se acepta." },
  { code: 418, name: "I'm a teapot", meaning: "Broma del RFC 2324. Algunos servicios lo usan para rechazar bots." },
  { code: 422, name: "Unprocessable Content", meaning: "La sintaxis es correcta pero los datos no son válidos." },
  { code: 429, name: "Too Many Requests", meaning: "Has superado el límite. Mira la cabecera Retry-After." },

  { code: 500, name: "Internal Server Error", meaning: "Algo falló en el servidor y no supo decir qué." },
  { code: 501, name: "Not Implemented", meaning: "El servidor no sabe hacer lo que se le pide." },
  { code: 502, name: "Bad Gateway", meaning: "Un intermediario recibió una respuesta inválida de quien está detrás." },
  { code: 503, name: "Service Unavailable", meaning: "Caído o saturado, en principio de forma temporal." },
  { code: 504, name: "Gateway Timeout", meaning: "Un intermediario se cansó de esperar al servidor de detrás." },
];

export function statusFamily(code: number): {
  label: string;
  tone: "info" | "good" | "warn" | "error";
} {
  if (code < 200) return { label: "Informativo", tone: "info" };
  if (code < 300) return { label: "Correcto", tone: "good" };
  if (code < 400) return { label: "Redirección", tone: "info" };
  if (code < 500) return { label: "Error del cliente", tone: "warn" };
  return { label: "Error del servidor", tone: "error" };
}
