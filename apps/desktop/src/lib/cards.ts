import { useCallback, useRef, useState } from "react";
import { api, type Card } from "./api";

/**
 * Card details fetched on demand: the grid asks for the cards it is about to
 * show, and they are fetched in one batch and kept for the session.
 */
export function useCardCache() {
  const cache = useRef(new Map<number, Card>());
  const pending = useRef(new Set<number>());
  const [version, setVersion] = useState(0);

  const request = useCallback((ids: number[]) => {
    const missing = ids.filter((id) => !cache.current.has(id) && !pending.current.has(id));
    if (missing.length === 0) return;
    missing.forEach((id) => pending.current.add(id));
    api
      .getCards(missing)
      .then((cards) => {
        cards.forEach((c) => cache.current.set(c.id, c));
        setVersion((v) => v + 1);
      })
      .finally(() => missing.forEach((id) => pending.current.delete(id)));
  }, []);

  const get = useCallback((id: number) => cache.current.get(id), []);
  return { get, request, version };
}
