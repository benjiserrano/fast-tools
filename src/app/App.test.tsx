// @vitest-environment jsdom
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AppInfo, Tool } from "../lib/types";

const TOOLS: Tool[] = [
  {
    id: "convert-data",
    name: "Datos estructurados",
    summary: "CSV, JSON, YAML y más",
    category: "convert",
    runtime: "native",
    status: "planned",
    phase: 1,
    keywords: ["csv", "json"],
  },
  {
    id: "hash-text",
    name: "Hash de texto",
    summary: "MD5, SHA-256 y BLAKE3",
    category: "crypto",
    runtime: "native",
    status: "planned",
    phase: 2,
    keywords: ["hash", "sha256"],
  },
];

const PORTABLE: AppInfo = {
  name: "fast-tools",
  version: "0.1.0",
  storage: {
    root: "D:\\usb\\fast-tools-data",
    mode: "portable",
    degradedReason: null,
  },
};

const mocks = vi.hoisted(() => ({
  inTauri: vi.fn(() => true),
  listTools: vi.fn(),
  appInfo: vi.fn(),
  revealDataDir: vi.fn(),
}));

vi.mock("../lib/ipc", () => ({
  inTauri: mocks.inTauri,
  listTools: mocks.listTools,
  appInfo: mocks.appInfo,
  getTool: vi.fn(),
  revealDataDir: mocks.revealDataDir,
  ipcErrorMessage: (error: unknown) => String(error),
}));

beforeEach(() => {
  mocks.inTauri.mockReturnValue(true);
  mocks.listTools.mockResolvedValue(TOOLS);
  mocks.appInfo.mockResolvedValue(PORTABLE);
});

// Sin los globals de vitest, Testing Library no registra su limpieza
// automática y el DOM de un test se filtra al siguiente.
afterEach(cleanup);

async function renderApp() {
  const { App } = await import("./App");
  render(<App />);
  await waitFor(() => expect(mocks.listTools).toHaveBeenCalled());
}

describe("shell de la aplicación", () => {
  it("lista las herramientas agrupadas por categoría", async () => {
    await renderApp();
    expect(await screen.findByText("Datos estructurados")).toBeDefined();

    // Los nombres de categoría salen también en la rejilla de bienvenida, así
    // que la consulta se acota a la navegación lateral.
    const nav = within(screen.getByRole("navigation"));
    expect(nav.getByText("Conversión")).toBeDefined();
    expect(nav.getByText("Cripto e identificadores")).toBeDefined();
    expect(nav.getByText("Hash de texto")).toBeDefined();

    // Solo aparecen las categorías con herramientas, no las cinco.
    expect(nav.queryByText("Imágenes")).toBeNull();
  });

  it("marca con su fase las herramientas todavía no implementadas", async () => {
    await renderApp();

    expect(await screen.findByText("F1")).toBeDefined();
    expect(screen.getByText("F2")).toBeDefined();
  });

  it("muestra la bienvenida con el recuento de herramientas disponibles", async () => {
    await renderApp();

    expect(
      await screen.findByText(/0 de 2 herramientas disponibles/),
    ).toBeDefined();
  });

  it("la barra de estado informa del modo portable y su ruta", async () => {
    await renderApp();

    expect(await screen.findByText("Portable")).toBeDefined();
    expect(screen.getByText("D:\\usb\\fast-tools-data")).toBeDefined();
  });

  it("avisa cuando el almacenamiento ha degradado a AppData", async () => {
    mocks.appInfo.mockResolvedValue({
      ...PORTABLE,
      storage: {
        root: "C:\\Users\\dev\\AppData\\Local\\fast-tools",
        mode: "localAppData",
        degradedReason: "«E:\\fast-tools-data» no admite escritura: acceso denegado",
      },
    } satisfies AppInfo);

    await renderApp();

    expect(await screen.findByText("Datos en AppData")).toBeDefined();
    expect(screen.getByText(/no admite escritura/)).toBeDefined();
  });

  it("explica el fallo en vez de quedarse en blanco si no hay IPC", async () => {
    mocks.inTauri.mockReturnValue(false);

    const { App } = await import("./App");
    render(<App />);

    expect(
      await screen.findByText(/No se pudo iniciar la interfaz/),
    ).toBeDefined();
    expect(mocks.listTools).not.toHaveBeenCalled();
  });
});
