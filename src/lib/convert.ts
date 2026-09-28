/** Contrato IPC del motor de conversión. Espejo de `src-tauri/src/commands/convert.rs`. */

import { invoke } from "@tauri-apps/api/core";

export type FormatId = "json" | "yaml" | "toml" | "xml" | "csv" | "tsv" | "xlsx";

export interface FormatInfo {
  id: FormatId;
  label: string;
  extensions: string[];
  defaultExtension: string;
  binary: boolean;
  tabular: boolean;
}

export interface ConvertOpts {
  pretty: boolean;
  indent: number;
  separator: string;
  inferTypes: boolean;
  xmlRoot: string;
  sheet: string | null;
}

export const DEFAULT_OPTS: ConvertOpts = {
  pretty: true,
  indent: 2,
  separator: ".",
  inferTypes: true,
  xmlRoot: "root",
  sheet: null,
};

export interface TextResponse {
  output: string;
  bytes: number;
}

export interface FileOutcome {
  source: string;
  output: string | null;
  error: string | null;
}

export function listFormats(): Promise<FormatInfo[]> {
  return invoke<FormatInfo[]>("list_formats");
}

export function convertTargets(from: FormatId): Promise<FormatId[]> {
  return invoke<FormatId[]>("convert_targets", { from });
}

export function convertText(
  from: FormatId,
  to: FormatId,
  input: string,
  opts: ConvertOpts,
): Promise<TextResponse> {
  return invoke<TextResponse>("convert_text", {
    request: { from, to, input, opts },
  });
}

export function detectFormat(
  path: string | null,
  sample: string | null,
): Promise<FormatId | null> {
  return invoke<FormatId | null>("detect_format", { path, sample });
}

export interface LoadedFile {
  text: string;
  format: FormatId | null;
  path: string;
}

export function loadTextFile(path: string): Promise<LoadedFile> {
  return invoke<LoadedFile>("load_text_file", { path });
}

export function saveTextFile(
  path: string,
  contents: string,
  overwrite: boolean,
): Promise<void> {
  return invoke<void>("save_text_file", { path, contents, overwrite });
}

export function convertFiles(
  paths: string[],
  to: FormatId,
  outputDir: string | null,
  overwrite: boolean,
  opts: ConvertOpts,
): Promise<FileOutcome[]> {
  return invoke<FileOutcome[]>("convert_files", {
    request: { paths, to, outputDir, overwrite, opts },
  });
}

/** Nombre de archivo sin la carpeta, para mostrarlo en listas. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}
