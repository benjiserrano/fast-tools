/**
 * Espejo en TypeScript de `src-tauri/src/registry.rs` y `commands.rs`.
 *
 * Rust serializa los enums con `rename_all = "camelCase"`, así que los valores
 * aquí deben coincidir exactamente. El test `contrato.test.ts` comprueba que
 * el catálogo real que devuelve Rust encaja con estos tipos.
 */

export type Category = "convert" | "text" | "crypto" | "net" | "image";

export type Runtime = "web" | "native";

export type ToolStatus = "ready" | "planned";

export interface Tool {
  id: string;
  name: string;
  summary: string;
  category: Category;
  runtime: Runtime;
  status: ToolStatus;
  /** Fase del plan en la que se implementa (1-6). */
  phase: number;
  keywords: string[];
}

export type StorageMode = "portable" | "localAppData" | "temporary";

export interface StorageInfo {
  root: string;
  mode: StorageMode;
  degradedReason: string | null;
}

export interface AppInfo {
  name: string;
  version: string;
  storage: StorageInfo;
}

export const CATEGORY_ORDER: Category[] = [
  "convert",
  "text",
  "crypto",
  "net",
  "image",
];

export const CATEGORY_LABEL: Record<Category, string> = {
  convert: "Conversión",
  text: "Texto y datos",
  crypto: "Cripto e identificadores",
  net: "Web y red",
  image: "Imágenes",
};

/** Color del punto que identifica cada categoría en la barra lateral. */
export const CATEGORY_DOT: Record<Category, string> = {
  convert: "bg-bolt-500",
  text: "bg-emerald-400",
  crypto: "bg-spark-400",
  net: "bg-warn-400",
  image: "bg-rose-400",
};

export const STORAGE_LABEL: Record<StorageMode, string> = {
  portable: "Portable",
  localAppData: "Datos en AppData",
  temporary: "Datos temporales",
};
