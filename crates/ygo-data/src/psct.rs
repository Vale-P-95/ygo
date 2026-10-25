//! Reading a card's text by the punctuation of Konami's Problem-Solving Card
//! Text: before a colon the condition or the moment of activation, before a
//! semicolon what is done on activation (a cost, or targets), after it the
//! action; sentences of their own for the once-per-turn limits and the
//! summoning restrictions; the first line of an Extra Deck monster for its
//! materials. It does not understand the text: what it does not recognise
//! is left without a part. Plan: `docs/plans/anatomia-effetti.md` §2.2.

use serde::Serialize;

/// What a piece of the text says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Materials,
    Condition,
    Cost,
    Action,
    Restriction,
    Limit,
}

/// A piece of the text, in order: the pieces put together give the text back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    pub text: String,
    pub part: Option<Part>,
    /// The effect the piece belongs to, counted from 0, in the order of the
    /// text; `None` for materials, limits, restrictions and spaces.
    pub effect: Option<usize>,
}

/// A card text read by its punctuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub pieces: Vec<Piece>,
    pub effects: Vec<TextEffect>,
}

/// One effect of the text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEffect {
    /// Activated ("You can", a condition with a colon, a semicolon), not
    /// one that just applies.
    pub activated: bool,
    /// What it says it does, by the names of the engine's categories
    /// (`ygo_engine::consts::category::NAMES`).
    pub categories: Vec<&'static str>,
}

/// Verbs that do something to a card rather than set the effect up: a text
/// before a semicolon with one of them (and no target) is a cost.
const COST_VERBS: &[&str] = &["discard", "tribute", "pay ", "banish", "send", "detach", "shuffle", "reveal", "remove", "return", "place"];

/// What kind of card the text is on, as far as reading it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    Monster,
    /// A Fusion, Synchro, Xyz or Link Monster: its first line lists its
    /// materials.
    ExtraDeck,
    /// A Spell or Trap Card: its first effect is its activation.
    SpellTrap,
}

/// Reads `text`, the text of a card of `frame`.
pub fn read(text: &str, frame: Frame) -> Reading {
    let mut reader =
        Reader { pieces: Vec::new(), effects: Vec::new(), open: false, bullets_part: Part::Action, spell_trap: frame == Frame::SpellTrap };
    let text = text.replace("\r\n", "\n");
    let mut lines = text.split_inclusive('\n').peekable();
    if frame == Frame::ExtraDeck {
        if let Some(first) = lines.next_if(|l| !l.contains(':') && !l.contains(';') && l.trim_end().ends_with(|c: char| c != '.')) {
            let (body, end) = split_newline(first);
            reader.push(body, Some(Part::Materials), None);
            reader.push(end, None, None);
        }
    }
    for line in lines {
        let (body, end) = split_newline(line);
        let trimmed = body.trim_start();
        if trimmed.starts_with('●') || trimmed.starts_with('•') {
            // A list: the conditions the effect answers ("any of these
            // effects"), or the actions to choose from.
            let effect = reader.effects.len().checked_sub(1);
            reader.push(body, Some(reader.bullets_part), effect);
        } else if trimmed.starts_with('*') || trimmed.starts_with('(') && trimmed.ends_with(')') && !trimmed.contains(':') {
            // Notes such as "* The above text is unofficial".
            reader.push(body, None, None);
        } else {
            for sentence in sentences(body) {
                reader.sentence(sentence);
            }
        }
        reader.push(end, None, None);
    }
    Reading { pieces: reader.pieces, effects: reader.effects }
}

struct Reader {
    pieces: Vec<Piece>,
    effects: Vec<TextEffect>,
    /// The last sentence was part of an effect another sentence can go on.
    open: bool,
    bullets_part: Part,
    spell_trap: bool,
}

impl Reader {
    fn push(&mut self, text: &str, part: Option<Part>, effect: Option<usize>) {
        if text.is_empty() {
            return;
        }
        let part = part.filter(|_| !text.trim().is_empty());
        let effect = effect.filter(|_| part.is_some());
        match self.pieces.last_mut() {
            Some(last) if last.part == part && last.effect == effect => last.text.push_str(text),
            _ => self.pieces.push(Piece { text: text.to_owned(), part, effect }),
        }
    }

