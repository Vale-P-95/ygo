/**
 * Exact draw probabilities (hypergeometric): drawing `n` cards from a deck
 * of `size` without replacement.
 */

/** Probability that none of `hits` copies is among the `n` cards drawn. */
export function pNone(size: number, hits: number, n: number): number {
  if (n > size) return hits > 0 ? 0 : 1;
  let p = 1;
  for (let i = 0; i < n; i++) p *= Math.max(0, size - hits - i) / (size - i);
  return p;
}

/** Probability of drawing at least one of `hits` copies. */
export function pAtLeastOne(size: number, hits: number, n: number): number {
  return 1 - pNone(size, hits, n);
}

/**
 * Probability of at least one card of group A and one of group B, where
 * `either` counts the copies in A or B (a card can be in both).
 */
export function pBoth(size: number, a: number, b: number, either: number, n: number): number {
  return 1 - pNone(size, a, n) - pNone(size, b, n) + pNone(size, either, n);
}

export function shuffle<T>(items: T[]): T[] {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

export const percent = (p: number) => `${(p * 100).toLocaleString("it-IT", { maximumFractionDigits: 1, minimumFractionDigits: 1 })}%`;
