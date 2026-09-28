/** Primitivas visuales compartidas por los paneles de herramientas. */

import type { ReactNode, SelectHTMLAttributes } from "react";

export function Button({
  children,
  onClick,
  disabled,
  variant = "ghost",
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  variant?: "ghost" | "primary";
  title?: string;
}) {
  const base =
    "rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40";
  const styles =
    variant === "primary"
      ? "bg-bolt-500/15 text-bolt-400 hover:bg-bolt-500/25"
      : "border border-ink-700 text-mist-400 hover:border-ink-600 hover:text-mist-200";

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      className={`${base} ${styles}`}
    >
      {children}
    </button>
  );
}

export function Select({
  label,
  ...props
}: { label?: string } & SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <label className="flex items-center gap-1.5 text-xs text-mist-500">
      {label && <span>{label}</span>}
      <select
        {...props}
        className="rounded-md border border-ink-700 bg-ink-850 px-2 py-1 text-xs text-mist-200 outline-none focus:border-ink-600"
      />
    </label>
  );
}

export function Toggle({
  label,
  checked,
  onChange,
  title,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  title?: string;
}) {
  return (
    <label
      title={title}
      className="flex cursor-pointer items-center gap-1.5 text-xs text-mist-500 hover:text-mist-400"
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="size-3.5 accent-bolt-500"
      />
      {label}
    </label>
  );
}

export function NumberField({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex items-center gap-1.5 text-xs text-mist-500">
      {label}
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        onChange={(event) => {
          const parsed = Number(event.target.value);
          if (Number.isFinite(parsed)) {
            onChange(Math.min(max, Math.max(min, parsed)));
          }
        }}
        className="w-14 rounded-md border border-ink-700 bg-ink-850 px-2 py-1 text-xs text-mist-200 outline-none focus:border-ink-600"
      />
    </label>
  );
}

export function TextField({
  label,
  value,
  onChange,
  width = "w-20",
  title,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  width?: string;
  title?: string;
}) {
  return (
    <label
      title={title}
      className="flex items-center gap-1.5 text-xs text-mist-500"
    >
      {label}
      <input
        type="text"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={`${width} rounded-md border border-ink-700 bg-ink-850 px-2 py-1 font-mono text-xs text-mist-200 outline-none focus:border-ink-600`}
      />
    </label>
  );
}

export function Banner({
  tone,
  children,
}: {
  tone: "error" | "warn" | "info";
  children: ReactNode;
}) {
  const styles = {
    error: "border-danger-400/40 bg-danger-400/10 text-danger-400",
    warn: "border-warn-400/40 bg-warn-400/10 text-warn-400",
    info: "border-ink-700 bg-ink-850 text-mist-400",
  }[tone];

  return (
    <div
      className={`selectable rounded-md border px-3 py-2 text-xs leading-relaxed ${styles}`}
    >
      {children}
    </div>
  );
}

export function Toolbar({ children }: { children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-ink-800 px-4 py-2.5">
      {children}
    </div>
  );
}

export function Spacer() {
  return <div className="ml-auto" />;
}
