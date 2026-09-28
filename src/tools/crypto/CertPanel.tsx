import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { FieldRow, Pane, ToolShell } from "../../components/Panels";
import { Banner, Button } from "../../components/ui";
import { loadTextFile } from "../../lib/convert";
import { ipcErrorMessage } from "../../lib/ipc";

interface CertificateInfo {
  subject: string;
  issuer: string;
  serial: string;
  version: number;
  signatureAlgorithm: string;
  publicKeyAlgorithm: string;
  notBefore: string;
  notAfter: string;
  daysRemaining: number;
  alternativeNames: string[];
  isCa: boolean;
  selfSigned: boolean;
  sha256Fingerprint: string;
  sha1Fingerprint: string;
  warnings: string[];
}

export function CertPanel() {
  const [input, setInput] = useState("");
  const [info, setInfo] = useState<CertificateInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!input.trim()) {
      setInfo(null);
      setError(null);
      return;
    }
    let cancelled = false;
    invoke<CertificateInfo>("inspect_certificate", { input })
      .then((value) => {
        if (cancelled) return;
        setInfo(value);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setInfo(null);
        setError(ipcErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [input]);

  const pickFile = useCallback(async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "Certificados", extensions: ["pem", "crt", "cer"] }],
    });
    if (typeof path !== "string") return;
    try {
      setInput((await loadTextFile(path)).text);
    } catch (cause) {
      setError(ipcErrorMessage(cause));
    }
  }, []);

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <span className="text-[11px] text-mist-500">
            Solo lectura: no se comprueba la cadena de confianza ni la revocación.
          </span>
          <div className="ml-auto">
            <Button onClick={pickFile}>Abrir archivo</Button>
          </div>
        </>
      }
    >
      <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
        <Pane label="Certificado PEM">
          <Editor
            value={input}
            format="json"
            onChange={setInput}
            placeholder="-----BEGIN CERTIFICATE-----"
          />
        </Pane>

        <Pane label="Detalles">
          <div className="space-y-3 overflow-y-auto p-4">
            {!info && !error && (
              <p className="text-xs text-mist-500">
                Pega un certificado en formato PEM o abre un archivo.
              </p>
            )}

            {info && (
              <>
                {info.warnings.map((warning) => (
                  <Banner key={warning} tone="warn">
                    {warning}
                  </Banner>
                ))}

                <div className="rounded-lg border border-ink-800">
                  <FieldRow label="Sujeto" value={info.subject} />
                  <FieldRow
                    label="Emisor"
                    value={info.issuer}
                    tone={info.selfSigned ? "muted" : "normal"}
                  />
                  <FieldRow
                    label="Nombres (SAN)"
                    value={
                      info.alternativeNames.length > 0
                        ? info.alternativeNames.join(", ")
                        : "ninguno"
                    }
                  />
                  <FieldRow label="Válido desde" value={info.notBefore} />
                  <FieldRow
                    label="Válido hasta"
                    value={`${info.notAfter} (${info.daysRemaining} días)`}
                    tone={
                      info.daysRemaining < 0
                        ? "bad"
                        : info.daysRemaining < 30
                          ? "bad"
                          : "good"
                    }
                  />
                  <FieldRow label="Número de serie" value={info.serial} />
                  <FieldRow label="Versión" value={`v${info.version}`} tone="muted" />
                  <FieldRow
                    label="Algoritmo de firma"
                    value={info.signatureAlgorithm}
                    tone="muted"
                  />
                  <FieldRow
                    label="Clave pública"
                    value={info.publicKeyAlgorithm}
                    tone="muted"
                  />
                  <FieldRow
                    label="Autoridad (CA)"
                    value={info.isCa ? "sí" : "no"}
                    tone="muted"
                  />
                  <FieldRow label="Huella SHA-256" value={info.sha256Fingerprint} />
                  <FieldRow
                    label="Huella SHA-1"
                    value={info.sha1Fingerprint}
                    tone="muted"
                  />
                </div>
              </>
            )}
          </div>
        </Pane>
      </div>
    </ToolShell>
  );
}
