import { Command } from "cmdk";

import type { Category, Tool } from "../lib/types";
import { CATEGORY_LABEL, CATEGORY_ORDER } from "../lib/types";

interface CommandPaletteProps {
  open: boolean;
  tools: Tool[];
  onOpenChange: (open: boolean) => void;
  onSelect: (id: string) => void;
}

export function CommandPalette({
  open,
  tools,
  onOpenChange,
  onSelect,
}: CommandPaletteProps) {
  return (
    <Command.Dialog
      open={open}
      onOpenChange={onOpenChange}
      label="Buscar herramienta"
      // El overlay cubre la ventana; el diálogo se ancla en el tercio superior,
      // que es donde el ojo espera un buscador.
      overlayClassName="fixed inset-0 z-40 bg-ink-950/70 backdrop-blur-sm"
      contentClassName="fixed left-1/2 top-[18%] z-50 w-[min(38rem,90vw)] -translate-x-1/2 overflow-hidden rounded-xl border border-ink-700 bg-ink-900 shadow-2xl"
    >
      <Command.Input
        autoFocus
        placeholder="Escribe para buscar: json, hash, imagen, cron…"
        className="w-full border-b border-ink-800 bg-transparent px-4 py-3 text-sm text-mist-50 outline-none placeholder:text-mist-500"
      />

      <Command.List className="max-h-80 overflow-y-auto p-2">
        <Command.Empty className="px-3 py-8 text-center text-sm text-mist-500">
          Ninguna herramienta coincide.
        </Command.Empty>

        {CATEGORY_ORDER.map((category) => (
          <CategoryGroup
            key={category}
            category={category}
            tools={tools.filter((tool) => tool.category === category)}
            onSelect={onSelect}
          />
        ))}
      </Command.List>
    </Command.Dialog>
  );
}

function CategoryGroup({
  category,
  tools,
  onSelect,
}: {
  category: Category;
  tools: Tool[];
  onSelect: (id: string) => void;
}) {
  if (tools.length === 0) return null;

  return (
    <Command.Group
      heading={CATEGORY_LABEL[category]}
      className="mb-1 [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:text-mist-500"
    >
      {tools.map((tool) => (
        <Command.Item
          key={tool.id}
          value={tool.name}
          keywords={[tool.id, tool.summary, ...tool.keywords]}
          onSelect={() => onSelect(tool.id)}
          className="flex cursor-pointer items-center gap-3 rounded-md px-2 py-2 text-sm text-mist-400 data-[selected=true]:bg-ink-800 data-[selected=true]:text-mist-50"
        >
          <span className="min-w-0 shrink-0 font-medium">{tool.name}</span>
          <span className="min-w-0 flex-1 truncate text-xs text-mist-500">
            {tool.summary}
          </span>
          {tool.status === "planned" && (
            <span className="shrink-0 rounded border border-ink-700 px-1 font-mono text-[10px] text-mist-500">
              Fase {tool.phase}
            </span>
          )}
        </Command.Item>
      ))}
    </Command.Group>
  );
}
