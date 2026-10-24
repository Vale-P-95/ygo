//! Cards placed on the field before the duel starts: Equip Cards on their
//! monsters, Extra Deck monsters with their materials. They need the
//! downloaded game data (`duelforge-cli fetch-data`, or `DUELFORGE_DATA`);
//! without it they are skipped.

use std::path::PathBuf;

use ygo_engine::consts::{location, position};
use ygo_engine::{generic_materials, Action, ActionKind, Decision, DeckList, Duel, DuelState, Engine, Opponent, Placement, Prelude, Step};

fn setup() -> Option<(Engine, DeckList)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = std::env::var_os("DUELFORGE_DATA").map(PathBuf::from).unwrap_or_else(|| root.join("data"));
    if !data.join("scripts/constant.lua").is_file() {
        eprintln!("skipped: no game data in {}", data.display());
        return None;
    }
    let engine = Engine::load(&data).expect("game data loads");
    let ydk = std::fs::read_to_string(root.join("tests/fixtures/decks/invoked-siracusa-md.ydk")).unwrap();
    Some((engine, DeckList::from_ydk(&ydk).unwrap()))
}

fn code(engine: &Engine, name: &str) -> u32 {
    engine.cards().find_by_name(name).unwrap_or_else(|| panic!("{name} in the database")).code
}

/// Your side of the duel once it waits for your first decision.
fn placed(engine: &Engine, deck: &DeckList, cards: &[Placement]) -> DuelState {
    let mut duel = Duel::goldfish_vs_placed(engine, deck, &[], 1, &Opponent::default(), cards).unwrap();
    duel.advance().unwrap();
    duel.state()
}

fn monster(code: u32, sequence: u32) -> Placement {
    Placement::new(code, location::MZONE, sequence, position::FACEUP_ATTACK)
}

fn spell(code: u32, sequence: u32) -> Placement {
    Placement::new(code, location::SZONE, sequence, position::FACEUP_ATTACK)
}

#[test]
fn an_equip_spell_goes_to_a_monster_it_can_equip() {
    let Some((engine, deck)) = setup() else { return };
    let dragon = code(&engine, "Alexandrite Dragon");
    let warrior = code(&engine, "Celtic Guardian");
    let axe = code(&engine, "Axe of Despair");
    let sword = code(&engine, "Legendary Sword");

    let state = placed(&engine, &deck, &[monster(dragon, 2), spell(axe, 1)]);
    let me = &state.players[0];
    assert_eq!(me.spells[1].as_ref().and_then(|c| c.equipped_to), Some((0, location::MZONE, 2)));
    assert_eq!(me.monsters[2].as_ref().map(|c| c.attack), Some(3000));

    // Equip only to a Warrior: to the Warrior, wherever it is in the list.
    let state = placed(&engine, &deck, &[spell(sword, 1), monster(dragon, 0), monster(warrior, 3)]);
    let me = &state.players[0];
    assert_eq!(me.spells[1].as_ref().and_then(|c| c.equipped_to), Some((0, location::MZONE, 3)));
    assert_eq!(me.monsters[0].as_ref().map(|c| c.attack), Some(2000));

    // With no Warrior it is not equipped.
    let state = placed(&engine, &deck, &[monster(dragon, 2), spell(sword, 1)]);
    assert!(state.players[0].spells[1].as_ref().is_none_or(|c| c.equipped_to.is_none()));

    // A target given wins over the strongest monster.
    let mut to_weak = spell(axe, 1);
    to_weak.equip_to = Some(3);
    let state = placed(&engine, &deck, &[monster(dragon, 2), monster(warrior, 3), to_weak]);
    assert_eq!(state.players[0].monsters[3].as_ref().map(|c| c.attack), Some(2400));
}

#[test]
fn an_extra_deck_monster_has_its_materials() {
    let Some((engine, deck)) = setup() else { return };
    let apollousa = code(&engine, "Apollousa, Bow of the Goddess");
    let attack = |p: Placement| placed(&engine, &deck, &[p]).players[0].monsters[5].as_ref().map(|c| c.attack);

    // No materials: what it was before (its ATK counts them).
    assert_eq!(attack(monster(apollousa, 5)), Some(0));
    // Generic ones: as many as its Link Rating, with different names.
    let mut generic = monster(apollousa, 5);
    generic.materials = generic_materials(&engine, apollousa);
    assert_eq!(generic.materials.len(), 4);
    assert_eq!(attack(generic.clone()), Some(3200));
    let state = placed(&engine, &deck, &[generic]);
    assert_eq!(state.players[0].grave.len(), 4, "the new materials go to the GY");

    // Materials already in the GY are used, not made again.
    let elf = code(&engine, "Mystical Elf");
    let dragon = code(&engine, "Alexandrite Dragon");
    let mut two = monster(apollousa, 5);
    two.materials = vec![elf, dragon];
    let in_grave = |code| Placement::new(code, location::GRAVE, 0, position::FACEUP_ATTACK);
    let state = placed(&engine, &deck, &[in_grave(elf), two, in_grave(dragon)]);
    assert_eq!(state.players[0].monsters[5].as_ref().map(|c| c.attack), Some(1600));
    assert_eq!(state.players[0].grave.len(), 2);
}

const ALEISTER: u32 = 86120751;
const HAND: [u32; 5] = [ALEISTER, 76334960, 47679935, 14558128, 42141493];

