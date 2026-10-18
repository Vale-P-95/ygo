import { useSyncExternalStore } from "react";

export interface Toast {
  id: number;
  message: string;
  tone: "info" | "error" | "ok";
}

let toasts: Toast[] = [];
let next = 1;
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((l) => l());
}

export function toast(message: string, tone: Toast["tone"] = "info") {
  const id = next++;
  toasts = [...toasts.slice(-3), { id, message, tone }];
  emit();
  window.setTimeout(() => {
    toasts = toasts.filter((t) => t.id !== id);
    emit();
  }, 2800);
}

export function useToasts(): Toast[] {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => toasts,
  );
}
