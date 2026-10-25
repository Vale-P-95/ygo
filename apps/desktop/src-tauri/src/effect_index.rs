//! The masks behind the catalog's `eff:` filters (`ygo_data::effects`), for
//! every card of the store: read once, from the scripts when the engine has
//! them and from the texts.

use std::collections::HashMap;

use ygo_data::effects::{effect_mask, ScriptEffect};
use ygo_data::psct::{self, Frame};
use ygo_data::{Card, CardKind, CardStore, SearchQuery};
use ygo_engine::{EffectKind, Engine};

/// How the text of `card` is read.
pub fn frame_of(card: &Card) -> Frame {
    match card.kind {
        CardKind::Monster if ["fusion", "synchro", "xyz", "link"].iter().any(|f| card.frame.starts_with(f)) => Frame::ExtraDeck,
        CardKind::Monster => Frame::Monster,
        _ => Frame::SpellTrap,
    }
}

/// Every deck card of `store`, to read outside its lock.
pub fn all_cards(store: &CardStore) -> Result<Vec<Card>, ygo_data::Error> {
    store.get_many(&store.search(&SearchQuery::default())?)
}

/// The mask of each of `cards`; without `engine`, from the texts only.
pub fn build(cards: &[Card], engine: Option<&Engine>) -> HashMap<u32, u16> {
    let scripts = engine.map(|engine| {
        let codes: Vec<u32> = cards.iter().map(|c| engine.cards().canonical_code(c.id)).collect();
        ygo_engine::read_anatomy(engine, &codes)
    });
    cards
        .iter()
        .enumerate()
        .map(|(i, card)| {
            let mut text = card.desc.clone();
            if let Some(pend) = &card.pend_desc {
                text = format!("{pend}\n{text}");
            }
            let mut reading = psct::read(&card.desc, frame_of(card));
            if let Some(pend) = &card.pend_desc {
                let pendulum = psct::read(pend, Frame::Monster);
                reading.pieces.extend(pendulum.pieces);
                reading.effects.extend(pendulum.effects);
            }
            let script = scripts.as_ref().and_then(|s| s.get(i)).filter(|a| a.error.is_none());
            let categories: Vec<Vec<&'static str>> = script.map(|a| a.effects.iter().map(|e| e.categories()).collect()).unwrap_or_default();
            let effects: Option<Vec<ScriptEffect>> = script.map(|a| {
                a.effects
                    .iter()
                    .zip(&categories)
                    .map(|(e, categories)| ScriptEffect {
                        activated: e.kind.activated(),
                        card_activation: e.kind == EffectKind::Activation,
                        quick: matches!(e.kind, EffectKind::Quick { .. }),
                        range: e.range,
                        categories,
                    })
                    .collect()
            });
            (card.id, effect_mask(&text, &reading, effects.as_deref()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ygo_data::EffectFilter;

    use super::*;

    /// The filters on the downloaded data: how many cards each finds, and a
    /// few cards whose effects are known. Skipped without the data.
    #[test]
    fn filters_on_the_real_data() {
        let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("../../../data"));
        if !data.join("scripts/constant.lua").is_file() || !data.join("cards.db").is_file() {
            eprintln!("skipped: no game data in {}", data.display());
            return;
        }
        let store = CardStore::open(&data.join("cards.db")).unwrap();
        let cards = all_cards(&store).unwrap();
        let engine = Engine::load(&data).unwrap();
        let started = std::time::Instant::now();
        let with_scripts = build(&cards, Some(&engine));
        let elapsed = started.elapsed();
        let text_only = build(&cards, None);
        eprintln!("{} cards in {elapsed:.1?}", cards.len());
        for f in EffectFilter::ALL {
            let count = |index: &HashMap<u32, u16>| index.values().filter(|m| *m & f.bit() != 0).count();
            eprintln!("{:<32} {:>5} (text only {:>5})", f.label(), count(&with_scripts), count(&text_only));
        }
        let mask = |name: &str| {
            let card = cards.iter().find(|c| c.name == name).unwrap();
            EffectFilter::ALL.into_iter().filter(|f| with_scripts[&card.id] & f.bit() != 0).collect::<Vec<_>>()
        };
        let aeon = mask("Hyperinvoked Aeon");
        assert!(aeon.contains(&EffectFilter::Banish) && aeon.contains(&EffectFilter::OpponentExtra), "{aeon:?}");
        let ash = mask("Ash Blossom & Joyous Spring");
        for f in [EffectFilter::Negate, EffectFilter::Quick, EffectFilter::Hand] {
            assert!(ash.contains(&f), "{ash:?}");
        }
        assert!(!mask("Pot of Greed").contains(&EffectFilter::Hand));
    }
}
