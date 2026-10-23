# Banco di generalità

*Corsia 8, 23/10/2026. È la priorità 1 di `revisione-motore.md` (§4.2). La domanda: il motore va peggio sui deck su cui nessuno l'ha tarato?*

## Cosa misura
- **Due insiemi di deck, congelati.** Il manifesto `tests/fixtures/generality/sets.json` tiene, per ogni lista, la fonte (`ref`) e l'impronta (`hash`: Main e Extra Deck in ordine). Le liste sono scaricate e non si committano.
  - `rebuild` le riscarica in `data/generality/`, che git ignora:
    - YGOPRODeck: dalla pagina del deck;
    - Master Duel Meta: per `_id`;
    - le due liste Invoked: dal repository.
  - Poi controlla l'impronta e dice quali liste non si ricostruiscono più uguali, perché cambiate o sparite. Un confronto con liste diverse non vale: quelle si tolgono o si congela un manifesto nuovo.
  - **Il 23/10 si ricostruiscono tutte: 63 su 63.**
  - **Taratura:** i deck su cui il motore è stato tarato. Sono le 41 liste di Master Duel Meta del 22/10 usate dal protocollo e dalla caccia, più le due liste Invoked committate in `tests/fixtures/decks`. I deck del banco senza lista (Ignister, Labrynth, Mitsurugi, Yummy Mimighoul) contano come tarati: un deck fresco del loro archetipo si scarta.
  - **Freschi:** deck su cui nessuna modifica è mai stata misurata. Sono le liste di torneo più recenti di YGOPRODeck, TCG e OCG (anche di archetipi che in Master Duel non ci sono), e le liste più nuove di Master Duel Meta.
- **Regola dei freschi, senza nomi.** Uno *staple* è una carta presente in 6 deck di taratura o più: non dice niente dell'archetipo. Una lista è fresca se, delle sue carte che non sono staple, al massimo un quarto è in un qualsiasi deck di taratura. Due liste fresche che condividono più della metà di queste carte sono lo stesso archetipo: si tiene la più nuova.
- **Il numero.** Per ogni deck si cercano due mani, estratte come in `combo_hunt.py` (seme della mano `<deck>-<k>`, seme dei duelli 7). Ogni mano si cerca con il budget normale (4000 nodi) e con 10 volte tanto, su un thread, misurando le prime 10 board.
  - **Per mano:** il valore della board migliore misurata con il budget normale, diviso quello con il budget grande (il valore del banco, `bench.py`). Dice quanto la ricerca arriva vicino con il tempo che ha.
  - **Per insieme:** la media sulle mani.
  - **Divario:** taratura meno freschi, in punti percentuali.
  - **Cosa si rompe, per insieme:** run fallite, turni di prova falliti, linee interrotte.

## Comandi
```
python3 scripts/mdm_decks.py --out $S/mdm                                  # solo per rifare la taratura da zero
python3 scripts/generality.py freeze $S/gen --tuning $S/tun --count 20     # solo per congelare insiemi nuovi (rotazione)
python3 scripts/generality.py rebuild                                      # le liste in data/generality, controllate col manifesto
python3 scripts/generality.py check data/generality                        # di nuovo, senza scaricare
python3 scripts/generality.py run $S/gen/runs-main --sets data/generality --cli <cli di main> --jobs 4
python3 scripts/generality.py run $S/gen/runs-branch --sets data/generality --cli <cli del branch> --jobs 4
python3 scripts/generality.py report $S/gen/runs-main --against $S/gen/runs-branch
```
Le run già scritte si tengono: un giro interrotto riprende da dove si era fermato.

## Risultati su `main` 77999c9
*(in corso)*

## Come si giudica una modifica
Vale per ogni modifica a `crates/ygo-engine` e `crates/ygo-solver` che cambia la ricerca o la misura, accanto al protocollo di `AGENTS.md`.
1. **Lo stesso giro prima e dopo**, con le stesse liste (`check` deve dire che corrispondono) e lo stesso budget: `report --against`.
2. **I numeri si riportano per tutti e due gli insiemi**, mai solo per la taratura: valore di taratura, valore dei freschi, divario, e quello che si rompe.
3. **Una modifica non passa se:**
   - i freschi scendono di più di 1 punto mentre la taratura sale o resta (è adattamento al banco);
   - il divario cresce di più di 2 punti;
   - sui freschi compare una run fallita, un turno di prova fallito o una linea interrotta in più.
4. **Ogni costante nuova** (un peso, una soglia, un ordine) si accetta solo con il test appaiato di `paired_test.py` su taratura e freschi insieme, con la regola del segno.
5. **Rotazione:** ogni 2-3 mesi i freschi entrano nella taratura, e si congelano freschi nuovi con `freeze`. Il manifesto vecchio resta nella storia di git, con la data.
