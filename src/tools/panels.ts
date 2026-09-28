import { lazy, type ComponentType } from "react";

import type { Tool } from "../lib/types";

export interface PanelProps {
  tool: Tool;
}

type Panel = ComponentType<PanelProps>;

/**
 * Carga diferida con el nombre del export. La mayoría de los paneles no usan
 * `export default`, así que hay que desenvolverlos a mano.
 */
function load<M extends Record<string, unknown>>(
  importer: () => Promise<M>,
  name: keyof M,
): Panel {
  return lazy(() =>
    importer().then((module) => ({ default: module[name] as Panel })),
  );
}

/**
 * Punto de extensión: cada fase registra aquí el panel de sus herramientas,
 * con la clave igual al `id` del catálogo de `registry.rs`.
 *
 * Los paneles se cargan bajo demanda. Con decenas de herramientas, meterlas
 * todas en el paquete inicial retrasaría el primer pintado por código que la
 * mayoría de las sesiones no llega a usar.
 *
 * Los identificadores sin entrada muestran el marcador «Fase N», que es lo
 * correcto mientras la herramienta esté planificada y no construida.
 */
export const PANELS: Record<string, Panel | undefined> = {
  // Fase 1 · Conversión
  "convert-data": load(
    () => import("./convert-data/ConvertDataPanel"),
    "ConvertDataPanel",
  ),
  "convert-batch": load(
    () => import("./convert-batch/ConvertBatchPanel"),
    "ConvertBatchPanel",
  ),

  // Fase 2 · Texto y datos
  "json-format": load(() => import("./text/FormatPanel"), "FormatPanel"),
  "yaml-format": load(() => import("./text/FormatPanel"), "FormatPanel"),
  "xml-format": load(() => import("./text/FormatPanel"), "FormatPanel"),
  "sql-format": load(() => import("./text/SqlPanel"), "SqlPanel"),
  "text-diff": load(() => import("./text/DiffPanel"), "DiffPanel"),
  "list-diff": load(() => import("./text/ListDiffPanel"), "ListDiffPanel"),
  base64: load(() => import("./text/TransformPanel"), "TransformPanel"),
  "url-encode": load(() => import("./text/TransformPanel"), "TransformPanel"),
  escape: load(() => import("./text/TransformPanel"), "TransformPanel"),
  "case-convert": load(() => import("./text/TransformPanel"), "CasePanel"),
  "text-stats": load(() => import("./text/StatsPanel"), "StatsPanel"),
  "line-ops": load(() => import("./text/LinesPanel"), "LinesPanel"),
  lorem: load(() => import("./text/LoremPanel"), "LoremPanel"),

  // Fase 2 · Cripto e identificadores
  "hash-text": load(() => import("./crypto/HashPanel"), "HashTextPanel"),
  "hash-file": load(() => import("./crypto/HashPanel"), "HashFilePanel"),
  hmac: load(() => import("./crypto/HashPanel"), "HmacPanel"),
  "uuid-gen": load(() => import("./crypto/IdsPanel"), "IdsPanel"),
  "jwt-decode": load(() => import("./crypto/JwtPanel"), "JwtPanel"),
  "password-hash": load(
    () => import("./crypto/PasswordHashPanel"),
    "PasswordHashPanel",
  ),
  "password-gen": load(() => import("./crypto/IdsPanel"), "PasswordGenPanel"),
  "cert-inspect": load(() => import("./crypto/CertPanel"), "CertPanel"),

  // Fase 3 · Imágenes
  "convert-image": load(() => import("./image/ImagePanel"), "ImagePanel"),
  "image-resize": load(() => import("./image/ImagePanel"), "ImagePanel"),
  "image-compress": load(() => import("./image/ImagePanel"), "ImagePanel"),
  "favicon-gen": load(() => import("./image/FaviconPanel"), "FaviconPanel"),
  "qr-gen": load(() => import("./image/QrPanel"), "QrPanel"),
  "exif-view": load(() => import("./image/ExifPanel"), "ExifPanel"),
  "color-palette": load(() => import("./image/PalettePanel"), "PalettePanel"),
  "color-convert": load(() => import("./image/ColorPanel"), "ColorPanel"),

  // Fase 4 · Motores externos
  engines: load(() => import("./engines/EnginesPanel"), "EnginesPanel"),
  "convert-doc": load(
    () => import("./engines/ExternalBatchPanel"),
    "DocPanel",
  ),
  "convert-media": load(
    () => import("./engines/ExternalBatchPanel"),
    "MediaPanel",
  ),

  // Fase 5 · Web y red
  "regex-test": load(() => import("./net/RegexPanel"), "RegexPanel"),
  "cron-parse": load(() => import("./net/CronPanel"), "CronPanel"),
  "http-client": load(() => import("./net/HttpPanel"), "HttpPanel"),
  "url-inspect": load(() => import("./net/WebPanels"), "UrlPanel"),
  timestamp: load(() => import("./net/WebPanels"), "TimestampPanel"),
  "http-status": load(() => import("./net/WebPanels"), "HttpStatusPanel"),
};
