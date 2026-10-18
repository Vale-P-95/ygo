//! Deck lists in the `.ydk` format used by EDOPro/YGOPro.

use crate::Error;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeckList {
    pub main: Vec<u32>,
    pub extra: Vec<u32>,
    pub side: Vec<u32>,
}

impl DeckList {
    /// Parses `.ydk` text: `#main`, `#extra` and `!side` sections with one
    /// passcode per line; other `#` lines are comments.
    pub fn from_ydk(text: &str) -> Result<Self, Error> {
        #[derive(Clone, Copy)]
        enum Section {
            None,
            Main,
            Extra,
            Side,
        }
        let mut deck = DeckList::default();
        let mut section = Section::None;
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            match line {
                "" => continue,
                "#main" => section = Section::Main,
                "#extra" => section = Section::Extra,
                "!side" => section = Section::Side,
                _ if line.starts_with('#') || line.starts_with('!') => {}
                _ => {
                    let code: u32 = line.parse().map_err(|_| Error::Data(format!("line {}: invalid passcode {line:?}", n + 1)))?;
                    match section {
                        Section::Main => deck.main.push(code),
                        Section::Extra => deck.extra.push(code),
                        Section::Side => deck.side.push(code),
                        Section::None => return Err(Error::Data(format!("line {}: passcode outside a section", n + 1))),
                    }
                }
            }
        }
        Ok(deck)
    }

    pub fn to_ydk(&self) -> String {
        let mut out = String::from("#created by DuelForge\n#main\n");
        for (header, cards) in [("", &self.main), ("#extra\n", &self.extra), ("!side\n", &self.side)] {
            out.push_str(header);
            for code in cards {
                out.push_str(&code.to_string());
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_comments() {
        let deck = DeckList::from_ydk("#created by x\n#main\n1\n1\n2\n#extra\n3\n!side\n4\n").unwrap();
        assert_eq!(deck.main, vec![1, 1, 2]);
        assert_eq!(deck.extra, vec![3]);
        assert_eq!(deck.side, vec![4]);
        assert_eq!(DeckList::from_ydk(&deck.to_ydk()).unwrap(), deck);
    }

    #[test]
    fn parses_fixture_decks() {
        for file in ["invoked-siracusa-md.ydk", "invoked-siracusa-ewcq2026.ydk"] {
            let path = format!("{}/../../tests/fixtures/decks/{file}", env!("CARGO_MANIFEST_DIR"));
            let deck = DeckList::from_ydk(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert_eq!((deck.main.len(), deck.extra.len(), deck.side.len()), (40, 15, 15), "{file}");
        }
    }

    #[test]
    fn rejects_garbage() {
        assert!(DeckList::from_ydk("#main\nabc\n").is_err());
        assert!(DeckList::from_ydk("123\n").is_err());
    }
}
