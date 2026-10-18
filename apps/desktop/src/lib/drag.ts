/**
 * Card drag & drop with pointer events (works the same in every webview,
 * unlike HTML5 drag and drop). A drag starts after a few pixels of movement,
 * so clicks and double clicks still work; drop targets are elements with a
 * `data-dropzone` attribute.
 */
import { useSyncExternalStore, type PointerEvent as ReactPointerEvent } from "react";

export type DropZone = "main" | "extra" | "side" | "catalog";

export interface DragSource {
  id: number;
  from: DropZone;
  /** Position in the source zone (deck zones only). */
  index?: number;
}

export interface DragState {
  source: DragSource;
  x: number;
  y: number;
  over: DropZone | null;
}

let state: DragState | null = null;
const listeners = new Set<() => void>();
let onDrop: ((source: DragSource, target: DropZone) => void) | null = null;

function emit() {
  listeners.forEach((l) => l());
}

function zoneAt(x: number, y: number): DropZone | null {
  const el = document.elementFromPoint(x, y)?.closest("[data-dropzone]");
  return (el?.getAttribute("data-dropzone") as DropZone | null) ?? null;
}

/** Registers what happens when a card is dropped (one handler at a time). */
export function setDropHandler(handler: typeof onDrop) {
  onDrop = handler;
}

export function useDrag(): DragState | null {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => state,
  );
}

/** Call from `onPointerDown` of a draggable card. */
export function pointerDownDraggable(e: ReactPointerEvent, source: DragSource) {
  if (e.button !== 0) return;
  const startX = e.clientX;
  const startY = e.clientY;
  let started = false;
  const move = (ev: PointerEvent) => {
    if (!started) {
      if (Math.hypot(ev.clientX - startX, ev.clientY - startY) < 6) return;
      started = true;
      document.body.style.cursor = "grabbing";
    }
    state = { source, x: ev.clientX, y: ev.clientY, over: zoneAt(ev.clientX, ev.clientY) };
    emit();
  };
  const up = (ev: PointerEvent) => {
    removeEventListener("pointermove", move);
    removeEventListener("pointerup", up);
    document.body.style.cursor = "";
    if (started) {
      const target = zoneAt(ev.clientX, ev.clientY);
      state = null;
      emit();
      if (target && onDrop) onDrop(source, target);
    }
  };
  addEventListener("pointermove", move);
  addEventListener("pointerup", up);
}
