import { useEffect, useState } from "react";
import { CommandPalette } from "./components/CommandPalette";
import { Embers } from "./components/Embers";
import { ChartTooltip, DragLayer, Toasts } from "./components/Overlays";
import { Sidebar } from "./components/Sidebar";
import { WelcomeModal, hasSeenOnboarding } from "./components/WelcomeModal";
import { api, isDesktop, type DataStatus, type DeckSummary } from "./lib/api";
import { rememberDeck, requestNewDeck } from "./lib/deck";
import { useSettings } from "./lib/settings";
import { sound } from "./lib/sound";
import { Combo } from "./pages/Combo";
import { DeckBuilder } from "./pages/DeckBuilder";
import { Lab } from "./pages/Lab";
import { Overview } from "./pages/Overview";
import { Settings } from "./pages/Settings";
import { Shelf } from "./pages/Shelf";
import { Upcoming } from "./pages/Upcoming";
import { sectionById, sections, type SectionId } from "./sections";

/** Ember color per section, matching its accent. */
const EMBER_COLORS: Partial<Record<SectionId, string>> = {
  "deck-builder": "#ff8a3d",
  stats: "#5ee6db",
  combo: "#c39bff",
  shelf: "#e2bb7d",
};

function isEditableElement(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

function useTheme() {
  const { theme, cardEffects } = useSettings();
  useEffect(() => {
    const root = document.documentElement;
    const apply = () => {
      const light = theme === "light" || (theme === "system" && matchMedia("(prefers-color-scheme: light)").matches);
      root.dataset.theme = light ? "light" : "forge";
    };
    apply();
    root.dataset.cardEffects = cardEffects ? "on" : "off";
    if (theme !== "system") return;
    const media = matchMedia("(prefers-color-scheme: light)");
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme, cardEffects]);
}

export default function App() {
  useTheme();
  const [active, setActive] = useState<SectionId>("deck-builder");
  const [status, setStatus] = useState<DataStatus | null>(null);
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [guideOpen, setGuideOpen] = useState<boolean>(() => !hasSeenOnboarding());
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [navVersion, setNavVersion] = useState(0);

  useEffect(() => {
    if (isDesktop) api.dataStatus().then(setStatus).catch(() => setStatus(null));
  }, []);

  useEffect(() => {
    if (!isDesktop || !status?.cards) return;
    api
      .listDecks()
      .then(setDecks)
      .catch(() => setDecks([]));
  }, [status?.cards, active, guideOpen, paletteOpen, navVersion]);

  // Portals (viewer, previews) live in <body>: give it the section accent too.
  useEffect(() => {
    document.body.dataset.section = active;
  }, [active]);

  // Global keyboard shortcuts: Ctrl/Cmd+K (Command Palette), Ctrl/Alt+1..7 (Sections), ? (Quick Guide).
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        sound.play("open");
        setGuideOpen(false);
        setPaletteOpen((open) => !open);
        return;
      }

      if ((e.ctrlKey || e.altKey) && !e.shiftKey && !e.metaKey && e.key >= "1" && e.key <= "7") {
        const idx = Number(e.key) - 1;
        const targetSection = sections[idx];
        if (targetSection) {
          e.preventDefault();
          setPaletteOpen(false);
          setGuideOpen(false);
          if (targetSection.id !== active) {
            sound.play("page");
            setActive(targetSection.id);
          }
        }
        return;
      }

      if (e.key === "?" && !e.ctrlKey && !e.metaKey && !e.altKey && !isEditableElement(e.target)) {
        e.preventDefault();
        sound.play("open");
        setPaletteOpen(false);
        setGuideOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active]);

  const section = sectionById(active);
  const artReady = !!status?.cards;
  const deckCount = decks.length;

  return (
    <div className="flex h-full">
      <Sidebar
        active={active}
        onSelect={setActive}
        onOpenPalette={() => {
          setGuideOpen(false);
          setPaletteOpen(true);
        }}
        onOpenGuide={() => {
          setPaletteOpen(false);
          setGuideOpen(true);
        }}
        compact={(active === "deck-builder" || active === "shelf" || active === "stats" || active === "combo") && !!status?.cards}
      />
      <main data-section={active} className="relative min-w-0 flex-1">
        <Embers color={EMBER_COLORS[active] ?? "#ffc65c"} />
        <div key={`${active}:${navVersion}`} className="relative h-full anim-fade">
          {active === "overview" && (
            <Overview status={status} deckCount={deckCount} onStatus={setStatus} go={setActive} onOpenGuide={() => setGuideOpen(true)} />
          )}
          {active === "deck-builder" && <DeckBuilder status={status} onStatus={setStatus} />}
          {active === "shelf" && (
            <Shelf
              artReady={artReady}
              onOpenDeck={(id) => {
                rememberDeck(id);
                setActive("deck-builder");
              }}
              onNewDeck={() => {
                requestNewDeck();
                setActive("deck-builder");
              }}
            />
          )}
          {active === "stats" && <Lab artReady={artReady} onGoForge={() => setActive("deck-builder")} />}
          {active === "combo" && <Combo artReady={artReady} onGoForge={() => setActive("deck-builder")} />}
          {active === "settings" && <Settings onStatus={setStatus} />}
          {!["overview", "deck-builder", "shelf", "stats", "combo", "settings"].includes(active) && <Upcoming section={section} artReady={artReady} />}
        </div>
      </main>
      <CommandPalette
        open={paletteOpen}
        activeSection={active}
        decks={decks}
        onClose={() => setPaletteOpen(false)}
        onSelectSection={setActive}
        onOpenDeckIn={(deckId, targetSection) => {
          rememberDeck(deckId);
          setNavVersion((v) => v + 1);
          setActive(targetSection);
        }}
        onNewDeck={() => {
          requestNewDeck();
          setNavVersion((v) => v + 1);
          setActive("deck-builder");
        }}
        onOpenGuide={() => setGuideOpen(true)}
      />
      <WelcomeModal open={guideOpen} status={status} deckCount={deckCount} onClose={() => setGuideOpen(false)} onGo={setActive} />
      <DragLayer />
      <Toasts />
      <ChartTooltip />
    </div>
  );
}