    fn sentence(&mut self, sentence: &str) {
        let lead = sentence.len() - sentence.trim_start().len();
        self.push(&sentence[..lead], None, None);
        let s = &sentence[lead..];
        let lower = s.to_lowercase();
        if is_limit(&lower) {
            self.push(s, Some(Part::Limit), None);
            self.open = false;
            return;
        }
        if is_restriction(&lower) {
            self.push(s, Some(Part::Restriction), None);
            self.open = false;
            return;
        }
        let colon = find_outside_quotes(s, ':');
        let semicolon = find_outside_quotes(s, ';').filter(|&i| colon.is_none_or(|c| i > c));
        // "Then, …", "It cannot attack…": a sentence about what the one
        // before did.
        let goes_on = self.open
            && colon.is_none()
            && semicolon.is_none()
            && ["then", "it ", "its ", "they ", "that ", "those ", "if you do", "this effect", "also,", "otherwise"]
                .iter()
                .any(|w| lower.starts_with(w));
        let effect = if goes_on {
            self.effects.len() - 1
        } else {
            // Old texts say "…, you can" or "Once per turn, you can"; a Spell
            // or Trap Card's first effect is its activation; "You can Special
            // Summon this card by…" is a way to Summon it, not an effect.
            let procedure = colon.is_none() && semicolon.is_none() && lower.contains("summon this card");
            let activated = !procedure
                && (colon.is_some()
                    || semicolon.is_some()
                    || lower.starts_with("you can")
                    || lower.contains(", you can")
                    || self.spell_trap && self.effects.is_empty());
            self.effects.push(TextEffect { activated, categories: Vec::new() });
            self.effects.len() - 1
        };
        let mut rest = s;
        if let Some(c) = colon {
            let condition = &s[..=c];
            self.push(condition, Some(Part::Condition), Some(effect));
            let condition = condition.to_lowercase();
            self.bullets_part = if condition.contains("these") || condition.contains("following") { Part::Condition } else { Part::Action };
            rest = &s[c + 1..];
        }
        if let Some(i) = semicolon {
            let start = s.len() - rest.len();
            let setup = &s[start..=i];
            let lowered = setup.to_lowercase();
            let cost = !lowered.contains("target") && !lowered.contains("declare") && COST_VERBS.iter().any(|v| lowered.contains(v));
            self.push(setup, Some(if cost { Part::Cost } else { Part::Action }), Some(effect));
            rest = &s[i + 1..];
        }
        self.push(rest, Some(Part::Action), Some(effect));
        let verbs = verbs(&s[s.len() - rest.len()..].to_lowercase());
        let list = &mut self.effects[effect].categories;
        for verb in verbs {
            if !list.contains(&verb) {
                list.push(verb);
            }
        }
        self.open = true;
    }
}

fn split_newline(line: &str) -> (&str, &str) {
    match line.strip_suffix('\n') {
        Some(body) => (body, "\n"),
        None => (line, ""),
    }
}

