import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import type { Category, Runtime, ToolStatus } from "./types";
import { CATEGORY_DOT, CATEGORY_LABEL, CATEGORY_ORDER } from "./types";

/**
 * El catálogo vive en Rust. Estos tests leen `registry.rs` directamente para
 * que añadir una categoría allí y olvidarse de la etiqueta aquí rompa la
 * suite en vez de renderizar `undefined` en la barra lateral.
 */

interface RustEntry {
  id: string;
  category: Category;
  runtime: Runtime;
  status: ToolStatus;
  phase: number;
}

const ENTRY_PATTERN =
  /^\s*"([a-z0-9-]+)"\s+"[^"]*"\s+\[(\w+)\s+(\w+)\s+(\w+)\s+(\d+)\]/gm;

function toCamel(variant: string): string {
  return variant.charAt(0).toLowerCase() + variant.slice(1);
}

function readRustCatalog(): RustEntry[] {
  const source = readFileSync("src-tauri/src/registry.rs", "utf8");
  const entries: RustEntry[] = [];
  for (const match of source.matchAll(ENTRY_PATTERN)) {
    const [, id, category, runtime, status, phase] = match;
    entries.push({
      id: id!,
      category: toCamel(category!) as Category,
      runtime: toCamel(runtime!) as Runtime,
      status: toCamel(status!) as ToolStatus,
      phase: Number(phase),
    });
  }
  return entries;
}

describe("contrato con registry.rs", () => {
  const catalog = readRustCatalog();

  it("encuentra el catálogo", () => {
    expect(catalog.length).toBeGreaterThan(20);
  });

  it("cada categoría usada en Rust tiene etiqueta, color y orden", () => {
    for (const entry of catalog) {
      expect(CATEGORY_LABEL[entry.category], entry.id).toBeTruthy();
      expect(CATEGORY_DOT[entry.category], entry.id).toBeTruthy();
      expect(CATEGORY_ORDER, entry.id).toContain(entry.category);
    }
  });

  it("los valores de runtime y estado son los esperados", () => {
    for (const entry of catalog) {
      expect(["web", "native"], entry.id).toContain(entry.runtime);
      expect(["ready", "planned"], entry.id).toContain(entry.status);
      expect(entry.phase, entry.id).toBeGreaterThanOrEqual(1);
      expect(entry.phase, entry.id).toBeLessThanOrEqual(6);
    }
  });

  it("toda herramienta marcada como lista tiene panel registrado", async () => {
    // Sin esto se puede marcar una herramienta como Ready en Rust y olvidar el
    // panel: la barra lateral la ofrecería y al pulsarla saldría «Fase N».
    const { PANELS } = await import("../tools/panels");

    const listas = catalog.filter((entry) => entry.status === "ready");
    expect(listas.length).toBeGreaterThan(0);

    const sinPanel = listas.filter((entry) => !PANELS[entry.id]);
    expect(sinPanel.map((entry) => entry.id)).toEqual([]);
  });

  it("todo panel registrado corresponde a una herramienta del catálogo", async () => {
    const { PANELS } = await import("../tools/panels");
    const conocidos = new Set(catalog.map((entry) => entry.id));

    const huerfanos = Object.keys(PANELS).filter((id) => !conocidos.has(id));
    expect(huerfanos).toEqual([]);
  });

  it("CATEGORY_ORDER no tiene duplicados y cubre las etiquetas", () => {
    expect(new Set(CATEGORY_ORDER).size).toBe(CATEGORY_ORDER.length);
    expect([...CATEGORY_ORDER].sort()).toEqual(
      Object.keys(CATEGORY_LABEL).sort(),
    );
  });
});
