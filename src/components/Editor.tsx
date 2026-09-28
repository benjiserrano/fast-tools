import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { yaml } from "@codemirror/lang-yaml";
import { oneDark } from "@codemirror/theme-one-dark";
import { EditorView } from "@codemirror/view";
import CodeMirror from "@uiw/react-codemirror";
import { useMemo } from "react";

import type { FormatId } from "../lib/convert";

/**
 * TOML, CSV y TSV no llevan resaltado: sus paquetes de lenguaje añadirían peso
 * al binario portable a cambio de muy poco —son formatos planos donde el color
 * apenas ayuda a leer—. Se editan igual, solo que en un color.
 */
function languageFor(format: FormatId) {
  switch (format) {
    case "json":
      return [json()];
    case "yaml":
      return [yaml()];
    case "xml":
      return [xml()];
    default:
      return [];
  }
}

const baseTheme = EditorView.theme({
  "&": { height: "100%", fontSize: "13px" },
  ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.6" },
  "&.cm-focused": { outline: "none" },
});

export function Editor({
  value,
  format,
  onChange,
  readOnly = false,
  placeholder,
}: {
  value: string;
  format: FormatId;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  placeholder?: string;
}) {
  const extensions = useMemo(
    () => [...languageFor(format), baseTheme, EditorView.lineWrapping],
    [format],
  );

  return (
    <div className="selectable h-full overflow-hidden">
      <CodeMirror
        value={value}
        onChange={onChange}
        readOnly={readOnly}
        editable={!readOnly}
        theme={oneDark}
        extensions={extensions}
        placeholder={placeholder}
        height="100%"
        basicSetup={{
          lineNumbers: true,
          foldGutter: format === "json" || format === "xml",
          highlightActiveLine: !readOnly,
          highlightActiveLineGutter: !readOnly,
          autocompletion: false,
          searchKeymap: true,
        }}
      />
    </div>
  );
}
