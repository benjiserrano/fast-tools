/** Reexporta el contrato de motores y añade lo que solo usa esta pantalla. */

export {
  convertDocuments,
  convertMediaFiles,
  DEFAULT_MEDIA_OPTIONS,
  installEngine,
  listDocFormats,
  listEngines,
  listMediaFormats,
  onEngineProgress,
  removeEngine,
  SOURCE_LABEL,
  verifyEngine,
} from "../../lib/engines";

export type {
  DocFormatInfo,
  EngineSource,
  EngineStatus,
  FileOutcome,
  MediaFormatInfo,
  MediaOptions,
  ProgressEvent,
  VerifyEngineResult,
} from "../../lib/engines";

/** Tamaños de motor: siempre en megabytes, que es la escala en la que están. */
export function formatEngineSize(bytes: number): string {
  const megabytes = bytes / (1024 * 1024);
  if (megabytes < 10) return `${megabytes.toFixed(1)} MB`;
  return `${Math.round(megabytes)} MB`;
}
