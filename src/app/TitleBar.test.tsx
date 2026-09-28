// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const windowMocks = vi.hoisted(() => ({
  minimize: vi.fn<() => Promise<void>>(),
  toggleMaximize: vi.fn<() => Promise<void>>(),
  close: vi.fn<() => Promise<void>>(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => windowMocks,
}));

import { TitleBar } from "./TitleBar";

beforeEach(() => {
  windowMocks.minimize.mockResolvedValue();
  windowMocks.toggleMaximize.mockResolvedValue();
  windowMocks.close.mockResolvedValue();
});

afterEach(cleanup);

describe("controles de ventana", () => {
  it("ejecuta minimizar, maximizar/restaurar y cerrar", () => {
    render(<TitleBar />);

    fireEvent.click(screen.getByRole("button", { name: "Minimizar" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Maximizar o restaurar" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(windowMocks.minimize).toHaveBeenCalledOnce();
    expect(windowMocks.toggleMaximize).toHaveBeenCalledOnce();
    expect(windowMocks.close).toHaveBeenCalledOnce();
  });
});
