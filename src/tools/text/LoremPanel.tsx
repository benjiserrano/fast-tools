import { useCallback, useEffect, useState } from "react";

import { Editor } from "../../components/Editor";
import { CopyButton, Pane, ToolShell } from "../../components/Panels";
import { Button, NumberField, Select, Toggle } from "../../components/ui";
import type { LoremUnit } from "../../lib/web-tools/text";
import { generateLorem } from "../../lib/web-tools/text";

export function LoremPanel() {
  const [count, setCount] = useState(3);
  const [unit, setUnit] = useState<LoremUnit>("paragraphs");
  const [startClassic, setStartClassic] = useState(true);
  const [output, setOutput] = useState("");

  const regenerate = useCallback(() => {
    setOutput(generateLorem(count, unit, startClassic));
  }, [count, unit, startClassic]);

  useEffect(regenerate, [regenerate]);

  return (
    <ToolShell
      toolbar={
        <>
          <NumberField
            label="Cantidad"
            value={count}
            min={1}
            max={500}
            onChange={setCount}
          />
          <Select
            label="Unidad"
            value={unit}
            onChange={(event) => setUnit(event.target.value as LoremUnit)}
          >
            <option value="paragraphs">Párrafos</option>
            <option value="sentences">Frases</option>
            <option value="words">Palabras</option>
          </Select>
          <Toggle
            label="Empezar por «Lorem ipsum»"
            checked={startClassic}
            onChange={setStartClassic}
          />
          <div className="ml-auto flex items-center gap-2">
            <Button onClick={regenerate}>Generar otro</Button>
            <CopyButton text={output} />
          </div>
        </>
      }
    >
      <Pane label="Texto generado">
        <Editor value={output} format="json" readOnly />
      </Pane>
    </ToolShell>
  );
}
