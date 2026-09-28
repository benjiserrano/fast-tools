import { describe, expect, it } from "vitest";

import {
  analyzeUrl,
  buildUrl,
  describeTimestamp,
  HTTP_STATUS,
  parseTimestamp,
  statusFamily,
} from "./net";

describe("inspección de URL", () => {
  it("descompone una dirección completa", () => {
    const { parts } = analyzeUrl(
      "https://usuario:clave@api.ejemplo.test:8443/v1/pedidos?estado=abierto&pagina=2#detalle",
    );

    expect(parts).not.toBeNull();
    expect(parts!.protocol).toBe("https");
    expect(parts!.host).toBe("api.ejemplo.test");
    expect(parts!.port).toBe("8443");
    expect(parts!.path).toBe("/v1/pedidos");
    expect(parts!.hash).toBe("detalle");
    expect(parts!.query).toEqual([
      ["estado", "abierto"],
      ["pagina", "2"],
    ]);
  });

  it("conserva los parámetros repetidos en vez de quedarse con el último", () => {
    const { parts } = analyzeUrl("https://x.test/?id=1&id=2&id=3");
    expect(parts!.query).toHaveLength(3);
  });

  it("decodifica los valores escapados", () => {
    const { parts } = analyzeUrl("https://x.test/?q=ma%C3%B1ana%20y%20tarde");
    expect(parts!.query[0]).toEqual(["q", "mañana y tarde"]);
  });

  it("dice que falta el esquema cuando es eso lo que pasa", () => {
    const { error } = analyzeUrl("ejemplo.test/ruta");
    expect(error).toContain("Falta el esquema");
  });

  it("avisa del tráfico sin cifrar", () => {
    const { warnings } = analyzeUrl("http://ejemplo.test");
    expect(warnings.some((w) => w.includes("sin cifrar"))).toBe(true);
  });

  it("avisa de las credenciales incrustadas", () => {
    const { warnings } = analyzeUrl("https://ana:secreta@ejemplo.test");
    expect(warnings.some((w) => w.includes("credenciales"))).toBe(true);
  });

  it("avisa de dominios que imitan a otros con caracteres no latinos", () => {
    // «а» cirílica en lugar de la latina: se ve igual y es otro dominio.
    const { warnings } = analyzeUrl("https://pаypal.test");
    expect(warnings.some((w) => w.includes("no latinos"))).toBe(true);
    // Y enseña a qué se traduce de verdad, que es el dato útil.
    expect(warnings.some((w) => w.includes("xn--"))).toBe(true);
  });

  it("una dirección vacía no es un error", () => {
    const result = analyzeUrl("   ");
    expect(result.error).toBeNull();
    expect(result.parts).toBeNull();
  });

  it("reconstruye lo que ha descompuesto", () => {
    const original = "https://api.ejemplo.test:8443/v1/x?a=1&b=2#z";
    const { parts } = analyzeUrl(original);
    expect(buildUrl(parts!)).toBe(original);
  });
});

describe("marcas de tiempo", () => {
  it("distingue segundos de milisegundos", () => {
    const enSegundos = parseTimestamp("1700000000")!;
    const enMillis = parseTimestamp("1700000000000")!;
    expect(enSegundos.getTime()).toBe(enMillis.getTime());
  });

  it("acepta ISO 8601 y texto de fecha", () => {
    const iso = parseTimestamp("2026-03-14T15:09:26Z")!;
    expect(iso.getUTCFullYear()).toBe(2026);
    expect(iso.getUTCMonth()).toBe(2);
    expect(iso.getUTCDate()).toBe(14);
  });

  it("acepta marcas anteriores a 1970", () => {
    const anterior = parseTimestamp("-86400")!;
    expect(anterior.getUTCFullYear()).toBe(1969);
  });

  it("devuelve nulo en vez de una fecha inventada", () => {
    expect(parseTimestamp("no es una fecha")).toBeNull();
    expect(parseTimestamp("")).toBeNull();
  });

  it("describe el instante en todas las formas", () => {
    const view = describeTimestamp(new Date("2026-03-14T15:09:26.000Z"));

    expect(view.unixSeconds).toBe(1773500966);
    expect(view.unixMillis).toBe(1773500966000);
    expect(view.iso).toBe("2026-03-14T15:09:26.000Z");
    expect(view.utc).toContain("GMT");
    expect(view.weekday).toBeTruthy();
    expect(view.timeZone).toBeTruthy();
  });

  it("el tiempo relativo distingue pasado de futuro", () => {
    const pasado = describeTimestamp(new Date(Date.now() - 7_200_000));
    const futuro = describeTimestamp(new Date(Date.now() + 7_200_000));

    expect(pasado.relative).toContain("hace");
    expect(futuro.relative).toContain("dentro de");
  });

  it("va y vuelve por Unix sin perder el instante", () => {
    const original = new Date("2026-09-22T09:53:20.000Z");
    const view = describeTimestamp(original);
    expect(parseTimestamp(String(view.unixSeconds))!.getTime()).toBe(
      original.getTime(),
    );
  });
});

describe("códigos HTTP", () => {
  it("cubre los códigos que más se consultan", () => {
    const codes = HTTP_STATUS.map((status) => status.code);
    for (const expected of [200, 201, 301, 401, 403, 404, 422, 429, 500, 502, 503]) {
      expect(codes).toContain(expected);
    }
  });

  it("no tiene códigos duplicados", () => {
    const codes = HTTP_STATUS.map((status) => status.code);
    expect(new Set(codes).size).toBe(codes.length);
  });

  it("todos tienen nombre y explicación", () => {
    for (const status of HTTP_STATUS) {
      expect(status.name.length).toBeGreaterThan(0);
      expect(status.meaning.endsWith(".")).toBe(true);
    }
  });

  it("clasifica por familia", () => {
    expect(statusFamily(100).label).toBe("Informativo");
    expect(statusFamily(204).tone).toBe("good");
    expect(statusFamily(301).label).toBe("Redirección");
    expect(statusFamily(404).tone).toBe("warn");
    expect(statusFamily(503).tone).toBe("error");
  });

  it("distingue el 401 del 403, que es la confusión habitual", () => {
    const unauthorized = HTTP_STATUS.find((s) => s.code === 401)!;
    const forbidden = HTTP_STATUS.find((s) => s.code === 403)!;

    expect(unauthorized.meaning).toContain("autenticarse");
    expect(forbidden.meaning).toContain("permiso");
  });
});
