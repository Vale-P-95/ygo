import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { IconButton } from "./IconButton";

describe("IconButton", () => {
  it("imposta data-tip e aria-label ma omette title per evitare doppio tooltip nativo", () => {
    const html = renderToStaticMarkup(
      <IconButton icon={<span>icon</span>} tooltip="Chiudi pannello" ariaLabel="Chiudi pannello" />,
    );

    expect(html).toContain('data-tip="Chiudi pannello"');
    expect(html).toContain('aria-label="Chiudi pannello"');
    expect(html).not.toContain("title=");
  });

  it("omette aria-pressed per pulsanti di azione semplici (active non specificato)", () => {
    const html = renderToStaticMarkup(
      <IconButton icon={<span>icon</span>} tooltip="Salva" ariaLabel="Salva deck" />,
    );

    expect(html).not.toContain("aria-pressed");
  });

  it("imposta aria-pressed=\"true\" solo quando active è true", () => {
    const html = renderToStaticMarkup(
      <IconButton icon={<span>icon</span>} tooltip="Filtro" ariaLabel="Attiva filtro" active={true} />,
    );

    expect(html).toContain('aria-pressed="true"');
  });

  it("imposta aria-pressed=\"false\" quando active è false per pulsanti a due stati", () => {
    const html = renderToStaticMarkup(
      <IconButton icon={<span>icon</span>} tooltip="Filtro" ariaLabel="Attiva filtro" active={false} />,
    );

    expect(html).toContain('aria-pressed="false"');
  });

  it("applica le classi di variante e dimensione", () => {
    const html = renderToStaticMarkup(
      <IconButton
        icon={<span>del</span>}
        tooltip="Elimina"
        ariaLabel="Elimina deck"
        variant="danger"
        size="lg"
      />,
    );

    expect(html).toContain("text-danger");
    expect(html).toContain("h-9 w-9");
  });
});