/// Plays to the next free moment, giving no optional effect, and returns
/// it with its actions; `answers` gets the answers given.
fn to_free_moment(duel: &mut Duel, answers: &mut Vec<Vec<u8>>) -> Vec<Action> {
    loop {
        let Step::Decision { decision, .. } = duel.advance().unwrap() else { panic!("the turn goes on") };
        let actions = duel.actions(&decision);
        if matches!(decision, Decision::IdleCmd { .. }) {
            return actions;
        }
        let answer = actions.iter().find(|a| a.kind == ActionKind::No).or(actions.first()).unwrap().response.clone();
        duel.respond(&answer);
        answers.push(answer);
    }
}

/// Whether the hand's Aleister can be Normal Summoned at the next free moment.
fn can_normal_summon(duel: &mut Duel) -> bool {
    to_free_moment(duel, &mut Vec::new())
        .iter()
        .any(|a| a.kind == ActionKind::NormalSummon && a.card.is_some_and(|c| c.code == ALEISTER && c.location == location::HAND))
}

#[test]
fn a_turn_goes_on_with_a_card_more_and_its_normal_summon_used() {
    let Some((engine, deck)) = setup() else { return };
    // With nothing played, the card joining the hand can be Normal Summoned.
    let joining = Prelude { answers: Vec::new(), joining: vec![ALEISTER] };
    let mut duel = Duel::goldfish_against_from(&engine, &deck, &HAND, 1, None, &[], &joining).unwrap();
    assert_eq!(duel.count(0, location::HAND), 6);
    assert_eq!(duel.count(0, location::DECK), 34);
    assert!(can_normal_summon(&mut duel));

    // The Normal Summon of the line played first stays used.
    let mut duel = Duel::goldfish(&engine, &deck, &HAND, 1).unwrap();
    let mut answers = Vec::new();
    let summon = to_free_moment(&mut duel, &mut answers)
        .into_iter()
        .find(|a| a.kind == ActionKind::NormalSummon && a.card.is_some_and(|c| c.code == ALEISTER))
        .expect("Aleister can be Normal Summoned");
    answers.push(summon.response);
    let prelude = Prelude { answers, joining: vec![ALEISTER] };
    let mut duel = Duel::goldfish_against_from(&engine, &deck, &HAND, 1, None, &[], &prelude).unwrap();
    assert!(!can_normal_summon(&mut duel));
    assert_eq!(duel.count(0, location::MZONE), 1);

    // A card not in the Deck cannot join.
    let missing = Prelude { answers: Vec::new(), joining: vec![code(&engine, "Mystical Elf")] };
    assert!(Duel::goldfish_against_from(&engine, &deck, &HAND, 1, None, &[], &missing).is_err());
}

#[test]
fn the_opponents_board_is_placed_on_its_side() {
    let Some((engine, deck)) = setup() else { return };
    let dragon = code(&engine, "Alexandrite Dragon");
    let axe = code(&engine, "Axe of Despair");
    let apollousa = code(&engine, "Apollousa, Bow of the Goddess");
    let grave = Placement::new(dragon, location::GRAVE, 0, position::FACEUP_ATTACK);
    let link = Placement { materials: generic_materials(&engine, apollousa), ..monster(apollousa, 5) };
    let axe_on_dragon = Placement { equip_to: Some(2), ..spell(axe, 1) };
    let opponent = Opponent { placed: vec![monster(dragon, 2), axe_on_dragon, link, grave], ..Opponent::default() };
    let mut duel = Duel::goldfish_vs_placed(&engine, &deck, &[], 1, &opponent, &[]).unwrap();
    duel.advance().unwrap();
    let state = duel.state();
    let (me, them) = (&state.players[0], &state.players[1]);
    assert!(me.monsters.iter().chain(&me.spells).all(Option::is_none), "nothing on your side");
    // Equipped on its side, and Apollousa with generic materials of its own.
    assert_eq!(them.spells[1].as_ref().and_then(|c| c.equipped_to), Some((1, location::MZONE, 2)));
    assert_eq!(them.monsters[2].as_ref().map(|c| c.attack), Some(3000));
    assert!(them.monsters.iter().flatten().any(|c| c.code == apollousa && c.attack > 0), "{:?}", them.monsters);
    assert!(them.grave.contains(&dragon));
}

#[test]
fn going_second_your_turn_is_the_third_with_a_battle_phase() {
    let Some((engine, deck)) = setup() else { return };
    let dragon = code(&engine, "Alexandrite Dragon");
    let rest: Vec<u32> = deck.main.iter().copied().filter(|c| !HAND.contains(c)).take(10).collect();
    let opponent = Opponent {
        hand: rest[..2].to_vec(),
        deck: rest[2..].to_vec(),
        placed: vec![monster(dragon, 2)],
        went_first: true,
        ..Opponent::default()
    };
    let mut duel = Duel::goldfish_against_placed(&engine, &deck, &HAND, 1, Some(&opponent), &[]).unwrap();
    let actions = to_free_moment(&mut duel, &mut Vec::new());
    let (turn, player, _) = duel.turn_info();
    assert_eq!((turn, player), (3, 0));
    // Turns 1 and 2 passed with nothing done, and no one drew.
    let state = duel.state();
    assert_eq!(state.players[0].hand.len(), HAND.len());
    assert_eq!((state.players[1].hand.len(), state.players[1].deck.len()), (2, 8));
    assert!(state.players[1].monsters[2].as_ref().is_some_and(|c| c.code == dragon));
    assert!(actions.iter().any(|a| a.kind == ActionKind::ToBattlePhase), "{actions:?}");
    // The duel ends when the opponent's next turn begins.
    assert_eq!(duel.last_turn(), Some(3));
}
