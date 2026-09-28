import { getCurrentWindow } from "@tauri-apps/api/window";

function runWindowAction(action: () => Promise<void>, label: string) {
  void action().catch((error: unknown) => {
    console.error(`[Fast tools] No se pudo ${label}:`, error);
  });
}

export function TitleBar() {
  return (
    <header className="titlebar">
      <div className="titlebar-brand" data-tauri-drag-region>
        <span className="titlebar-mark" aria-hidden="true">/</span>
        <span>Fast tools</span>
      </div>

      <div className="titlebar-controls">
        <button
          className="titlebar-button"
          type="button"
          aria-label="Minimizar"
          title="Minimizar"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={() =>
            runWindowAction(
              () => getCurrentWindow().minimize(),
              "minimizar la ventana",
            )
          }
        >
          <span className="titlebar-glyph titlebar-glyph-minimize" aria-hidden="true" />
        </button>
        <button
          className="titlebar-button"
          type="button"
          aria-label="Maximizar o restaurar"
          title="Maximizar o restaurar"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={() =>
            runWindowAction(
              () => getCurrentWindow().toggleMaximize(),
              "maximizar o restaurar la ventana",
            )
          }
        >
          <span className="titlebar-glyph titlebar-glyph-maximize" aria-hidden="true" />
        </button>
        <button
          className="titlebar-button titlebar-close"
          type="button"
          aria-label="Cerrar"
          title="Cerrar"
          onMouseDown={(event) => event.stopPropagation()}
          onClick={() =>
            runWindowAction(
              () => getCurrentWindow().close(),
              "cerrar la ventana",
            )
          }
        >
          <span className="titlebar-glyph titlebar-glyph-close" aria-hidden="true" />
        </button>
      </div>
    </header>
  );
}
