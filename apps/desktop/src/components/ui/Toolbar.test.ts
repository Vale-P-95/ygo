import { describe, expect, it } from "vitest";
import { resolveToolbarCollapse, type ToolbarItem } from "./Toolbar";

describe("resolveToolbarCollapse (quando la barra si raccoglie in «Filtri»)", () => {
  const sampleItems: ToolbarItem[] = [
    { id: "starter", label: "Starter", priority: 10, active: true },
    { id: "extender", label: "Estensori", priority: 8, active: false },
    { id: "handtrap", label: "Handtrap", priority: 6, active: true },
    { id: "breaker", label: "Board breaker", priority: 4, active: false },
    { id: "brick", label: "Brick", priority: 2, active: true },
  ];

  it("mantiene tutti gli elementi visibili se la capienza è sufficiente", () => {
    const res = resolveToolbarCollapse(sampleItems, { maxVisibleItems: 5 });
    expect(res.isCollapsed).toBe(false);
    expect(res.visibleItems.length).toBe(5);
    expect(res.collapsedItems).toEqual([]);
    expect(res.activeCollapsedCount).toBe(0);
  });

  it("raccoglie gli elementi a priorità più bassa in «Filtri» quando si supera il limite", () => {
    const res = resolveToolbarCollapse(sampleItems, { maxVisibleItems: 3 });
    expect(res.isCollapsed).toBe(true);
    expect(res.visibleItems.map((i) => i.id)).toEqual(["starter", "extender", "handtrap"]);
    expect(res.collapsedItems.map((i) => i.id)).toEqual(["breaker", "brick"]);
    // Tra breaker (active: false) e brick (active: true), solo 1 è attivo
    expect(res.activeCollapsedCount).toBe(1);
  });

  it("non collassa mai gli elementi con alwaysVisible=true", () => {
    const withAlwaysVisible: ToolbarItem[] = [
      { id: "search", label: "Cerca...", alwaysVisible: true, priority: 0 },
      ...sampleItems,
    ];

    const res = resolveToolbarCollapse(withAlwaysVisible, { maxVisibleItems: 2 });
    expect(res.isCollapsed).toBe(true);
    // search deve rimanere visibile nonostante priority 0
    expect(res.visibleItems.map((i) => i.id)).toContain("search");
    expect(res.collapsedItems.map((i) => i.id)).not.toContain("search");
    // e i due elementi a priorità più alta tra i collassabili
    expect(res.visibleItems.map((i) => i.id)).toEqual(["search", "starter", "extender"]);
  });

  it("in modalità compatta (isCompact=true) collassa tutti gli elementi non alwaysVisible", () => {
    const withAlwaysVisible: ToolbarItem[] = [
      { id: "search", label: "Cerca...", alwaysVisible: true },
      { id: "f1", label: "Filtro 1", active: true },
      { id: "f2", label: "Filtro 2", active: true },
      { id: "f3", label: "Filtro 3", active: false },
    ];

    const res = resolveToolbarCollapse(withAlwaysVisible, { isCompact: true });
    expect(res.isCollapsed).toBe(true);
    expect(res.visibleItems.map((i) => i.id)).toEqual(["search"]);
    expect(res.collapsedItems.length).toBe(3);
    expect(res.activeCollapsedCount).toBe(2);
  });

  it("calcola la capienza in base alla larghezza disponibile in pixel", () => {
    // 5 elementi × 100px = 500px necessari.
    // Con 350px disponibili e 80px riservati al pulsante «Filtri»:
    // Spazio utile = 350 - 80 = 270px -> capienza = floor(270 / 100) = 2 elementi
    const res = resolveToolbarCollapse(sampleItems, {
      availableWidth: 350,
      itemMinWidth: 100,
      filtersButtonWidth: 80,
    });

    expect(res.isCollapsed).toBe(true);
    expect(res.visibleItems.length).toBe(2);
    expect(res.collapsedItems.length).toBe(3);
  });

  it("gestisce liste vuote o composte solo da elementi alwaysVisible", () => {
    expect(resolveToolbarCollapse([], { maxVisibleItems: 2 })).toEqual({
      visibleItems: [],
      collapsedItems: [],
      isCollapsed: false,
      activeCollapsedCount: 0,
    });

    const onlyFixed: ToolbarItem[] = [{ id: "q", label: "Q", alwaysVisible: true }];
    const resFixed = resolveToolbarCollapse(onlyFixed, { maxVisibleItems: 0 });
    expect(resFixed.isCollapsed).toBe(false);
    expect(resFixed.visibleItems.length).toBe(1);
    expect(resFixed.collapsedItems.length).toBe(0);
  });

  it("supporta larghezze eterogenee reali (item.width) e riempimento greedy in base a priorità", () => {
    const variedItems: ToolbarItem[] = [
      { id: "short", label: "Tutte", priority: 10, width: 80, active: false },
      { id: "long", label: "Posiziona dal Deck", priority: 8, width: 200, active: true },
      { id: "medium", label: "Evoca", priority: 9, width: 110, active: false },
    ];

    // Total width = 80 + 200 + 110 = 390px
    // With 400px available, all fit
    const allFit = resolveToolbarCollapse(variedItems, { availableWidth: 400 });
    expect(allFit.isCollapsed).toBe(false);
    expect(allFit.visibleItems.length).toBe(3);

    // With 260px available and 90px filters button: space for items = 170px
    // Priority order: short (10, 80px), medium (9, 110px), long (8, 200px)
    // short (80px) fits (rem: 90px), medium (110px) doesn't fit in 90px, long (200px) doesn't fit
    // visible: short (80px), collapsed: medium and long
    const partialFit = resolveToolbarCollapse(variedItems, {
      availableWidth: 260,
      filtersButtonWidth: 90,
    });
    expect(partialFit.isCollapsed).toBe(true);
    expect(partialFit.visibleItems.map((i) => i.id)).toEqual(["short"]);
    expect(partialFit.collapsedItems.map((i) => i.id)).toEqual(["long", "medium"]);
    expect(partialFit.activeCollapsedCount).toBe(1); // long is active
  });
});
