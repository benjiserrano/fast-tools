/**
 * Navegación entre herramientas desde dentro de un panel.
 *
 * Se usa un evento del DOM en lugar de pasar una función por las props: los
 * paneles se cargan de forma diferida y son hojas del árbol, así que hacer
 * llegar un `onNavigate` hasta ellos obligaría a enhebrarlo por cada capa
 * intermedia solo para los dos casos que lo necesitan.
 */

const EVENT = "fast-tools:navigate";

export function navigateToTool(id: string): void {
  window.dispatchEvent(new CustomEvent<string>(EVENT, { detail: id }));
}

export function onNavigateToTool(handler: (id: string) => void): () => void {
  const listener = (event: Event) => {
    handler((event as CustomEvent<string>).detail);
  };
  window.addEventListener(EVENT, listener);
  return () => window.removeEventListener(EVENT, listener);
}
