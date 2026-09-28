/** Contrato IPC de las herramientas de imagen. Espejo de `commands/images.rs`. */

import { invoke } from "@tauri-apps/api/core";

export type ImageKindId =
  | "png"
  | "jpeg"
  | "webp"
  | "avif"
  | "gif"
  | "bmp"
  | "tiff"
  | "ico"
  | "svg";

export interface ImageFormatInfo {
  id: ImageKindId;
  label: string;
  extensions: string[];
  defaultExtension: string;
  canRead: boolean;
  canWrite: boolean;
  lossy: boolean;
  supportsAlpha: boolean;
  limitation: string | null;
}

export interface LoadedImage {
  path: string;
  kind: ImageKindId;
  label: string;
  width: number;
  height: number;
  bytes: number;
  hasAlpha: boolean;
  preview: string;
}

export type FitMode = "contain" | "cover" | "stretch";

export interface ResizeOptions {
  width: number | null;
  height: number | null;
  fit: FitMode;
  rotate: number;
  flipHorizontal: boolean;
  flipVertical: boolean;
  noUpscale: boolean;
}

export const DEFAULT_RESIZE: ResizeOptions = {
  width: null,
  height: null,
  fit: "contain",
  rotate: 0,
  flipHorizontal: false,
  flipVertical: false,
  noUpscale: true,
};

export interface SaveImageOptions {
  quality: number;
  background: string;
}

export const DEFAULT_SAVE: SaveImageOptions = {
  quality: 85,
  background: "#ffffff",
};

export interface ProcessRequest {
  path: string;
  to: ImageKindId;
  resize: ResizeOptions;
  save: SaveImageOptions;
  svgWidth: number | null;
}

export interface ProcessResult {
  width: number;
  height: number;
  bytes: number;
  originalBytes: number;
  preview: string;
}

export interface PaletteColor {
  hex: string;
  rgb: [number, number, number];
  share: number;
}

export interface ExifField {
  tag: string;
  value: string;
  group: string;
  sensitive: boolean;
}

export interface ExifReport {
  fields: ExifField[];
  location: string | null;
  warnings: string[];
}

export interface StripExifResult {
  destination: string;
  removedBytes: number;
  recompressed: boolean;
}

export type CorrectionId = "low" | "medium" | "quartile" | "high";

export interface CorrectionInfo {
  id: CorrectionId;
  label: string;
  note: string;
}

export interface QrOptions {
  correction: CorrectionId;
  size: number;
  foreground: string;
  background: string;
  margin: number;
}

export const DEFAULT_QR: QrOptions = {
  correction: "medium",
  size: 512,
  foreground: "#000000",
  background: "#ffffff",
  margin: 4,
};

export interface QrResult {
  preview: string;
  svg: string;
  bytes: number;
}

export const listImageFormats = () =>
  invoke<ImageFormatInfo[]>("list_image_formats");

export const loadImage = (path: string, svgWidth: number | null = null) =>
  invoke<LoadedImage>("load_image", { path, svgWidth });

export const processImage = (request: ProcessRequest) =>
  invoke<ProcessResult>("process_image", { request });

export const saveProcessedImage = (
  request: ProcessRequest,
  destination: string,
  overwrite: boolean,
) =>
  invoke<string>("save_processed_image", { request, destination, overwrite });

export const faviconSizes = () => invoke<number[]>("favicon_sizes");

export const generateFavicon = (
  path: string,
  sizes: number[],
  destination: string,
  overwrite: boolean,
) =>
  invoke<string>("generate_favicon", { path, sizes, destination, overwrite });

export const extractPalette = (path: string, count: number) =>
  invoke<PaletteColor[]>("extract_palette", { path, count });

export const readExif = (path: string) => invoke<ExifReport>("read_exif", { path });

export const stripExif = (
  path: string,
  destination: string,
  overwrite: boolean,
) => invoke<StripExifResult>("strip_exif", { path, destination, overwrite });

export const qrCorrectionLevels = () =>
  invoke<CorrectionInfo[]>("qr_correction_levels");

export const generateQr = (content: string, options: QrOptions) =>
  invoke<QrResult>("generate_qr", { content, options });

export const saveQr = (
  content: string,
  options: QrOptions,
  destination: string,
  overwrite: boolean,
) => invoke<string>("save_qr", { content, options, destination, overwrite });

/** Tamaño legible, para comparar el antes y el después. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}
