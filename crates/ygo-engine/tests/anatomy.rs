//! The anatomy of card effects read from the scripts. They need the
//! downloaded game data (`duelforge-cli fetch-data`, or `DUELFORGE_DATA`);
//! without it they are skipped.

use std::path::PathBuf;

use ygo_engine::consts::{category, effect_code, effect_type, event};
use ygo_engine::{read_anatomy, EffectKind, Engine};

fn data() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    Some(data)
}

fn code(engine: &Engine, name: &str) -> u32 {
    engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name} in the database")).code
}

/// The constants the anatomy reads are the ones of the scripts.
#[test]
fn constants_match_the_scripts() {
    let Some(data) = data() else { return };
    let text = std::fs::read_to_string(data.join("scripts/constant.lua")).unwrap();
    let value = |name: &str| -> u64 {
        let line = text.lines().find(|l| l.split_whitespace().next() == Some(name)).unwrap_or_else(|| panic!("{name} in constant.lua"));
        let v = line.split('=').nth(1).unwrap().split_whitespace().next().unwrap();
        match v.strip_prefix("0x") {
            Some(hex) => u64::from_str_radix(hex, 16).unwrap(),
            None => v.parse().unwrap(),
        }
    };
    assert_eq!(value("EFFECT_TYPE_QUICK_O"), effect_type::QUICK_O as u64);
    assert_eq!(value("EFFECT_TYPE_TRIGGER_O"), effect_type::TRIGGER_O as u64);
    assert_eq!(value("EFFECT_TYPE_ACTIVATE"), effect_type::ACTIVATE as u64);
    assert_eq!(value("EFFECT_FUSION_MATERIAL"), effect_code::FUSION_MATERIAL as u64);
    assert_eq!(value("EFFECT_SPSUMMON_PROC"), effect_code::SPSUMMON_PROC as u64);
    assert_eq!(value("EVENT_SPSUMMON_SUCCESS"), event::SPSUMMON_SUCCESS as u64);
    assert_eq!(value("EVENT_CHAINING"), event::CHAINING as u64);
    for (bit, name) in ["DESTROY", "RELEASE", "REMOVE", "TOHAND", "TODECK", "TOGRAVE", "DECKDES", "HANDES"].iter().enumerate() {
        assert_eq!(value(&format!("CATEGORY_{name}")), 1 << bit, "CATEGORY_{name}");
    }
    assert_eq!(value("CATEGORY_NEGATE"), category::NEGATE);
    assert_eq!(value("CATEGORY_SET"), 1 << (category::NAMES.len() - 1));
}

/// Hyperinvoked Aeon: a Fusion Summon trigger that banishes, and a Quick
/// Effect that uses up the same once-per-turn limit (`ricerca-letteratura.md`,
/// the Aeon case).
#[test]
fn aeon() {
    let Some(data) = data() else { return };
    let engine = Engine::load(&data).unwrap();
    let aeon = code(&engine, "Hyperinvoked Aeon");
    let [card] = read_anatomy(&engine, &[aeon]).try_into().unwrap();
    assert_eq!(card.error, None);
    for e in &card.effects {
        eprintln!("{:?} {} {:?} {:?}", e.kind, e.code, e.categories(), e.count_limit);
    }
    assert!(card.effects.iter().any(|e| e.kind == EffectKind::Procedure));
    let banish = card
        .effects
        .iter()
        .find(|e| matches!(e.kind, EffectKind::Trigger { .. }) && e.code == event::SPSUMMON_SUCCESS && e.category & category::REMOVE != 0)
        .expect("a summon trigger that banishes");
    let quick = card.effects.iter().find(|e| matches!(e.kind, EffectKind::Quick { .. })).expect("a Quick Effect");
    assert!(banish.shares_limit(quick));
    assert_eq!(card.effect_of(banish.description), Some(banish));
}

/// A handtrap and a Normal Monster: a Quick Effect from the hand; no effects
/// and no error.
#[test]
fn handtrap_and_normal_monster() {
    let Some(data) = data() else { return };
    let engine = Engine::load(&data).unwrap();
    let ash = code(&engine, "Ash Blossom & Joyous Spring");
    let normal = code(&engine, "Dark Magician");
    let [ash, normal] = read_anatomy(&engine, &[ash, normal]).try_into().unwrap();
    assert_eq!(ash.error, None);
    let negate = ash.effects.iter().find(|e| matches!(e.kind, EffectKind::Quick { .. })).expect("a Quick Effect");
    assert_eq!(negate.range & 0x2, 0x2, "used from the hand");
    assert!(negate.interrupts() && negate.has_cost && negate.count_limit.is_some());
    assert_eq!((normal.effects.len(), normal.error.as_deref()), (0, None));
}
