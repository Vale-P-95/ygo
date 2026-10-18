//! Import + search on a small synthetic card list shaped like the
//! YGOPRODeck `cardinfo.php?misc=yes` response.

use ygo_data::{import, parse_query, CardKind, CardStore};

const SAMPLE: &str = r#"{"data": [
  {"id": 1001, "name": "Forge Golem", "type": "Effect Monster", "humanReadableCardType": "Effect Monster",
   "frameType": "effect", "desc": "When this card is Normal Summoned: You can add 1 \"Anvil\" card from your Deck to your hand.",
   "race": "Machine", "attribute": "FIRE", "archetype": "Forge", "typeline": ["Machine", "Effect"],
   "level": 4, "atk": 1800, "def": 1000, "card_images": [{"id": 1001}, {"id": 1002}],
   "misc_info": [{"formats": ["TCG", "OCG", "Master Duel"], "md_rarity": "Super Rare", "views": 500, "tcg_date": "2020-01-01"}]},
  {"id": 1003, "name": "Forge Titan", "type": "Fusion Monster", "humanReadableCardType": "Fusion Effect Monster",
   "frameType": "fusion", "desc": "\"Forge Golem\" + 1 FIRE monster\nOnce per turn (Quick Effect): You can negate the activation of a card.",
   "race": "Machine", "attribute": "FIRE", "archetype": "Forge", "typeline": ["Machine", "Fusion", "Effect"],
   "level": 8, "atk": 3000, "def": 2500, "card_images": [{"id": 1003}],
   "misc_info": [{"formats": ["TCG", "OCG", "Master Duel"], "md_rarity": "Ultra Rare", "views": 900, "tcg_date": "2021-05-01"}]},
  {"id": 1004, "name": "Anvil of Sparks", "type": "Spell Card", "humanReadableCardType": "Quick-Play Spell",
   "frameType": "spell", "desc": "Special Summon 1 \"Forge\" monster from your GY.", "race": "Quick-Play",
   "archetype": "Forge", "card_images": [{"id": 1004}], "banlist_info": {"ban_tcg": "Limited"},
   "misc_info": [{"formats": ["TCG", "OCG"], "views": 300, "tcg_date": "2021-06-01"}]},
  {"id": 1005, "name": "Laboratory Lockdown", "type": "Trap Card", "humanReadableCardType": "Continuous Trap",
   "frameType": "trap", "desc": "Neither player can Special Summon monsters from the Extra Deck.", "race": "Continuous",
   "card_images": [{"id": 1005}], "misc_info": [{"formats": ["TCG", "Master Duel"], "md_rarity": "Rare", "views": 100}]},
  {"id": 1006, "name": "Gear Link Engineer", "type": "Link Monster", "humanReadableCardType": "Link Effect Monster",
   "frameType": "link", "desc": "2 Machine monsters", "race": "Machine", "attribute": "EARTH", "typeline": ["Machine", "Link", "Effect"],
   "atk": 2300, "linkval": 2, "linkmarkers": ["Bottom-Left", "Bottom-Right"], "card_images": [{"id": 1006}],
   "misc_info": [{"formats": ["TCG", "OCG", "Master Duel"], "md_rarity": "Ultra Rare", "views": 700}]},
  {"id": 1007, "name": "Mystery Tinkerer", "type": "Effect Monster", "humanReadableCardType": "Effect Monster",
   "frameType": "effect", "desc": "Its ATK is unknown.", "race": "Spellcaster", "attribute": "DARK", "typeline": ["Spellcaster", "Effect"],
   "level": 3, "atk": 0, "def": 0, "card_images": [{"id": 1007}],
   "misc_info": [{"formats": ["OCG"], "views": 10, "question_atk": 1}]},
  {"id": 1008, "name": "Workshop Skill", "type": "Skill Card", "humanReadableCardType": "Skill Card",
   "frameType": "skill", "desc": "Speed Duel only.", "race": "Kaiba", "card_images": [{"id": 1008}],
   "misc_info": [{"formats": ["Speed Duel"], "views": 5}]}
]}"#;

fn store() -> CardStore {
    let cards = import::parse_cardinfo(SAMPLE.as_bytes()).unwrap();
    let mut store = CardStore::open_in_memory().unwrap();
    store.replace_all(&cards, "test-1").unwrap();
    store
}

fn search(store: &CardStore, text: &str) -> Vec<u32> {
    let parsed = parse_query(text);
    assert!(parsed.errors.is_empty(), "{text}: {:?}", parsed.errors);
    store.search(&parsed.query).unwrap()
}

#[test]
fn import_keeps_fields_and_version() {
    let store = store();
    assert_eq!(store.version().as_deref(), Some("test-1"));
    assert_eq!(store.count().unwrap(), 7);
    let cards = store.get_many(&[1003, 1006, 1007, 1004]).unwrap();
    assert_eq!(cards.iter().map(|c| c.id).collect::<Vec<_>>(), [1003, 1006, 1007, 1004]);
    assert_eq!(cards[0].type_line, ["Machine", "Fusion", "Effect"]);
    assert_eq!(cards[0].md_rarity.as_deref(), Some("Ultra Rare"));
    assert_eq!(cards[1].link_rating, Some(2));
    assert_eq!(cards[1].def, None);
    assert_eq!(cards[2].atk, None, "question-mark ATK is unknown");
    assert_eq!(cards[3].kind, CardKind::Spell);
    assert_eq!(cards[3].ban_tcg.as_deref(), Some("Limited"));
}

#[test]
fn text_search_matches_name_and_effect_text() {
    let store = store();
    let forge = search(&store, "forge");
    assert_eq!(forge.len(), 3);
    assert_eq!(forge[2], 1004, "cards with the word in their name rank before text-only matches");
    assert_eq!(search(&store, "\"negate the activation\""), [1003]);
    assert_eq!(search(&store, "name:anvil"), [1004]);
    assert_eq!(search(&store, "lab"), [1005], "prefix match");
}

#[test]
fn filters_combine() {
    let store = store();
    assert_eq!(search(&store, "type:fusion"), [1003]);
    assert_eq!(search(&store, "tipo:mostro attr:fire atk>=2000"), [1003]);
    assert_eq!(search(&store, "race:machine sort:atk"), [1003, 1006, 1001]);
    assert_eq!(search(&store, "type:quick-play"), [1004]);
    assert_eq!(search(&store, "type:continuous t:trap"), [1005]);
    assert_eq!(search(&store, "format:md rarity:ur"), [1003, 1006], "sorted by name without text");
    assert_eq!(search(&store, "link>=2"), [1006]);
    assert_eq!(search(&store, "ban:limited"), [1004]);
    assert_eq!(search(&store, "arch:forge sort:popular"), [1003, 1001, 1004]);
}

#[test]
fn skills_and_tokens_only_when_asked() {
    let store = store();
    assert!(!search(&store, "").contains(&1008));
    assert_eq!(search(&store, "type:skill"), [1008]);
}

#[test]
fn facets_list_filter_values() {
    let facets = store().facets().unwrap();
    assert_eq!(facets.attributes, ["DARK", "EARTH", "FIRE"]);
    assert!(facets.races.contains(&"Machine".to_owned()));
    assert!(!facets.races.contains(&"Quick-Play".to_owned()));
    assert_eq!(facets.archetypes, ["Forge"]);
    assert!(facets.formats.contains(&"Master Duel".to_owned()));
}
