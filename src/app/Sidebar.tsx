import { useMemo } from "react";

import type { Category, Tool } from "../lib/types";
import { CATEGORY_DOT, CATEGORY_LABEL, CATEGORY_ORDER } from "../lib/types";

interface SidebarProps {
  tools: Tool[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onOpenPalette: () => void;
}

export function Sidebar({
  tools,
  selectedId,
  onSelect,
  onOpenPalette,
}: SidebarProps) {
  const grouped = useMemo(() => groupByCategory(tools), [tools]);

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r border-ink-800 bg-ink-900">
      <header className="flex items-center gap-2 px-4 py-3">
        <Bolt />
        <span className="font-semibold tracking-tight text-mist-50">
          Fast tools
        </span>
      </header>

      <button
        type="button"
        onClick={onOpenPalette}
        className="mx-3 mb-3 flex items-center justify-between rounded-md border border-ink-700 bg-ink-850 px-3 py-2 text-left text-sm text-mist-500 transition-colors hover:border-ink-600 hover:text-mist-400"
      >
        <span>Buscar herramienta…</span>
        <kbd className="rounded border border-ink-700 bg-ink-900 px-1.5 py-0.5 font-mono text-[10px] text-mist-500">
          Ctrl Alt T
        </kbd>
      </button>

      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        {CATEGORY_ORDER.map((category) => {
          const items = grouped[category];
          if (items.length === 0) return null;
          return (
            <section key={category} className="mb-4">
              <h2 className="flex items-center gap-2 px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-mist-500">
                <span
                  className={`size-1.5 rounded-full ${CATEGORY_DOT[category]}`}
                />
                {CATEGORY_LABEL[category]}
              </h2>
              <ul>
                {items.map((tool) => (
                  <li key={tool.id}>
                    <button
                      type="button"
                      onClick={() => onSelect(tool.id)}
                      title={tool.summary}
                      className={`flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors ${
                        tool.id === selectedId
                          ? "bg-ink-800 text-mist-50"
                          : "text-mist-400 hover:bg-ink-850 hover:text-mist-200"
                      }`}
                    >
                      <span className="truncate">{tool.name}</span>
                      {tool.status === "planned" && (
                        <span className="shrink-0 rounded border border-ink-700 px-1 font-mono text-[10px] text-mist-500">
                          F{tool.phase}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </nav>
    </aside>
  );
}

function groupByCategory(tools: Tool[]): Record<Category, Tool[]> {
  const grouped = {
    convert: [],
    text: [],
    crypto: [],
    net: [],
    image: [],
  } as Record<Category, Tool[]>;
  for (const tool of tools) grouped[tool.category].push(tool);
  return grouped;
}

function Bolt() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className="size-5">
      <defs>
        <linearGradient id="bolt-gradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--color-bolt-400)" />
          <stop offset="100%" stopColor="var(--color-spark-400)" />
        </linearGradient>
      </defs>
      <path
        d="M14.1 1.3 6.1 13.1h4.8l-1.4 9.6 8.2-12.2h-4.8z"
        fill="url(#bolt-gradient)"
      />
    </svg>
  );
}
