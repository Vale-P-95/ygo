//! Hand-picked card groups behind the catalog's quick filters (`tag:`).
//! Names are matched exactly against the card data; a name that does not
//! match simply finds nothing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tag {
    /// Monsters (and Traps) used from the hand on the opponent's turn.
    Handtrap,
    /// Generic cards played across many decks.
    Staple,
    /// Cards that clear or get through an established board.
    Breaker,
    /// The user's favourite cards (stored in `user.db`).
    Favorite,
}

impl Tag {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "handtrap" | "handtraps" | "ht" => Tag::Handtrap,
            "staple" | "staples" => Tag::Staple,
            "breaker" | "breakers" | "board-breaker" | "boardbreaker" => Tag::Breaker,
            "fav" | "favorite" | "favorites" | "preferita" | "preferite" => Tag::Favorite,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Tag::Handtrap => "Handtrap",
            Tag::Staple => "Staple",
            Tag::Breaker => "Board breaker",
            Tag::Favorite => "★ Preferite",
        }
    }

    /// Card names in the group (empty for favourites, which are per user).
    pub fn names(self) -> &'static [&'static str] {
        match self {
            Tag::Handtrap => HANDTRAPS,
            Tag::Staple => STAPLES,
            Tag::Breaker => BREAKERS,
            Tag::Favorite => &[],
        }
    }
}

const HANDTRAPS: &[&str] = &[
    "Ash Blossom & Joyous Spring",
    "Effect Veiler",
    "Infinite Impermanence",
    "Droll & Lock Bird",
    "Maxx \"C\"",
    "Mulcharmy Fuwalos",
    "Mulcharmy Purulia",
    "Mulcharmy Meowls",
    "Nibiru, the Primal Being",
    "Ghost Belle & Haunted Mansion",
    "Ghost Ogre & Snow Rabbit",
    "Ghost Mourner & Moonlit Chill",
    "Ghost Sister & Spooky Dogwood",
    "Ghost Reaper & Winter Cherries",
    "D.D. Crow",
    "PSY-Framegear Gamma",
    "Dimension Shifter",
    "Artifact Lancea",
    "Skull Meister",
    "Contact \"C\"",
    "Retaliating \"C\"",
    "Bystial Magnamhut",
    "Bystial Druiswurm",
    "Token Collector",
    "Herald of Orange Light",
    "Gnomaterial",
    "Dominus Impulse",
    "Dominus Purge",
    "Dominus Spark",
];

const STAPLES: &[&str] = &[
    "Ash Blossom & Joyous Spring",
    "Infinite Impermanence",
    "Effect Veiler",
    "Droll & Lock Bird",
    "Mulcharmy Fuwalos",
    "Mulcharmy Purulia",
    "Mulcharmy Meowls",
    "Maxx \"C\"",
    "Nibiru, the Primal Being",
    "Ghost Belle & Haunted Mansion",
    "Ghost Ogre & Snow Rabbit",
    "D.D. Crow",
    "Dimension Shifter",
    "Artifact Lancea",
    "Bystial Magnamhut",
    "Bystial Druiswurm",
    "Called by the Grave",
    "Crossout Designator",
    "Forbidden Droplet",
    "Triple Tactics Talent",
    "Triple Tactics Thrust",
    "Dark Ruler No More",
    "Lightning Storm",
    "Evenly Matched",
    "Harpie's Feather Duster",
    "Raigeki",
    "Pot of Prosperity",
    "Pot of Desires",
    "Pot of Extravagance",
    "Pot of Duality",
    "Cosmic Cyclone",
    "Book of Moon",
    "Solemn Judgment",
    "Solemn Strike",
    "Super Polymerization",
    "Accesscode Talker",
    "I:P Masquerena",
    "S:P Little Knight",
    "Knightmare Unicorn",
    "Knightmare Phoenix",
    "Baronne de Fleur",
    "Underworld Goddess of the Closed World",
    "Divine Arsenal AA-ZEUS - Sky Thunder",
    "Number 41: Bagooska the Terribly Tired Tapir",
    "Salamangreat Almiraj",
    "Linkuriboh",
    "Chaos Angel",
];

const BREAKERS: &[&str] = &[
    "Dark Ruler No More",
    "Lightning Storm",
    "Evenly Matched",
    "Harpie's Feather Duster",
    "Raigeki",
    "Dark Hole",
    "Heavy Storm",
    "Forbidden Droplet",
    "Triple Tactics Talent",
    "Super Polymerization",
    "Cosmic Cyclone",
    "Twin Twisters",
    "Mystical Space Typhoon",
    "Book of Eclipse",
    "Interrupted Kaiju Slumber",
    "Gameciel, the Sea Turtle Kaiju",
    "Dogoran, the Mad Flame Kaiju",
    "Radian, the Multidimensional Kaiju",
    "Kumongous, the Sticky String Kaiju",
    "Gadarla, the Mystery Dust Kaiju",
    "Jizukiru, the Star Destroying Kaiju",
    "Lava Golem",
    "Ignister Prominence, the Blasting Dracoslayer",
    "Nibiru, the Primal Being",
];
