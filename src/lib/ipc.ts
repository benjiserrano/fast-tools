/**
 * Única puerta de entrada al núcleo Rust.
 *
 * Ningún componente llama a `invoke` directamente: así el contrato IPC queda
 * tipado en un solo sitio y cambiar un comando rompe la compilación en vez de
 * fallar en tiempo de ejecución.
 */

import { invoke } from "@tauri-apps/api/core";

import type { AppInfo, Tool } from "./types";

/** Falso cuando la UI se abre en un navegador suelto en vez de en la ventana. */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function listTools(): Promise<Tool[]> {
  return invoke<Tool[]>("list_tools");
}

export function getTool(id: string): Promise<Tool> {
  return invoke<Tool>("get_tool", { id });
}

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

export function revealDataDir(): Promise<void> {
  return invoke<void>("reveal_data_dir");
}

/** Normaliza cualquier fallo de IPC a un mensaje mostrable. */
export function ipcErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return JSON.stringify(error);
}
