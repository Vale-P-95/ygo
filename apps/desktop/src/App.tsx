import { useEffect, useState } from "react";
import { Embers } from "./components/Embers";
import { ChartTooltip, DragLayer, Toasts } from "./components/Overlays";
import { Sidebar } from "./components/Sidebar";
import { WelcomeModal, hasSeenOnboarding } from "./components/WelcomeModal";
import { api, isDesktop, type DataStatus } from "./lib/api";
import { rememberDeck, requestNewDeck } from "./lib/deck";
import { useSettings } from "./lib/settings";
import { Combo } from "./pages/Combo";
import { DeckBuilder } from "./pages/DeckBuilder";
import { Lab } from "./pages/Lab";
import { Overview } from "./pages/Overview";
import { Settings } from "./pages/Settings";
import { Shelf } from "./pages/Shelf";
import { Upcoming } from "./pages/Upcoming";
import { sectionById, type SectionId } from "./sections";

/** Ember color per section, matching its accent. */
const EMBER_COLORS: Partial<Record<SectionId, string>> = {
  "deck-builder": "#ff8a3d",
  stats: "#5ee6db",
  combo: "#c39bff",
  shelf: "#e2bb7d",
};

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
  const [deckCount, setDeckCount] = useState(0);
  const [guideOpen, setGuideOpen] = useState<boolean>(() => !hasSeenOnboarding());

  useEffect(() => {
    if (isDesktop) api.dataStatus().then(setStatus).catch(() => setStatus(null));
  }, []);

  useEffect(() => {
    if (!isDesktop || !status?.cards) return;
    api
      .listDecks()
      .then((decks) => setDeckCount(decks.length))
      .catch(() => setDeckCount(0));
  }, [status?.cards, active, guideOpen]);

  // Portals (viewer, previews) live in <body>: give it the section accent too.
  useEffect(() => {
    document.body.dataset.section = active;
  }, [active]);

  const section = sectionById(active);
  const artReady = !!status?.cards;

  return (
    <div className="flex h-full">
      <Sidebar
        active={active}
        onSelect={setActive}
        onOpenGuide={() => setGuideOpen(true)}
        compact={(active === "deck-builder" || active === "shelf" || active === "stats" || active === "combo") && !!status?.cards}
      />
      <main data-section={active} className="relative min-w-0 flex-1">
        <Embers color={EMBER_COLORS[active] ?? "#ffc65c"} />
        <div key={active} className="relative h-full anim-fade">
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
      <WelcomeModal open={guideOpen} status={status} deckCount={deckCount} onClose={() => setGuideOpen(false)} onGo={setActive} />
      <DragLayer />
      <Toasts />
      <ChartTooltip />
    </div>
  );
}
