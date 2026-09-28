import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";

import {
  DropZone,
  ImagePreview,
  imageDetail,
  useImageSource,
} from "../../components/ImagePicker";
import { FieldRow, ToolShell } from "../../components/Panels";
import { Banner, Button, Toggle } from "../../components/ui";
import type { ExifReport } from "../../lib/images";
import { formatBytes, readExif, stripExif } from "../../lib/images";
import { ipcErrorMessage } from "../../lib/ipc";

export function ExifPanel() {
  const { image, error: sourceError, dragging, pick } = useImageSource();
  const [report, setReport] = useState<ExifReport | null>(null);
  const [onlySensitive, setOnlySensitive] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!image) {
      setReport(null);
      return;
    }
    let cancelled = false;
    readExif(image.path)
      .then((value) => {
        if (cancelled) return;
        setReport(value);
        setError(null);
      })
      .catch((cause) => {
        if (!cancelled) setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [image]);

  const handleStrip = useCallback(async () => {
    if (!image) return;
    const base = image.path.split(/[\\/]/).pop() ?? "imagen";
    const dot = base.lastIndexOf(".");
    const suggestion =
      dot > 0 ? `${base.slice(0, dot)}-limpia${base.slice(dot)}` : `${base}-limpia`;

    try {
      const destination = await save({ defaultPath: suggestion });
      if (!destination) return;

      const result = await stripExif(image.path, destination, true);
      setNotice(
        `Quitados ${formatBytes(result.removedBytes)} de metadatos.` +
          (result.recompressed
            ? " La imagen se ha vuelto a comprimir, así que pierde algo de calidad."
            : " El flujo de imagen no se ha tocado: cero pérdida de calidad."),
      );
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, [image]);

  const fields = report
    ? onlySensitive
      ? report.fields.filter((field) => field.sensitive)
      : report.fields
    : [];

  const sensitiveCount = report?.fields.filter((field) => field.sensitive).length ?? 0;

  return (
    <ToolShell
      error={error ?? sourceError}
      notice={notice}
      toolbar={
        <>
          {report && report.fields.length > 0 && (
            <Toggle
              label={`Solo lo delicado (${sensitiveCount})`}
              title="Campos que revelan lugar, fecha, equipo o autoría"
              checked={onlySensitive}
              onChange={setOnlySensitive}
            />
          )}
          <div className="ml-auto flex items-center gap-2">
            {image && <Button onClick={pick}>Cambiar imagen</Button>}
            <Button
              variant="primary"
              onClick={handleStrip}
              disabled={!image || !report || report.fields.length === 0}
            >
              Guardar sin metadatos
            </Button>
          </div>
        </>
      }
    >
      <div className="flex h-full flex-col gap-3 p-4">
        {report?.warnings.map((warning) => (
          <Banner key={warning} tone={report.location ? "error" : "warn"}>
            {warning}
          </Banner>
        ))}

        {!image ? (
          <DropZone dragging={dragging} onPick={pick}>
            <p className="text-xs text-mist-500">
              Las fotos de móvil suelen llevar coordenadas GPS
            </p>
          </DropZone>
        ) : (
          <div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
            <ImagePreview
              source={image.preview}
              label={image.label}
              detail={imageDetail(image)}
            />

            <div className="flex min-h-0 flex-col">
              <h3 className="px-1 pb-2 text-[11px] uppercase tracking-wider text-mist-500">
                Metadatos {report && `· ${report.fields.length}`}
              </h3>
              <div className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-ink-800">
                {report?.location && (
                  <FieldRow label="Ubicación" value={report.location} tone="bad" />
                )}
                {fields.map((field, index) => (
                  <FieldRow
                    key={`${field.tag}-${index}`}
                    label={field.tag}
                    value={field.value}
                    tone={field.sensitive ? "bad" : "normal"}
                  />
                ))}
                {fields.length === 0 && !report?.location && (
                  <p className="p-4 text-xs text-mist-500">
                    {report?.fields.length === 0
                      ? "Esta imagen no lleva metadatos: ya está limpia."
                      : "Ningún campo coincide con el filtro."}
                  </p>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </ToolShell>
  );
}
