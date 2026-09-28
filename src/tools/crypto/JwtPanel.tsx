import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { FieldRow, Pane, ToolShell } from "../../components/Panels";
import { Banner, TextField } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

type SignatureStatus =
  | "valid"
  | "invalid"
  | "unsupported"
  | "notChecked"
  | "unsecured";

interface TokenInfo {
  header: unknown;
  payload: unknown;
  signatureBase64: string;
  algorithm: string | null;
  signature: SignatureStatus;
  signatureDetail: string;
  warnings: string[];
  issuedAt: string | null;
  expiresAt: string | null;
}

const SIGNATURE_LABEL: Record<SignatureStatus, string> = {
  valid: "Firma válida",
  invalid: "Firma incorrecta",
  unsupported: "Firma no comprobada",
  notChecked: "Firma sin comprobar",
  unsecured: "Token sin firma",
};

const SIGNATURE_TONE: Record<SignatureStatus, "error" | "warn" | "info"> = {
  valid: "info",
  invalid: "error",
  unsupported: "warn",
  notChecked: "warn",
  unsecured: "error",
};

export function JwtPanel() {
  const [token, setToken] = useState("");
  const [secret, setSecret] = useState("");
  const [info, setInfo] = useState<TokenInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token.trim()) {
      setInfo(null);
      setError(null);
      return;
    }
    let cancelled = false;
    invoke<TokenInfo>("inspect_jwt", { token, secret: secret || null })
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
  }, [token, secret]);

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <TextField
            label="Secreto (HS256/384/512)"
            width="w-72"
            title="Solo se usa para comprobar la firma; no sale de esta máquina"
            value={secret}
            onChange={setSecret}
          />
          <span className="text-[11px] text-mist-500">
            Cualquiera puede leer el contenido de un JWT: solo la firma demuestra
            que es auténtico.
          </span>
        </>
      }
    >
      <div className="grid h-full grid-cols-2 gap-px bg-ink-800">
        <Pane label="Token">
          <Editor
            value={token}
            format="json"
            onChange={setToken}
            placeholder="Pega aquí el JWT, sin el prefijo «Bearer»"
          />
        </Pane>

        <Pane label="Contenido">
          <div className="space-y-3 overflow-y-auto p-4">
            {!info && (
              <p className="text-xs text-mist-500">
                Pega un token para verlo decodificado.
              </p>
            )}

            {info && (
              <>
                <Banner tone={SIGNATURE_TONE[info.signature]}>
                  <strong>{SIGNATURE_LABEL[info.signature]}</strong> —{" "}
                  {info.signatureDetail}
                </Banner>

                {info.warnings.map((warning) => (
                  <Banner key={warning} tone="warn">
                    {warning}
                  </Banner>
                ))}

                <div className="rounded-lg border border-ink-800">
                  {info.algorithm && (
                    <FieldRow label="Algoritmo" value={info.algorithm} />
                  )}
                  {info.issuedAt && <FieldRow label="Emitido" value={info.issuedAt} />}
                  {info.expiresAt && (
                    <FieldRow label="Caduca" value={info.expiresAt} />
                  )}
                </div>

                <Section title="Cabecera" value={info.header} />
                <Section title="Contenido" value={info.payload} />
              </>
            )}
          </div>
        </Pane>
      </div>
    </ToolShell>
  );
}

function Section({ title, value }: { title: string; value: unknown }) {
  return (
    <div>
      <h3 className="mb-1 text-[11px] uppercase tracking-wider text-mist-500">
        {title}
      </h3>
      <pre className="selectable overflow-x-auto rounded-lg border border-ink-800 bg-ink-900 p-3 font-mono text-xs text-mist-200">
        {JSON.stringify(value, null, 2)}
      </pre>
    </div>
  );
}
