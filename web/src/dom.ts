/** Small helpers for building the interface without `innerHTML`. */

/**
 * Returns the element with `id`, checking that it is a `type`. Fails loudly
 * if the page lacks it, since the page and the code ship together.
 */
export function byId(id: string): HTMLElement;
export function byId<T extends HTMLElement>(id: string, type: new () => T): T;
export function byId(
  id: string,
  type: new () => HTMLElement = HTMLElement,
): HTMLElement {
  const found = document.getElementById(id);
  if (!(found instanceof type)) {
    throw new Error(`the page has no #${id} ${type.name}`);
  }
  return found;
}

/** Creates an element with optional classes and text. */
export function element<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  options: { className?: string; text?: string } = {},
): HTMLElementTagNameMap[K] {
  const created = document.createElement(tag);
  if (options.className !== undefined) {
    created.className = options.className;
  }
  if (options.text !== undefined) {
    created.textContent = options.text;
  }
  return created;
}
