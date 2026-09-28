// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FormatInfo } from "../../lib/convert";

const FORMATS: FormatInfo[] = [
  {
    id: "json",
    label: "JSON",
    extensions: ["json"],
    defaultExtension: "json",
    binary: false,
    tabular: false,
  },
  {
    id: "yaml",
    label: "YAML",
    extensions: ["yaml", "yml"],
    defaultExtension: "yaml",
    binary: false,
    tabular: false,
  },
  {
    id: "csv",
    label: "CSV",
    extensions: ["csv"],
    defaultExtension: "csv",
    binary: false,
    tabular: true,
  },
  {
    id: "xlsx",
    label: "Excel (XLSX)",
    extensions: ["xlsx"],
    defaultExtension: "xlsx",
    binary: true,
    tabular: true,
  },
];

const mocks = vi.hoisted(() => ({
  listFormats: vi.fn(),
  convertText: vi.fn(),
  detectFormat: vi.fn(),
  loadTextFile: vi.fn(),
  saveTextFile: vi.fn(),
}));

vi.mock("../../lib/convert", async (importOriginal) => {
  const original = await importOriginal<typeof import("../../lib/convert")>();
  return {
    ...original,
    listFormats: mocks.listFormats,
    convertText: mocks.convertText,
    detectFormat: mocks.detectFormat,
    loadTextFile: mocks.loadTextFile,
    saveTextFile: mocks.saveTextFile,
  };
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

// CodeMirror necesita APIs de layout que jsdom no implementa; se sustituye por
// un textarea, que es suficiente para comprobar el cableado del panel.
vi.mock("../../components/Editor", () => ({
  Editor: ({
    value,
    onChange,
    readOnly,
  }: {
    value: string;
    onChange?: (value: string) => void;
    readOnly?: boolean;
  }) => (
    <textarea
      aria-label={readOnly ? "salida" : "entrada"}
      value={value}
      readOnly={readOnly}
      onChange={(event) => onChange?.(event.target.value)}
    />
  ),
}));

beforeEach(() => {
  mocks.listFormats.mockResolvedValue(FORMATS);
  mocks.detectFormat.mockResolvedValue("json");
  mocks.convertText.mockResolvedValue({ output: "- nombre: Ana\n", bytes: 15 });
});

afterEach(cleanup);

async function renderPanel() {
  const { ConvertDataPanel } = await import("./ConvertDataPanel");
  render(<ConvertDataPanel />);
  await waitFor(() => expect(mocks.listFormats).toHaveBeenCalled());
}

describe("panel de conversión de datos", () => {
  it("convierte la entrada inicial y muestra el resultado", async () => {
    await renderPanel();

    await waitFor(() => expect(mocks.convertText).toHaveBeenCalled());
    const salida = await screen.findByLabelText("salida");
    expect((salida as HTMLTextAreaElement).value).toBe("- nombre: Ana\n");
  });

  it("no ofrece formatos binarios en un panel de texto", async () => {
    await renderPanel();

    // XLSX no puede ni leerse ni escribirse como texto: va por el panel de lotes.
    await waitFor(() => expect(screen.getAllByText("JSON").length).toBeGreaterThan(0));
    expect(screen.queryByText("Excel (XLSX)")).toBeNull();
  });

  it("muestra el formato detectado junto a la opción automática", async () => {
    await renderPanel();

    expect(await screen.findByText(/Auto · JSON/)).toBeDefined();
  });

  it("enseña el error del motor en vez de una salida vacía y silenciosa", async () => {
    mocks.convertText.mockRejectedValue("JSON inválido: expected value at line 1");

    await renderPanel();

    expect(await screen.findByText(/JSON inválido/)).toBeDefined();
  });

  it("avisa cuando no se reconoce el formato de entrada", async () => {
    mocks.detectFormat.mockResolvedValue(null);

    await renderPanel();

    expect(await screen.findByText(/No se reconoce el formato/)).toBeDefined();
    expect(mocks.convertText).not.toHaveBeenCalled();
  });
});
