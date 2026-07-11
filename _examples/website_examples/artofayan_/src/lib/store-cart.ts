const CART_KEY = "artofayan-store-cart";

export function readCart(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(CART_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((id) => typeof id === "string") : [];
  } catch {
    return [];
  }
}

export function writeCart(productIds: string[]): void {
  window.localStorage.setItem(CART_KEY, JSON.stringify([...new Set(productIds)]));
}

export function addToCart(productId: string): string[] {
  const next = [...new Set([...readCart(), productId])];
  writeCart(next);
  window.dispatchEvent(new CustomEvent("store-cart-updated", { detail: { count: next.length } }));
  return next;
}

export function removeFromCart(productId: string): string[] {
  const next = readCart().filter((id) => id !== productId);
  writeCart(next);
  window.dispatchEvent(new CustomEvent("store-cart-updated", { detail: { count: next.length } }));
  return next;
}

export function clearCart(): void {
  writeCart([]);
  window.dispatchEvent(new CustomEvent("store-cart-updated", { detail: { count: 0 } }));
}
