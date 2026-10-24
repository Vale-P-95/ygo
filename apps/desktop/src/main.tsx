import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";

const Gallery = lazy(() => import("./pages/Gallery").then((m) => ({ default: m.Gallery })));

// The component gallery exists only with the mock backend (VITE_MOCK=1): #/galleria or /galleria.
const mock = import.meta.env.VITE_MOCK === "1" || import.meta.env.VITE_MOCK === "true";
const gallery = mock && (location.hash === "#/galleria" || location.pathname.replace(/\/$/, "").endsWith("/galleria"));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {gallery ? (
      <Suspense fallback={null}>
        <Gallery />
      </Suspense>
    ) : (
      <App />
    )}
  </StrictMode>,
);
