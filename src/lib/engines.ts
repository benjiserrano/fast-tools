/** Contrato IPC de los motores externos. Espejo de `commands/engines.rs`. */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type EngineSource = "managed" | "manual" | "system";

export interface EngineStatus {
  id: string;
  name: string;
  description: string;
  enables: string;
  available: boolean;
  source: EngineSource | null;
  path: string | null;
  version: string | null;
  diskBytes: number | null;
  downloadable: boolean;
  downloadSize: number | null;
  downloadVersion: string | null;
  downloadOrigin: string | null;
  license: string | null;
  noDownloadReason: string | null;
}

export interface ProgressEvent {
  engine: string;
  downloaded: number;
  total: number;
}

export interface VerifyEngineResult {
  ok: boolean;
  message: string;
}

export interface DocFormatInfo {
  id: string;
  label: string;
  extensions: string[];
  defaultExtension: string;
  canRead: boolean;
  canWrite: boolean;
  engine: string;
  limitation: string | null;
}

export interface MediaFormatInfo {
  id: string;
  label: string;
  audioOnly: boolean;
}

export interface MediaOptions {
  audioBitrate: number;
  videoQuality: number;
  audioOnly: boolean;
  startSeconds: number | null;
  durationSeconds: number | null;
}

export const DEFAULT_MEDIA_OPTIONS: MediaOptions = {
  audioBitrate: 192,
  videoQuality: 23,
  audioOnly: false,
  startSeconds: null,
  durationSeconds: null,
};

export interface FileOutcome {
  source: string;
  output: string | null;
  error: string | null;
}

export const SOURCE_LABEL: Record<EngineSource, string> = {
  managed: "Instalado por fast-tools",
  manual: "Copiado a mano",
  system: "Instalado en el sistema",
};

export const listEngines = () => invoke<EngineStatus[]>("list_engines");
export const installEngine = (id: string) =>
  invoke<EngineStatus>("install_engine", { id });
export const removeEngine = (id: string) =>
  invoke<EngineStatus>("remove_engine", { id });
export const verifyEngine = (id: string) =>
  invoke<VerifyEngineResult>("verify_engine", { id });

export const listDocFormats = () => invoke<DocFormatInfo[]>("list_doc_formats");
export const listMediaFormats = () =>
  invoke<MediaFormatInfo[]>("list_media_formats");

export const convertDocuments = (
  paths: string[],
  to: string,
  outputDir: string | null,
  overwrite: boolean,
) =>
  invoke<FileOutcome[]>("convert_documents", {
    request: { paths, to, outputDir, overwrite },
  });

export const convertMediaFiles = (
  paths: string[],
  to: string,
  outputDir: string | null,
  overwrite: boolean,
  options: MediaOptions,
) =>
  invoke<FileOutcome[]>("convert_media_files", {
    request: { paths, to, outputDir, overwrite, options },
  });

/** Suscribe a los avisos de progreso de descarga. */
export function onEngineProgress(handler: (event: ProgressEvent) => void) {
  return listen<ProgressEvent>("engine://progress", (event) =>
    handler(event.payload),
  );
}
