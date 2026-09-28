import { open, save } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Editor } from "../../components/Editor";
import {
  Banner,
  Button,
  NumberField,
  Select,
  TextField,
  Toggle,
  Toolbar,
} from "../../components/ui";
import type { ConvertOpts, FormatId, FormatInfo } from "../../lib/convert";
import {
  convertText,
  DEFAULT_OPTS,
  detectFormat,
  listFormats,
  loadTextFile,
  saveTextFile,
} from "../../lib/convert";
import { ipcErrorMessage } from "../../lib/ipc";

const AUTO = "auto";
/** Margen para que escribir no dispare una conversión por pulsación. */
const DEBOUNCE_MS = 200;

const EXAMPLE = `[
  { "nombre": "Ana", "edad": 33, "activo": true },
  { "nombre": "Luis", "edad": 41, "activo": false }
]`;

export function ConvertDataPanel() {
  const [formats, setFormats] = useState<FormatInfo[]>([]);
  const [input, setInput] = useState(EXAMPLE);
  const [output, setOutput] = useState("");
  const [from, setFrom] = useState<FormatId | typeof AUTO>(AUTO);
  const [detected, setDetected] = useState<FormatId | null>("json");
  const [to, setTo] = useState<FormatId>("yaml");
  const [opts, setOpts] = useState<ConvertOpts>(DEFAULT_OPTS);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    listFormats().then(setFormats).catch(() => setFormats([]));
  }, []);

  // El panel de texto no puede con formatos binarios: XLSX entra y sale por el
  // panel de lotes, que trabaja con archivos y no carga nada en el editor.
  const textFormats = useMemo(
    () => formats.filter((format) => !format.binary),
    [formats],
  );

  const effectiveFrom: FormatId | null = from === AUTO ? detected : from;

  // Detección automática: solo cuando el usuario no ha elegido origen a mano.
  useEffect(() => {
    if (from !== AUTO) return;
    let cancelled = false;
    detectFormat(null, input.slice(0, 4096))
      .then((format) => {
        if (!cancelled) setDetected(format);
      })
      .catch(() => {
        if (!cancelled) setDetected(null);
      });
    return () => {
      cancelled = true;
    };
  }, [input, from]);

  const requestId = useRef(0);

  useEffect(() => {
    if (!effectiveFrom) {
      setOutput("");
      setError(
        input.trim()
          ? "No se reconoce el formato de entrada. Elígelo en el desplegable «Desde»."
          : null,
      );
      return;
    }
    if (!input.trim()) {
      setOutput("");
      setError(null);
      return;
    }

    const id = ++requestId.current;
    const timer = setTimeout(() => {
      convertText(effectiveFrom, to, input, opts)
        .then((response) => {
          // Descarta respuestas de una entrada ya superada.
          if (id !== requestId.current) return;
          setOutput(response.output);
          setError(null);
        })
        .catch((cause) => {
          if (id !== requestId.current) return;
          setOutput("");
          setError(ipcErrorMessage(cause));
        });
    }, DEBOUNCE_MS);

    return () => clearTimeout(timer);
  }, [input, effectiveFrom, to, opts]);

  const flash = useCallback((message: string) => {
    setNotice(message);
    setTimeout(() => setNotice(null), 2500);
  }, []);

  const handleOpen = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Datos estructurados",
            extensions: textFormats.flatMap((format) => format.extensions),
          },
        ],
      });
      if (typeof selected !== "string") return;

      const file = await loadTextFile(selected);
      setInput(file.text);
      if (file.format) {
        setFrom(file.format);
        setDetected(file.format);
      }
      flash(`Cargado ${selected}`);
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, [textFormats, flash]);

  const handleSave = useCallback(async () => {
    if (!output) return;
    const extension = formats.find((format) => format.id === to)?.defaultExtension ?? to;
    try {
      const destination = await save({
        defaultPath: `convertido.${extension}`,
        filters: [{ name: to.toUpperCase(), extensions: [extension] }],
      });
      if (!destination) return;
      // El diálogo del sistema ya pregunta si el archivo existe, así que aquí
      // no hace falta volver a preguntarlo.
      await saveTextFile(destination, output, true);
      flash(`Guardado en ${destination}`);
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, [output, to, formats, flash]);

  const handleCopy = useCallback(async () => {
    if (!output) return;
    await navigator.clipboard.writeText(output);
    flash("Copiado al portapapeles");
  }, [output, flash]);

  const handleSwap = useCallback(() => {
    if (!effectiveFrom || !output) return;
    setInput(output);
    setFrom(to);
    setDetected(to);
    setTo(effectiveFrom);
  }, [effectiveFrom, output, to]);

  const showSeparator = to === "csv" || to === "tsv";
  const showXmlRoot = to === "xml";
  const showInfer = effectiveFrom
    ? ["csv", "tsv", "xml"].includes(effectiveFrom)
    : false;

  return (
    <div className="flex h-full flex-col">
      <Toolbar>
        <Select
          label="Desde"
          value={from}
          onChange={(event) => setFrom(event.target.value as FormatId | typeof AUTO)}
        >
          <option value={AUTO}>
            Auto{detected ? ` · ${detected.toUpperCase()}` : ""}
          </option>
          {textFormats.map((format) => (
            <option key={format.id} value={format.id}>
              {format.label}
            </option>
          ))}
        </Select>

        <Button onClick={handleSwap} disabled={!output} title="Intercambiar">
          ⇄
        </Button>

        <Select
          label="Hacia"
          value={to}
          onChange={(event) => setTo(event.target.value as FormatId)}
        >
          {textFormats.map((format) => (
            <option key={format.id} value={format.id}>
              {format.label}
            </option>
          ))}
        </Select>

        <div className="h-4 w-px bg-ink-800" />

        <Toggle
          label="Indentar"
          checked={opts.pretty}
          onChange={(pretty) => setOpts((prev) => ({ ...prev, pretty }))}
        />
        {opts.pretty && (
          <NumberField
            label="Espacios"
            value={opts.indent}
            min={1}
            max={8}
            onChange={(indent) => setOpts((prev) => ({ ...prev, indent }))}
          />
        )}
        {showInfer && (
          <Toggle
            label="Deducir tipos"
            title="Convierte «33» en número y «true» en booleano. Los códigos con ceros a la izquierda se respetan como texto."
            checked={opts.inferTypes}
            onChange={(inferTypes) => setOpts((prev) => ({ ...prev, inferTypes }))}
          />
        )}
        {showSeparator && (
          <TextField
            label="Separador"
            title="Une las claves anidadas al aplanar: cliente.direccion.ciudad"
            width="w-12"
            value={opts.separator}
            onChange={(separator) => setOpts((prev) => ({ ...prev, separator }))}
          />
        )}
        {showXmlRoot && (
          <TextField
            label="Raíz"
            title="Nombre del elemento raíz cuando los datos no traen uno"
            value={opts.xmlRoot}
            onChange={(xmlRoot) => setOpts((prev) => ({ ...prev, xmlRoot }))}
          />
        )}

        <div className="ml-auto flex items-center gap-2">
          <Button onClick={handleOpen}>Abrir archivo</Button>
          <Button onClick={handleCopy} disabled={!output}>
            Copiar
          </Button>
          <Button variant="primary" onClick={handleSave} disabled={!output}>
            Guardar como
          </Button>
        </div>
      </Toolbar>

      {(error || notice) && (
        <div className="px-4 pt-3">
          {error ? (
            <Banner tone="error">{error}</Banner>
          ) : (
            <Banner tone="info">{notice}</Banner>
          )}
        </div>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-2 gap-px bg-ink-800">
        <section className="flex min-h-0 flex-col bg-ink-950">
          <h2 className="px-4 py-1.5 text-[11px] uppercase tracking-wider text-mist-500">
            Entrada
          </h2>
          <div className="min-h-0 flex-1">
            <Editor
              value={input}
              format={effectiveFrom ?? "json"}
              onChange={setInput}
              placeholder="Pega aquí tus datos o abre un archivo"
            />
          </div>
        </section>

        <section className="flex min-h-0 flex-col bg-ink-950">
          <h2 className="px-4 py-1.5 text-[11px] uppercase tracking-wider text-mist-500">
            Salida · {to.toUpperCase()}
          </h2>
          <div className="min-h-0 flex-1">
            <Editor value={output} format={to} readOnly />
          </div>
        </section>
      </div>
    </div>
  );
}
