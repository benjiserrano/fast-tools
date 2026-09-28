/** Estructuras que se repiten en casi todos los paneles de herramientas. */

import { useCallback, useState, type ReactNode } from "react";

import { Editor } from "./Editor";
import { Banner, Button, Toolbar } from "./ui";
import type { FormatId } from "../lib/convert";

export function ToolShell({
  toolbar,
  error,
  notice,
  children,
}: {
  toolbar: ReactNode;
  error?: string | null;
  notice?: string | null;
  children: ReactNode;
}) {
  return (
    <div className="flex h-full flex-col">
      <Toolbar>{toolbar}</Toolbar>
      {(error || notice) && (
        <div className="px-4 pt-3">
          {error ? (
            <Banner tone="error">{error}</Banner>
          ) : (
            <Banner tone="info">{notice}</Banner>
          )}
        </div>
      )}
      <div className="min-h-0 flex-1">{children}</div>
    </div>
  );
}

export function SplitEditors({
  input,
  output,
  inputFormat = "json",
  outputFormat = "json",
  inputLabel = "Entrada",
  outputLabel = "Salida",
  onInputChange,
  placeholder,
}: {
  input: string;
  output: string;
  inputFormat?: FormatId;
  outputFormat?: FormatId;
  inputLabel?: string;
  outputLabel?: string;
  onInputChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
      <Pane label={inputLabel}>
        <Editor
          value={input}
          format={inputFormat}
          onChange={onInputChange}
          placeholder={placeholder}
        />
      </Pane>
      <Pane label={outputLabel}>
        <Editor value={output} format={outputFormat} readOnly />
      </Pane>
    </div>
  );
}

export function Pane({
  label,
  children,
  actions,
}: {
  label: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <section className="flex min-h-0 flex-col bg-ink-950">
      <header className="flex items-center gap-2 px-4 py-1.5">
        <h2 className="text-[11px] uppercase tracking-wider text-mist-500">
          {label}
        </h2>
        {actions && <div className="ml-auto flex items-center gap-2">{actions}</div>}
      </header>
      <div className="min-h-0 flex-1">{children}</div>
    </section>
  );
}

/**
 * Copia al portapapeles con confirmación visual.
 *
 * El aviso importa: sin él no hay forma de distinguir «he copiado» de «he
 * pulsado y no ha pasado nada», y el usuario pega lo que tuviera de antes.
 */
export function useCopy(): {
  copy: (text: string) => void;
  copied: boolean;
} {
  const [copied, setCopied] = useState(false);

  const copy = useCallback((text: string) => {
    if (!text) return;
    navigator.clipboard
      .writeText(text)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      })
      .catch(() => setCopied(false));
  }, []);

  return { copy, copied };
}

export function CopyButton({
  text,
  label = "Copiar",
}: {
  text: string;
  label?: string;
}) {
  const { copy, copied } = useCopy();
  return (
    <Button onClick={() => copy(text)} disabled={!text}>
      {copied ? "Copiado" : label}
    </Button>
  );
}

/** Fila de etiqueta y valor monoespaciado, para resultados tipo ficha. */
export function FieldRow({
  label,
  value,
  tone,
}: {
  label: string;
  value: ReactNode;
  tone?: "normal" | "muted" | "good" | "bad";
}) {
  const valueTone = {
    normal: "text-mist-200",
    muted: "text-mist-500",
    good: "text-bolt-400",
    bad: "text-danger-400",
  }[tone ?? "normal"];

  return (
    <div className="flex gap-3 border-b border-ink-800 px-4 py-2 text-xs last:border-b-0">
      <span className="w-44 shrink-0 text-mist-500">{label}</span>
      <span className={`selectable min-w-0 flex-1 break-all font-mono ${valueTone}`}>
        {value}
      </span>
    </div>
  );
}

export function EmptyHint({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <p className="max-w-md text-center text-sm text-mist-500">{children}</p>
    </div>
  );
}