/// The byte index of `wanted` outside double quotes (card names).
fn find_outside_quotes(s: &str, wanted: char) -> Option<usize> {
    let mut quoted = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' | '“' | '”' => quoted = !quoted,
            c if c == wanted && !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

/// The sentences of a line, each with the spaces before it: a period
/// outside quotes followed by a space or the end ends one.
fn sentences(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' | '“' | '”' => quoted = !quoted,
            '.' if !quoted && chars.peek().is_none_or(|&(_, next)| next == ' ') => {
                out.push(&line[start..=i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < line.len() {
        out.push(&line[start..]);
    }
    out
}

fn is_limit(lower: &str) -> bool {
    lower.starts_with("you can only")
        && (lower.contains("once per turn")
            || lower.contains("once per duel")
            || lower.contains("per turn")
            || lower.contains("once that turn")
            || lower.contains("once per chain"))
}

fn is_restriction(lower: &str) -> bool {
    lower.starts_with("you cannot")
        || lower.starts_with("must be ")
        || lower.starts_with("must first be ")
        || lower.starts_with("cannot be normal summoned")
        || lower.starts_with("cannot be special summoned")
        || (lower.starts_with("for the rest of this turn") && lower.contains("cannot"))
}

/// The engine categories an action names.
fn verbs(action: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut add = |on: bool, name: &'static str| {
        if on {
            out.push(name);
        }
    };
    add(action.contains("banish"), "banish");
    add(action.contains("negate the activation") || action.contains("negate the summon"), "negate activation");
    add(action.contains("negate") && !action.contains("negate the activation"), "negate effect");
    add(action.contains("destroy"), "destroy");
    add(action.contains("special summon"), "Special Summon");
    add(action.contains("draw "), "draw");
    add(action.contains("from your deck to your hand") || action.contains("from your deck to the hand"), "search");
    add(action.contains("to your hand") || action.contains("to the hand"), "to hand");
    add(action.contains("into the deck") || action.contains("into the extra deck") || action.contains("into the main deck"), "to deck");
    add(action.contains("send") && action.contains("to the gy"), "to GY");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(reading: &Reading) -> Vec<(Part, &str)> {
        reading.pieces.iter().filter_map(|p| Some((p.part?, p.text.trim()))).collect()
    }

    // Texts written for the tests in the style of the cards: no card text
    // of Konami in the repository.

    #[test]
    fn a_handtrap() {
        let text = "When your opponent activates a card or effect that includes this effect (Quick Effect): You can discard this card; negate that effect.\r\n● Add a card from the Deck to the hand.\r\nYou can only use this effect of \"Test Sprite\" once per turn.";
        let r = read(text, Frame::Monster);
        assert_eq!(
            parts(&r),
            [
                (Part::Condition, "When your opponent activates a card or effect that includes this effect (Quick Effect):"),
                (Part::Cost, "You can discard this card;"),
                (Part::Action, "negate that effect."),
                (Part::Action, "● Add a card from the Deck to the hand."),
                (Part::Limit, "You can only use this effect of \"Test Sprite\" once per turn."),
            ]
        );
        assert_eq!(r.effects, [TextEffect { activated: true, categories: vec!["negate effect"] }]);
        assert_eq!(r.pieces.iter().map(|p| p.text.as_str()).collect::<String>(), text.replace("\r\n", "\n"));
    }

    #[test]
    fn a_list_of_conditions() {
        let r = read("If a card or effect that includes any of these effects is activated (Quick Effect): You can negate it.\n● Special Summon from the Deck.", Frame::Monster);
        assert_eq!(r.pieces.iter().find(|p| p.text.starts_with('●')).unwrap().part, Some(Part::Condition));
    }

    #[test]
    fn an_extra_deck_monster() {
        let text = "2+ Fusion Monsters with different Attributes\nIf this card is Fusion Summoned: You can banish cards from the field, then you can banish up to 3 cards from your opponent's Extra Deck. (Quick Effect): You can declare 1 Type; monsters on the field become that Type. You can only use 1 \"Test. Aeon\" effect per turn.";
        let r = read(text, Frame::ExtraDeck);
        let p = parts(&r);
        assert_eq!(p[0], (Part::Materials, "2+ Fusion Monsters with different Attributes"));
        assert_eq!(p[1], (Part::Condition, "If this card is Fusion Summoned:"));
        assert_eq!(p[2].0, Part::Action);
        assert_eq!(p[3], (Part::Condition, "(Quick Effect):"));
        // Declaring is how the effect starts, not a cost.
        assert_eq!(p[4], (Part::Action, "You can declare 1 Type; monsters on the field become that Type."));
        // A period inside a quoted name does not end the sentence.
        assert_eq!(p.last().unwrap(), &(Part::Limit, "You can only use 1 \"Test. Aeon\" effect per turn."));
        assert_eq!(r.effects.len(), 2);
        assert_eq!(r.effects[0].categories, ["banish"]);
    }

    #[test]
    fn targets_restrictions_and_sentences_that_go_on() {
        let text = "Cannot be Special Summoned from the GY. You can target 1 monster on the field; destroy it. Then, draw 1 card. You cannot Special Summon monsters from the Extra Deck, except Fiend monsters.";
        let r = read(text, Frame::Monster);
        let p = parts(&r);
        assert_eq!(p[0].0, Part::Restriction);
        assert_eq!(p[1], (Part::Action, "You can target 1 monster on the field; destroy it."));
        assert_eq!(r.pieces.iter().filter(|p| p.effect == Some(0)).count(), 2, "\"Then, draw\" goes on the same effect");
        assert_eq!(r.effects, [TextEffect { activated: true, categories: vec!["destroy", "draw"] }]);
        assert_eq!(p.last().unwrap().0, Part::Restriction);
    }

    #[test]
    fn what_is_activated() {
        let r = read("You can Special Summon this card from your hand by discarding 1 card. Once per turn, you can draw 1 card. Monsters you control gain 100 ATK.", Frame::Monster);
        assert_eq!(r.effects.iter().map(|e| e.activated).collect::<Vec<_>>(), [false, true, false]);
        let r = read("Destroy 1 monster on the field.", Frame::SpellTrap);
        assert!(r.effects[0].activated);
    }
}
