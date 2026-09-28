import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";

import { CopyButton, ToolShell } from "../../components/Panels";
import { Banner, Button, NumberField, Select, TextField } from "../../components/ui";
import { ipcErrorMessage } from "../../lib/ipc";

interface VerifyResult {
  matches: boolean;
  scheme: string;
}

export function PasswordHashPanel() {
  const [password, setPassword] = useState("");
  const [scheme, setScheme] = useState("argon2id");
  const [cost, setCost] = useState(12);
  const [encoded, setEncoded] = useState("");
  const [verifyAgainst, setVerifyAgainst] = useState("");
  const [verifyResult, setVerifyResult] = useState<VerifyResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  // A propósito no se calcula al teclear: con un coste alto, bcrypt y Argon2
  // tardan cientos de milisegundos y la ventana se quedaría trabada.
  const compute = useCallback(async () => {
    if (!password) {
      setError("Escribe una contraseña.");
      return;
    }
    setRunning(true);
    setError(null);
    try {
      setEncoded(
        await invoke<string>("hash_password", {
          request: {
            password,
            scheme,
            cost: scheme === "bcrypt" ? cost : null,
          },
        }),
      );
    } catch (cause) {
      setError(ipcErrorMessage(cause));
      setEncoded("");
    } finally {
      setRunning(false);
    }
  }, [password, scheme, cost]);

  const verify = useCallback(async () => {
    setError(null);
    try {
      setVerifyResult(
        await invoke<VerifyResult>("verify_password", {
          passwordText: password,
          encoded: verifyAgainst || encoded,
        }),
      );
    } catch (cause) {
      setError(ipcErrorMessage(cause));
      setVerifyResult(null);
    }
  }, [password, verifyAgainst, encoded]);

  return (
    <ToolShell
      error={error}
      toolbar={
        <>
          <Select
            label="Esquema"
            value={scheme}
            onChange={(event) => setScheme(event.target.value)}
          >
            <option value="argon2id">Argon2id</option>
            <option value="bcrypt">bcrypt</option>
          </Select>
          {scheme === "bcrypt" && (
            <NumberField
              label="Coste"
              value={cost}
              min={10}
              max={15}
              onChange={setCost}
            />
          )}
          <div className="ml-auto flex items-center gap-2">
            <Button variant="primary" onClick={compute} disabled={running}>
              {running ? "Calculando…" : "Calcular resumen"}
            </Button>
            <CopyButton text={encoded} />
          </div>
        </>
      }
    >
      <div className="space-y-4 overflow-y-auto p-4">
        <Banner tone="info">
          {scheme === "argon2id"
            ? "Argon2id es la recomendación actual: resiste ataques con GPU y no tiene límite de longitud."
            : "bcrypt solo tiene en cuenta los primeros 72 bytes. Para frases de paso largas, usa Argon2id."}
        </Banner>

        <TextField
          label="Contraseña"
          width="w-[34rem]"
          value={password}
          onChange={(value) => {
            setPassword(value);
            setVerifyResult(null);
          }}
        />

        {encoded && (
          <div>
            <h3 className="mb-1 text-[11px] uppercase tracking-wider text-mist-500">
              Resumen
            </h3>
            <p className="selectable break-all rounded-lg border border-ink-800 bg-ink-900 p-3 font-mono text-xs text-bolt-400">
              {encoded}
            </p>
            <p className="mt-1 text-[11px] text-mist-500">
              Cada cálculo usa una sal distinta, así que el resultado cambia aunque
              la contraseña sea la misma. Es lo correcto.
            </p>
          </div>
        )}

        <div className="border-t border-ink-800 pt-4">
          <h3 className="mb-2 text-[11px] uppercase tracking-wider text-mist-500">
            Comprobar contra un resumen existente
          </h3>
          <div className="flex items-end gap-3">
            <TextField
              label="Resumen"
              width="w-[34rem]"
              title="Pega un resumen de bcrypt o Argon2; si lo dejas vacío se usa el de arriba"
              value={verifyAgainst}
              onChange={(value) => {
                setVerifyAgainst(value);
                setVerifyResult(null);
              }}
            />
            <Button onClick={verify} disabled={!password || (!verifyAgainst && !encoded)}>
              Comprobar
            </Button>
          </div>

          {verifyResult && (
            <div className="mt-3">
              <Banner tone={verifyResult.matches ? "info" : "error"}>
                {verifyResult.matches
                  ? `La contraseña coincide con el resumen ${verifyResult.scheme}.`
                  : `La contraseña NO coincide con el resumen ${verifyResult.scheme}.`}
              </Banner>
            </div>
          )}
        </div>
      </div>
    </ToolShell>
  );
}
