import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  api as ApiType,
  Card,
  CardKind,
  Checkpoint,
  Chip,
  ComboOutcome,
  ComboProgress,
  ComboScore,
  Cond,
  DataStatus,
  Deck,
  DeckBox,
  DeckSummary,
  Facets,
  HandStats,
  ImageSize,
  Imported,
  LabAnalysis,
  LabOverview,
  Legality,
  MatchEntry,
  MatchLog,
  NamedRecord,
  OptimizeResult,
  Progress,
  Recipe,
  RoleGuess,
  SavedSearch,
  SearchResult,
  ShelfDeck,
  SimRule,
  WinRecord,
} from "./api";
import { pAtLeastOne, pBoth } from "./probability";

type Api = typeof ApiType;

interface CardSeed {
  id: number;
  name: string;
  kind: CardKind;
  frame: string;
  humanType: string;
  typeLine: string[];
  desc: string;
  race: string;
  attribute?: string | null;
  archetype?: string | null;
  level?: number | null;
  atk?: number | null;
  def?: number | null;
  linkRating?: number | null;
  linkMarkers?: string[];
  mdRarity?: string | null;
  banMd?: string | null;
  banTcg?: string | null;
  banOcg?: string | null;
  handtrap?: boolean;
}

function makeCard(s: CardSeed): Card {
  return {
    id: s.id,
    name: s.name,
    kind: s.kind,
    frame: s.frame,
    humanType: s.humanType,
    typeLine: s.typeLine,
    desc: s.desc,
    pendDesc: null,
    race: s.race,
    attribute: s.attribute ?? null,
    archetype: s.archetype ?? null,
    level: s.level ?? null,
    atk: s.atk ?? null,
    def: s.def ?? null,
    scale: null,
    linkRating: s.linkRating ?? null,
    linkMarkers: s.linkMarkers ?? [],
    mdRarity: s.mdRarity ?? "UR",
    formats: ["Master Duel", "TCG", "OCG"],
    tcgDate: "2023-10-20",
    ocgDate: "2023-07-22",
    banTcg: s.banTcg ?? null,
    banOcg: s.banOcg ?? null,
    banMd: s.banMd ?? null,
    imageIds: [s.id],
    views: 150000,
  };
}

/**
 * Synthetic catalog with invented placeholder descriptions (never official
 * card text, per repository policy).
 */
const CARD_SEEDS: CardSeed[] = [
  // Snake-Eye & Diabellstar engine
  {
    id: 98159737,
    name: "Snake-Eye Ash",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Pyro", "Effect"],
    desc: "[Mock text] Searches a Level 1 FIRE monster on summon; can send itself and another face-up card to summon a Snake-Eye monster from Deck.",
    race: "Pyro",
    attribute: "FIRE",
    archetype: "Snake-Eye",
    level: 1,
    atk: 1200,
    def: 200,
    mdRarity: "UR",
    banMd: "Limited",
  },
  {
    id: 81674782,
    name: "Snakey-Eye Oak",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Pyro", "Effect"],
    desc: "[Mock text] Recovers a Level 1 FIRE monster from Graveyard or banished on summon; sends 2 face-up cards to Special Summon from Deck.",
    race: "Pyro",
    attribute: "FIRE",
    archetype: "Snake-Eye",
    level: 1,
    atk: 900,
    def: 200,
    mdRarity: "N",
  },
  {
    id: 45298492,
    name: "Snake-Eyes Poplar",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Pyro", "Effect"],
    desc: "[Mock text] Special Summons itself when added to hand; searches a Snake-Eye Spell/Trap on summon and places a monster in the Spell & Trap Zone when sent to GY.",
    race: "Pyro",
    attribute: "FIRE",
    archetype: "Snake-Eye",
    level: 1,
    atk: 700,
    def: 200,
    mdRarity: "UR",
    banMd: "Limited",
  },
  {
    id: 11264180,
    name: "Snake-Eyes Flamberge Dragon",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Dragon", "Effect"],
    desc: "[Mock text] Places a monster in the Spell & Trap Zone or Special Summons one from it; revives two Level 1 FIRE monsters when sent from hand or field to GY.",
    race: "Dragon",
    attribute: "FIRE",
    archetype: "Snake-Eye",
    level: 8,
    atk: 3000,
    def: 2500,
    mdRarity: "UR",
  },
  {
    id: 72270339,
    name: "Diabellstar the Black Witch",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Spellcaster", "Effect"],
    desc: "[Mock text] Special Summons itself by sending a card from hand or field to GY; sets a Sinful Spoils Spell/Trap directly from the Deck.",
    race: "Spellcaster",
    attribute: "DARK",
    archetype: "Diabellstar",
    level: 7,
    atk: 2500,
    def: 2000,
    mdRarity: "UR",
  },
  {
    id: 58074572,
    name: "Original Sinful Spoils - Snake-Eye",
    kind: "spell",
    frame: "spell",
    humanType: "Normal Spell",
    typeLine: ["Normal Spell"],
    desc: "[Mock text] Sends 1 face-up card you control to GY to Special Summon a Level 1 FIRE monster from hand or Deck; banishes from GY to search.",
    race: "Normal",
    archetype: "Sinful Spoils",
    mdRarity: "UR",
  },
  {
    id: 34267821,
    name: "WANTED: Seeker of Sinful Spoils",
    kind: "spell",
    frame: "spell",
    humanType: "Quick-Play Spell",
    typeLine: ["Quick-Play Spell"],
    desc: "[Mock text] Adds Diabellstar from Deck or GY to hand; banishes from GY to recycle a Sinful Spoils Spell/Trap and draw 1 card.",
    race: "Quick-Play",
    archetype: "Sinful Spoils",
    mdRarity: "UR",
    banMd: "Limited",
  },
  {
    id: 69105797,
    name: "Divine Temple of the Snake-Eye",
    kind: "spell",
    frame: "spell",
    humanType: "Field Spell",
    typeLine: ["Field Spell"],
    desc: "[Mock text] Places a Snake-Eye monster in the Spell & Trap Zone on activation, boosts Level 1 FIRE monsters, and summons a monster from S/T Zone when opponent summons.",
    race: "Field",
    archetype: "Snake-Eye",
    mdRarity: "UR",
  },
  {
    id: 97045737,
    name: "Bonfire",
    kind: "spell",
    frame: "spell",
    humanType: "Normal Spell",
    typeLine: ["Normal Spell"],
    desc: "[Mock text] Adds 1 Level 4 or lower Pyro monster from your Deck to your hand.",
    race: "Normal",
    mdRarity: "UR",
    banMd: "Semi-Limited",
  },
  // Fire King package
  {
    id: 13002461,
    name: "Legendary Fire King Ponix",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Winged Beast", "Effect"],
    desc: "[Mock text] Special Summons itself when a FIRE monster is destroyed; searches a Fire King Spell/Trap on summon and returns to hand next Standby Phase.",
    race: "Winged Beast",
    attribute: "FIRE",
    archetype: "Fire King",
    level: 1,
    atk: 500,
    def: 200,
    mdRarity: "UR",
  },
  {
    id: 75392615,
    name: "Sacred Fire King Garunix",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Winged Beast", "Effect"],
    desc: "[Mock text] Special Summons itself from hand or GY when a FIRE monster is destroyed; destroys a FIRE Beast/Beast-Warrior/Winged Beast from Deck on summon.",
    race: "Winged Beast",
    attribute: "FIRE",
    archetype: "Fire King",
    level: 8,
    atk: 2700,
    def: 1700,
    mdRarity: "UR",
  },
  {
    id: 42370001,
    name: "Fire King Avatar Kirin",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Beast", "Effect"],
    desc: "[Mock text] Quick Effect in Main Phase: destroys another FIRE monster in hand or field to Special Summon itself; revives a Fire King and destroys a card when destroyed.",
    race: "Beast",
    attribute: "FIRE",
    archetype: "Fire King",
    level: 8,
    atk: 2400,
    def: 200,
    mdRarity: "UR",
  },
  {
    id: 57554544,
    name: "Fire King Sanctuary",
    kind: "spell",
    frame: "spell",
    humanType: "Continuous Spell",
    typeLine: ["Continuous Spell"],
    desc: "[Mock text] Places Fire King Island from Deck on activation, protects your Field Spell, and performs an Xyz Summon during the opponent's turn.",
    race: "Continuous",
    archetype: "Fire King",
    mdRarity: "SR",
  },
  {
    id: 57554545,
    name: "Fire King Island",
    kind: "spell",
    frame: "spell",
    humanType: "Field Spell",
    typeLine: ["Field Spell"],
    desc: "[Mock text] Destroys 1 monster in hand or field to search a Fire King monster; destroys all monsters you control if it leaves the field.",
    race: "Field",
    archetype: "Fire King",
    mdRarity: "R",
  },
  // Handtraps & Staples
  {
    id: 14558127,
    name: "Ash Blossom & Joyous Spring",
    kind: "monster",
    frame: "effect",
    humanType: "Tuner Effect Monster",
    typeLine: ["Zombie", "Tuner", "Effect"],
    desc: "[Mock text] Quick Effect from hand: discards itself to negate any effect that adds from Deck, Special Summons from Deck, or sends from Deck to GY.",
    race: "Zombie",
    attribute: "FIRE",
    level: 3,
    atk: 0,
    def: 1800,
    mdRarity: "UR",
    handtrap: true,
  },
  {
    id: 23434538,
    name: 'Maxx "C"',
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Insect", "Effect"],
    desc: "[Mock text] Quick Effect from hand: sends itself to GY; draw 1 card each time the opponent Special Summons this turn.",
    race: "Insect",
    attribute: "EARTH",
    level: 2,
    atk: 500,
    def: 200,
    mdRarity: "UR",
    banTcg: "Forbidden",
    handtrap: true,
  },
  {
    id: 97268402,
    name: "Effect Veiler",
    kind: "monster",
    frame: "effect",
    humanType: "Tuner Effect Monster",
    typeLine: ["Spellcaster", "Tuner", "Effect"],
    desc: "[Mock text] Quick Effect during opponent's Main Phase: sends itself from hand to GY to negate the effects of 1 face-up monster opponent controls until end of turn.",
    race: "Spellcaster",
    attribute: "LIGHT",
    level: 1,
    atk: 0,
    def: 0,
    mdRarity: "UR",
    handtrap: true,
  },
  {
    id: 27204311,
    name: "Nibiru, the Primal Being",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Rock", "Effect"],
    desc: "[Mock text] Quick Effect after opponent summons 5 or more monsters in a turn: tributes all face-up monsters on field to summon itself and a Primal Being Token.",
    race: "Rock",
    attribute: "LIGHT",
    level: 11,
    atk: 3000,
    def: 600,
    mdRarity: "UR",
    handtrap: true,
  },
  {
    id: 94145021,
    name: "Droll & Lock Bird",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Spellcaster", "Effect"],
    desc: "[Mock text] Quick Effect if opponent adds a card from Deck outside the Draw Phase: sends itself from hand to GY to prevent further additions from Deck this turn.",
    race: "Spellcaster",
    attribute: "WIND",
    level: 1,
    atk: 0,
    def: 0,
    mdRarity: "SR",
    handtrap: true,
  },
  {
    id: 10045474,
    name: "Infinite Impermanence",
    kind: "trap",
    frame: "trap",
    humanType: "Normal Trap",
    typeLine: ["Normal Trap"],
    desc: "[Mock text] Targets 1 face-up monster opponent controls and negates its effects; can be activated from hand if you control no cards.",
    race: "Normal",
    mdRarity: "UR",
    handtrap: true,
  },
  {
    id: 24224830,
    name: "Called by the Grave",
    kind: "spell",
    frame: "spell",
    humanType: "Quick-Play Spell",
    typeLine: ["Quick-Play Spell"],
    desc: "[Mock text] Banishes 1 monster from opponent's GY and negates effects of monsters with the same original name until the end of the next turn.",
    race: "Quick-Play",
    mdRarity: "UR",
    banMd: "Semi-Limited",
    banTcg: "Limited",
  },
  {
    id: 65681983,
    name: "Crossout Designator",
    kind: "spell",
    frame: "spell",
    humanType: "Quick-Play Spell",
    typeLine: ["Quick-Play Spell"],
    desc: "[Mock text] Declares 1 card name, banishes it from your Deck, and negates the effects of cards with that name until end of turn.",
    race: "Quick-Play",
    mdRarity: "UR",
    banMd: "Limited",
  },
  {
    id: 54693926,
    name: "Forbidden Droplet",
    kind: "spell",
    frame: "spell",
    humanType: "Quick-Play Spell",
    typeLine: ["Quick-Play Spell"],
    desc: "[Mock text] Sends cards from hand or field to GY to halve ATK and negate effects of that many opponent's monsters; opponent cannot respond with matching card types.",
    race: "Quick-Play",
    mdRarity: "UR",
  },
  {
    id: 14532163,
    name: "Lightning Storm",
    kind: "spell",
    frame: "spell",
    humanType: "Normal Spell",
    typeLine: ["Normal Spell"],
    desc: "[Mock text] If you control no face-up cards: destroys all Attack Position monsters or all Spells and Traps your opponent controls.",
    race: "Normal",
    mdRarity: "UR",
    banMd: "Semi-Limited",
  },
  {
    id: 15693423,
    name: "Evenly Matched",
    kind: "trap",
    frame: "trap",
    humanType: "Normal Trap",
    typeLine: ["Normal Trap"],
    desc: "[Mock text] At end of Battle Phase, forces opponent to banish cards face-down until they control the same number as you; activates from hand if you control no cards.",
    race: "Normal",
    mdRarity: "SR",
  },
  // Extra Deck Monsters
  {
    id: 27548199,
    name: "Promethean Princess, Bestower of Flames",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Fiend", "Link", "Effect"],
    desc: "[Mock text] Revives a FIRE monster from GY; while in GY, destroys 1 FIRE monster you control and 1 opponent's monster when opponent Special Summons, then revives itself.",
    race: "Fiend",
    attribute: "FIRE",
    linkRating: 3,
    linkMarkers: ["Left", "Right", "Bottom"],
    atk: 2700,
    mdRarity: "UR",
  },
  {
    id: 29301450,
    name: "S:P Little Knight",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Warrior", "Link", "Effect"],
    desc: "[Mock text] Banishes 1 card on field or GY on Link Summon using an Extra Deck monster; Quick Effect when opponent activates an effect: banishes 2 monsters until End Phase.",
    race: "Warrior",
    attribute: "DARK",
    linkRating: 2,
    linkMarkers: ["Left", "Right"],
    atk: 1600,
    mdRarity: "UR",
  },
  {
    id: 65741786,
    name: "I:P Masquerena",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Cyberse", "Link", "Effect"],
    desc: "[Mock text] Quick Effect during opponent's Main Phase: Link Summons using materials including itself; the summoned Link Monster cannot be destroyed by card effects.",
    race: "Cyberse",
    attribute: "DARK",
    linkRating: 2,
    linkMarkers: ["Bottom-Left", "Bottom-Right"],
    atk: 800,
    mdRarity: "UR",
  },
  {
    id: 4280258,
    name: "Apollousa, Bow of the Goddess",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Fairy", "Link", "Effect"],
    desc: "[Mock text] Gains 800 ATK per material used; once per Chain, loses 800 ATK to negate an opponent's monster effect activation.",
    race: "Fairy",
    attribute: "WIND",
    linkRating: 4,
    linkMarkers: ["Top", "Bottom-Left", "Bottom", "Bottom-Right"],
    atk: 2400,
    mdRarity: "UR",
    banTcg: "Forbidden",
  },
  {
    id: 87835759,
    name: "Salamangreat Raging Phoenix",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Cyberse", "Link", "Effect"],
    desc: "[Mock text] Revives itself from GY and gains ATK when a FIRE monster you control is destroyed by battle or card effect.",
    race: "Cyberse",
    attribute: "FIRE",
    archetype: "Salamangreat",
    linkRating: 4,
    linkMarkers: ["Left", "Right", "Bottom-Left", "Bottom-Right"],
    atk: 2800,
    mdRarity: "UR",
  },
  {
    id: 18940556,
    name: "Amphibious Swarmship Amblowhale",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Machine", "Link", "Effect"],
    desc: "[Mock text] Gains ATK per Link Monster in GY; revives a Link-3 or lower monster when destroyed, and banishes from GY to destroy 1 card when a Link-3 or lower is destroyed.",
    race: "Machine",
    attribute: "FIRE",
    linkRating: 4,
    linkMarkers: ["Left", "Right", "Bottom-Left", "Bottom-Right"],
    atk: 2600,
    mdRarity: "SR",
  },
  {
    id: 48815792,
    name: "Salamangreat Sunlight Wolf",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Cyberse", "Link", "Effect"],
    desc: "[Mock text] Adds a FIRE monster from GY to hand when a monster is summoned to a zone it points to.",
    race: "Cyberse",
    attribute: "FIRE",
    archetype: "Salamangreat",
    linkRating: 2,
    linkMarkers: ["Top", "Bottom"],
    atk: 1800,
    mdRarity: "SR",
  },
  {
    id: 41999284,
    name: "Linkuriboh",
    kind: "monster",
    frame: "link",
    humanType: "Link Effect Monster",
    typeLine: ["Cyberse", "Link", "Effect"],
    desc: "[Mock text] Tributes itself when opponent's monster declares an attack to reduce its ATK to 0; tributes a Level 1 monster to revive itself from GY.",
    race: "Cyberse",
    attribute: "DARK",
    linkRating: 1,
    linkMarkers: ["Bottom"],
    atk: 300,
    mdRarity: "UR",
  },
  {
    id: 84815190,
    name: "Baronne de Fleur",
    kind: "monster",
    frame: "synchro",
    humanType: "Synchro Effect Monster",
    typeLine: ["Warrior", "Synchro", "Effect"],
    desc: "[Mock text] Destroys 1 card on field once per turn; once while face-up on field, negates any activation and destroys that card.",
    race: "Warrior",
    attribute: "WIND",
    level: 10,
    atk: 3000,
    def: 2400,
    mdRarity: "UR",
    banTcg: "Forbidden",
  },
  {
    id: 50954680,
    name: "Garunix Eternity, Hyang of the Fire Kings",
    kind: "monster",
    frame: "xyz",
    humanType: "Xyz Effect Monster",
    typeLine: ["Pyro", "Xyz", "Effect"],
    desc: "[Mock text] Destroys all other monsters on the field on Xyz Summon; detaches 1 material to destroy a Spell/Trap and floats into Fire King monsters when destroyed.",
    race: "Pyro",
    attribute: "FIRE",
    archetype: "Fire King",
    level: 8,
    atk: 3000,
    def: 2000,
    mdRarity: "UR",
  },
  {
    id: 75286621,
    name: "Invoked Mechaba",
    kind: "monster",
    frame: "fusion",
    humanType: "Fusion Effect Monster",
    typeLine: ["Machine", "Fusion", "Effect"],
    desc: "[Mock text] Once per turn, when a Spell/Trap or monster effect is activated: sends a card of the same type from hand to GY to negate and banish it.",
    race: "Machine",
    attribute: "LIGHT",
    archetype: "Invoked",
    level: 9,
    atk: 2500,
    def: 2100,
    mdRarity: "SR",
  },
  // Section theme cards (from sections.ts)
  {
    id: 59811955,
    name: "Ancient Gear Workshop",
    kind: "spell",
    frame: "spell",
    humanType: "Normal Spell",
    typeLine: ["Normal Spell"],
    desc: "[Mock text] Recovers 1 Ancient Gear monster from your Graveyard to your hand.",
    race: "Normal",
    archetype: "Ancient Gear",
    mdRarity: "N",
  },
  {
    id: 60764609,
    name: "Fiendsmith Engraver",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Fiend", "Effect"],
    desc: "[Mock text] Discards itself to search a Fiendsmith Spell/Trap; shuffles another LIGHT Fiend from GY into Deck to Special Summon itself from GY.",
    race: "Fiend",
    attribute: "LIGHT",
    archetype: "Fiendsmith",
    level: 6,
    atk: 1800,
    def: 2400,
    mdRarity: "UR",
  },
  {
    id: 21744288,
    name: "Witchcrafter Schmietta",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Spellcaster", "Effect"],
    desc: "[Mock text] Tributes itself and discards a Spell to Special Summon a Witchcrafter monster from Deck; banishes from GY to mill a Witchcrafter card.",
    race: "Spellcaster",
    attribute: "FIRE",
    archetype: "Witchcrafter",
    level: 4,
    atk: 1800,
    def: 600,
    mdRarity: "SR",
  },
  {
    id: 85668449,
    name: "Brain Research Lab",
    kind: "spell",
    frame: "spell",
    humanType: "Field Spell",
    typeLine: ["Field Spell"],
    desc: "[Mock text] Grants an extra Normal Summon for Psychic monsters and replaces LP costs with counters on this card.",
    race: "Field",
    mdRarity: "R",
  },
  {
    id: 86120752,
    name: "Aleister the Invoker",
    kind: "monster",
    frame: "effect",
    humanType: "Effect Monster",
    typeLine: ["Spellcaster", "Effect"],
    desc: "[Mock text] Searches Invocation on Normal Summon or flip; boosts a Fusion Monster by 1000 ATK/DEF from hand.",
    race: "Spellcaster",
    attribute: "DARK",
    archetype: "Invoked",
    level: 4,
    atk: 1000,
    def: 1800,
    mdRarity: "SR",
  },
  {
    id: 74063034,
    name: "Invocation",
    kind: "spell",
    frame: "spell",
    humanType: "Normal Spell",
    typeLine: ["Normal Spell"],
    desc: "[Mock text] Fusion Summons using materials from hand or banishing from field/GY for an Invoked Fusion Monster; shuffles itself from GY into Deck to recover banished Aleister.",
    race: "Normal",
    archetype: "Invoked",
    mdRarity: "SR",
  },
];

const CARDS = new Map<number, Card>(CARD_SEEDS.map((s) => [s.id, makeCard(s)]));
const HANDTRAP_IDS = new Set<number>(CARD_SEEDS.filter((s) => s.handtrap).map((s) => s.id));

const FRAME_PALETTE: Record<string, [string, string, string]> = {
  effect: ["#7a3a1e", "#2d160d", "#ff9d5c"],
  fusion: ["#582a7a", "#211030", "#c98fff"],
  synchro: ["#6e7785", "#242930", "#e6eef8"],
  xyz: ["#26262e", "#0f0f14", "#ffd56b"],
  link: ["#1b447a", "#0c1d36", "#6bb8ff"],
  spell: ["#185e52", "#0b2923", "#5fe6c8"],
  trap: ["#6e2250", "#2b0d20", "#ff7ab8"],
};

function escapeXml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** SVG data URI for card images when running in a plain browser with VITE_MOCK=1. */
export function mockCardImage(id: number, size: ImageSize): string {
  const c = CARDS.get(id);
  const [c1, c2, accent] = FRAME_PALETTE[c?.frame ?? "effect"] ?? FRAME_PALETTE.effect;
  const name = escapeXml(c?.name ?? `Card #${id}`);
  const shortName = name.length > 22 ? `${name.slice(0, 20)}…` : name;
  const sub = escapeXml(
    c?.linkRating ? `LINK-${c.linkRating} · ATK ${c.atk ?? 0}` : c?.level ? `★${c.level} · ${c.atk ?? 0}/${c.def ?? 0}` : (c?.humanType ?? "Card"),
  );
  if (size === "cropped") {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="${c1}"/><stop offset="100%" stop-color="${c2}"/></linearGradient></defs><rect width="600" height="400" fill="url(#g)"/><circle cx="440" cy="180" r="140" fill="${accent}" fill-opacity="0.16"/></svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 350"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="${c1}"/><stop offset="100%" stop-color="${c2}"/></linearGradient></defs><rect width="240" height="350" rx="10" fill="url(#g)" stroke="${accent}" stroke-opacity="0.55" stroke-width="4"/><rect x="14" y="14" width="212" height="28" rx="4" fill="#000" fill-opacity="0.35"/><text x="22" y="33" fill="#fff" font-family="sans-serif" font-weight="700" font-size="13">${shortName}</text><rect x="22" y="54" width="196" height="196" rx="6" fill="#09080c" fill-opacity="0.45" stroke="${accent}" stroke-opacity="0.35"/><circle cx="120" cy="152" r="56" fill="${accent}" fill-opacity="0.22"/><text x="120" y="158" text-anchor="middle" fill="${accent}" font-family="serif" font-weight="700" font-size="22">${escapeXml((c?.archetype ?? c?.race ?? "YGO").slice(0, 10).toUpperCase())}</text><rect x="14" y="262" width="212" height="72" rx="4" fill="#000" fill-opacity="0.45"/><text x="22" y="284" fill="${accent}" font-family="sans-serif" font-weight="600" font-size="12">${sub}</text><text x="22" y="306" fill="#d8cfc4" font-family="sans-serif" font-size="10">#${id}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

let statusState: DataStatus = {
  dataDir: "C:\\dev\\ygo-gemini\\data (mock)",
  cards: { version: "147.37", count: CARDS.size },
  engine: {
    apiVersion: "11.0",
    scripts: 19315,
    scriptsVersion: "2026-10-18",
    databases: 2,
    cdbVersion: "2026-10-18",
  },
  imagesCached: CARDS.size,
  engineError: null,
  banlistMd: "2026-10-06",
};

const progressListeners = new Set<(p: Progress) => void>();
const rolesProgressListeners = new Set<(p: { done: number; total: number }) => void>();
const simProgressListeners = new Set<(c: Checkpoint) => void>();
let simCancelled = false;

const favoritesState = new Set<number>([98159737, 72270339, 27548199, 29301450]);
let savedSearchesState: SavedSearch[] = [
  { name: "Starter Fuoco Lv.1", query: "attr:fire level:1 kind:monster" },
  { name: "Handtrap comuni", query: "tag:handtrap" },
];

let decksState: Deck[] = [
  {
    id: 1,
    name: "Snake-Eye Fire King",
    format: "md",
    main: [
      // Engine & starters (23)
      98159737,
      81674782,
      81674782,
      45298492,
      11264180,
      11264180,
      72270339,
      72270339,
      72270339,
      58074572,
      34267821,
      69105797,
      69105797,
      97045737,
      97045737,
      13002461,
      13002461,
      75392615,
      42370001,
      42370001,
      42370001,
      57554544,
      57554545,
      // Handtraps & staples (17)
      14558127,
      14558127,
      14558127,
      23434538,
      23434538,
      23434538,
      97268402,
      97268402,
      27204311,
      10045474,
      10045474,
      10045474,
      24224830,
      24224830,
      65681983,
      54693926,
      54693926,
    ],
    extra: [
      41999284,
      48815792,
      65741786,
      29301450,
      27548199,
      27548199,
      87835759,
      87835759,
      18940556,
      4280258,
      50954680,
      84815190,
      75286621,
      29301450,
      48815792,
    ],
    side: [94145021, 94145021, 94145021, 14532163, 14532163, 15693423],
    maybe: [94145021, 14532163, 15693423, 84815190],
    roles: {
      starter: [98159737, 45298492, 72270339, 34267821, 97045737, 13002461],
      extender: [81674782, 58074572, 69105797, 42370001, 24224830, 65681983],
      handtrap: [14558127, 23434538, 97268402, 27204311, 10045474],
      breaker: [54693926],
      brick: [11264180, 75392615, 57554544, 57554545],
    },
    box: {
      color: "crimson",
      material: "leather",
      shape: "grimoire",
      showcase: [98159737, 27548199, 11264180],
    },
    updatedAt: "2026-10-20",
  },
  {
    id: 2,
    name: "Fiendsmith Invoked Control",
    format: "md",
    main: [
      60764609,
      60764609,
      60764609,
      86120752,
      86120752,
      86120752,
      74063034,
      74063034,
      72270339,
      72270339,
      34267821,
      58074572,
      98159737,
      45298492,
      11264180,
      97045737,
      97045737,
      14558127,
      14558127,
      14558127,
      23434538,
      23434538,
      23434538,
      97268402,
      97268402,
      97268402,
      27204311,
      27204311,
      94145021,
      94145021,
      10045474,
      10045474,
      10045474,
      24224830,
      24224830,
      65681983,
      54693926,
      54693926,
      14532163,
      14532163,
    ],
    extra: [
      75286621,
      75286621,
      84815190,
      29301450,
      29301450,
      65741786,
      4280258,
      27548199,
      87835759,
      18940556,
      48815792,
      41999284,
      50954680,
      75286621,
      65741786,
    ],
    side: [15693423, 15693423, 15693423, 94145021, 54693926, 27204311],
    maybe: [21744288, 59811955, 85668449],
    roles: {
      starter: [60764609, 86120752, 72270339, 98159737, 97045737],
      extender: [34267821, 58074572, 45298492, 24224830, 65681983],
      handtrap: [14558127, 23434538, 97268402, 27204311, 94145021, 10045474],
      breaker: [54693926, 14532163],
      brick: [74063034, 11264180],
    },
    box: {
      color: "obsidian",
      material: "metal",
      shape: "monolith",
      showcase: [60764609, 86120752, 75286621],
    },
    updatedAt: "2026-10-19",
  },
];

let nextDeckId = 3;

function toSummary(d: Deck): DeckSummary {
  const id = d.id ?? 1;
  const preview = d.box.showcase.length > 0 ? d.box.showcase.slice(0, 3) : [...new Set(d.main)].slice(0, 3);
  return {
    id,
    name: d.name,
    format: d.format,
    mainCount: d.main.length,
    extraCount: d.extra.length,
    sideCount: d.side.length,
    updatedAt: d.updatedAt ?? "2026-10-20",
    cover: preview[0] ?? null,
    preview,
    box: d.box,
  };
}

function validateDeckInternal(deck: Deck): Legality {
  const issues: Legality["issues"] = [];
  if (deck.main.length < 40 || deck.main.length > 60) {
    issues.push({
      severity: "error",
      message: `Il Main Deck ha ${deck.main.length} carte (deve averne tra 40 e 60).`,
      cards: [],
    });
  }
  if (deck.extra.length > 15) {
    issues.push({
      severity: "error",
      message: `L'Extra Deck ha ${deck.extra.length} carte (massimo 15).`,
      cards: [],
    });
  }
  const counts = new Map<number, number>();
  for (const id of [...deck.main, ...deck.extra, ...deck.side]) {
    counts.set(id, (counts.get(id) ?? 0) + 1);
  }
  for (const [id, count] of counts) {
    const c = CARDS.get(id);
    const status = deck.format === "md" ? c?.banMd : deck.format === "tcg" ? c?.banTcg : c?.banOcg;
    const limit = status === "Forbidden" ? 0 : status === "Limited" ? 1 : status === "Semi-Limited" ? 2 : 3;
    if (count > limit) {
      issues.push({
        severity: "error",
        message: `${c?.name ?? id}: ${count} copie (limite: ${limit}).`,
        cards: [id],
      });
    }
  }
  return {
    legal: issues.every((i) => i.severity !== "error"),
    issues,
    banlist: statusState.banlistMd,
  };
}

function searchCatalog(rawQuery: string): SearchResult {
  const q = rawQuery.trim();
  const chips: Chip[] = [];
  const errors: string[] = [];
  const tokens = q ? q.split(/\s+/) : [];
  const textTerms: string[] = [];
  let filterKind: string | null = null;
  let filterAttr: string | null = null;
  let filterRace: string | null = null;
  let filterArch: string | null = null;
  let filterFrame: string | null = null;
  let filterLevel: number | null = null;
  let filterHandtrap = false;

  for (const tok of tokens) {
    const lower = tok.toLowerCase();
    if (lower === "tag:handtrap") {
      filterHandtrap = true;
      chips.push({ token: tok, label: "Handtrap" });
    } else if (lower.startsWith("kind:") || lower.startsWith("type:")) {
      filterKind = lower.split(":")[1] ?? null;
      chips.push({ token: tok, label: `Tipo: ${filterKind}` });
    } else if (lower.startsWith("attr:") || lower.startsWith("attribute:")) {
      filterAttr = (lower.split(":")[1] ?? "").toUpperCase();
      chips.push({ token: tok, label: `Attributo: ${filterAttr}` });
    } else if (lower.startsWith("race:")) {
      filterRace = lower.split(":")[1] ?? null;
      chips.push({ token: tok, label: `Razza: ${filterRace}` });
    } else if (lower.startsWith("arch:") || lower.startsWith("archetype:")) {
      filterArch = lower.split(":")[1] ?? null;
      chips.push({ token: tok, label: `Archetipo: ${filterArch}` });
    } else if (lower.startsWith("frame:") || lower.startsWith("is:")) {
      filterFrame = lower.split(":")[1] ?? null;
      chips.push({ token: tok, label: `Frame: ${filterFrame}` });
    } else if (lower.startsWith("level:") || lower.startsWith("lv:") || lower.startsWith("rank:") || lower.startsWith("link:")) {
      const n = Number(lower.split(":")[1]);
      if (!Number.isNaN(n)) {
        filterLevel = n;
        chips.push({ token: tok, label: `Livello/Link: ${n}` });
      }
    } else {
      textTerms.push(lower);
    }
  }

  const matches = [...CARDS.values()].filter((c) => {
    if (filterHandtrap && !HANDTRAP_IDS.has(c.id)) return false;
    if (filterKind && c.kind !== filterKind && c.frame !== filterKind) return false;
    if (filterAttr && c.attribute?.toUpperCase() !== filterAttr) return false;
    if (filterRace && !c.race.toLowerCase().includes(filterRace)) return false;
    if (filterArch && !(c.archetype ?? "").toLowerCase().includes(filterArch)) return false;
    if (filterFrame && c.frame !== filterFrame) return false;
    if (filterLevel != null && c.level !== filterLevel && c.linkRating !== filterLevel) return false;
    for (const term of textTerms) {
      const hay = `${c.name} ${c.desc} ${c.race} ${c.archetype ?? ""} ${c.humanType}`.toLowerCase();
      if (!hay.includes(term)) return false;
    }
    return true;
  });

  return {
    ids: matches.map((c) => c.id),
    chips,
    errors,
  };
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// --- Hypergeometric helpers for Lab ---

function binom(n: number, k: number): number {
  if (k < 0 || k > n) return 0;
  if (k === 0 || k === n) return 1;
  const m = Math.min(k, n - k);
  let res = 1;
  for (let i = 1; i <= m; i++) {
    res = (res * (n - m + i)) / i;
  }
  return res;
}

function hypergeomPmf(N: number, K: number, n: number, k: number): number {
  const denom = binom(N, n);
  if (denom === 0) return 0;
  return (binom(K, k) * binom(N - K, n - k)) / denom;
}

function dist6(N: number, K: number, hand: number): number[] {
  const out = [0, 0, 0, 0, 0, 0];
  for (let k = 0; k <= hand; k++) {
    const p = hypergeomPmf(N, K, hand, k);
    out[Math.min(5, k)] += p;
  }
  return out;
}

function countRoleCopies(main: number[], roleIds: number[] = []): number {
  const set = new Set(roleIds);
  let count = 0;
  for (const id of main) if (set.has(id)) count++;
  return count;
}

function countUnionCopies(main: number[], a: number[] = [], b: number[] = []): number {
  const set = new Set([...a, ...b]);
  let count = 0;
  for (const id of main) if (set.has(id)) count++;
  return count;
}

function buildHandStats(main: number[], roles: Record<string, number[]>, hand: number): HandStats {
  const N = Math.max(1, main.length);
  const kStarter = countRoleCopies(main, roles.starter);
  const kExt = countRoleCopies(main, roles.extender);
  const kHt = countRoleCopies(main, roles.handtrap);
  const kBrk = countRoleCopies(main, roles.breaker);
  const kBrick = countRoleCopies(main, roles.brick);
  const starterDist = dist6(N, kStarter, hand);
  const handtrapDist = dist6(N, kHt, hand);

  const starter = 1 - starterDist[0];
  const starter2 = Math.max(0, 1 - starterDist[0] - starterDist[1]);
  const handtrap = 1 - handtrapDist[0];
  const ideal = pBoth(N, kStarter, kHt, countUnionCopies(main, roles.starter, roles.handtrap), hand);
  const starterAndExt = pBoth(N, kStarter, kExt, countUnionCopies(main, roles.starter, roles.extender), hand);
  const resist = Math.min(1, Math.max(starter2, starterAndExt));
  const solid = Math.min(resist, ideal) * 0.92;
  const brick = starterDist[0];
  const garnet = pAtLeastOne(N, kBrick, hand);
  const breaker = pAtLeastOne(N, kBrk, hand);

  const expected: Record<string, number> = {};
  for (const [key, list] of Object.entries(roles)) {
    expected[key] = (countRoleCopies(main, list) * hand) / N;
  }

  return {
    hand,
    starter,
    starter2,
    handtrap,
    ideal,
    resist,
    solid,
    brick,
    garnet,
    breaker,
    expected,
    starterDist,
    handtrapDist,
  };
}

function evalCondProb(main: number[], roles: Record<string, number[]>, cond: Cond, hand: number): number {
  const N = Math.max(1, main.length);
  switch (cond.type) {
    case "count": {
      const K =
        cond.target.kind === "role"
          ? countRoleCopies(main, roles[cond.target.value])
          : main.filter((id) => id === cond.target.value).length;
      let p = 0;
      for (let k = 0; k <= hand; k++) {
        const match = cond.op === "atLeast" ? k >= cond.n : cond.op === "atMost" ? k <= cond.n : k === cond.n;
        if (match) p += hypergeomPmf(N, K, hand, k);
      }
      return Math.max(0, Math.min(1, p));
    }
    case "not":
      return Math.max(0, Math.min(1, 1 - evalCondProb(main, roles, cond.item, hand)));
    case "all": {
      if (cond.items.length === 0) return 1;
      let prod = 1;
      for (const item of cond.items) prod *= evalCondProb(main, roles, item, hand);
      return Math.max(0, Math.min(1, prod));
    }
    case "any": {
      if (cond.items.length === 0) return 0;
      let fail = 1;
      for (const item of cond.items) fail *= 1 - evalCondProb(main, roles, item, hand);
      return Math.max(0, Math.min(1, 1 - fail));
    }
  }
}

// --- Recipes & Match Log state ---

let recipesState: Recipe[] = [
  {
    id: 1,
    deckId: 1,
    name: "Starter + Handtrap + Estensore",
    cond: {
      type: "all",
      items: [
        { type: "count", target: { kind: "role", value: "starter" }, op: "atLeast", n: 1 },
        { type: "count", target: { kind: "role", value: "extender" }, op: "atLeast", n: 1 },
        { type: "count", target: { kind: "role", value: "handtrap" }, op: "atLeast", n: 1 },
      ],
    },
  },
];
let nextRecipeId = 2;

let matchesState: MatchEntry[] = [
  { id: 1, deckId: 1, deckName: "Snake-Eye Fire King", version: "v1", playedAt: "2026-10-12 18:15", opponent: "Tenpai Dragon", first: true, games: "WW", result: "W", tags: ["Master Duel", "Master I"], notes: "Promethean + Apollousa hanno fermato Sangen Summoning." },
  { id: 2, deckId: 1, deckName: "Snake-Eye Fire King", version: "v1", playedAt: "2026-10-12 18:45", opponent: "Yubel Fiendsmith", first: false, games: "LWL", result: "L", tags: ["Master Duel", "Master I"], notes: "Phantom of Yubel + Apollousa difficile da forzare senza Droplet." },
  { id: 3, deckId: 1, deckName: "Snake-Eye Fire King", version: "v1", playedAt: "2026-10-13 20:10", opponent: "Tearlaments", first: true, games: "W", result: "W", tags: ["Master Duel", "Master I"], notes: "Abyss Dweller / Kirin su Kitkallos." },
  { id: 4, deckId: 1, deckName: "Snake-Eye Fire King", version: "v1", playedAt: "2026-10-14 21:00", opponent: "Branded Despia", first: false, games: "W", result: "W", tags: ["Master Duel", "Master I"], notes: "Ash Blossom su Branded Fusion." },
  { id: 5, deckId: 1, deckName: "Snake-Eye Fire King", version: "v1", playedAt: "2026-10-15 19:30", opponent: "Tenpai Dragon", first: false, games: "L", result: "L", tags: ["Master Duel", "Master I"], notes: "Harpie's Feather Duster + Paidra." },
  { id: 6, deckId: 1, deckName: "Snake-Eye Fire King", version: "v2", playedAt: "2026-10-16 17:20", opponent: "Yubel Fiendsmith", first: true, games: "W", result: "W", tags: ["Master Duel", "Master I"], notes: "Aggiunta seconda Forbidden Droplet: ottimo impatto." },
  { id: 7, deckId: 1, deckName: "Snake-Eye Fire King", version: "v2", playedAt: "2026-10-17 18:05", opponent: "Centur-Ion", first: true, games: "W", result: "W", tags: ["Master Duel", "Rating"], notes: "Impermanence su Auxila, poi OTK con Flamberge + Raging Phoenix." },
  { id: 8, deckId: 1, deckName: "Snake-Eye Fire King", version: "v2", playedAt: "2026-10-18 15:40", opponent: "Tenpai Dragon", first: true, games: "W", result: "W", tags: ["Master Duel", "Rating"], notes: "Kirin in Battle Phase evita Trident Dragion." },
  { id: 9, deckId: 1, deckName: "Snake-Eye Fire King", version: "v2", playedAt: "2026-10-18 16:15", opponent: "Yubel Fiendsmith", first: false, games: "W", result: "W", tags: ["Master Duel", "Rating"], notes: "Maxx C risolta, pescati 3 extender." },
  { id: 10, deckId: 1, deckName: "Snake-Eye Fire King", version: "v2", playedAt: "2026-10-19 21:10", opponent: "Branded Despia", first: true, games: "L", result: "L", tags: ["Master Duel", "Rating"], notes: "Super Polymerization su Apollousa + Promethean." },
  { id: 11, deckId: 1, deckName: "Snake-Eye Fire King", version: "v2", playedAt: "2026-10-20 11:30", opponent: "Tearlaments", first: false, games: "W", result: "W", tags: ["Master Duel", "Rating"], notes: "Called by the Grave su Havnis." },
  { id: 12, deckId: 2, deckName: "Fiendsmith Invoked Control", version: "v1", playedAt: "2026-10-19 19:00", opponent: "Tenpai Dragon", first: true, games: "W", result: "W", tags: ["Master Duel", "Master I"], notes: "Mechaba bandisce Chundra." },
];
let nextMatchId = 13;

function makeWinRecord(wins: number, losses: number, draws = 0): WinRecord {
  const n = wins + losses;
  const p = n > 0 ? wins / n : 0;
  // Wilson 95% interval approximation
  const z = 1.96;
  const denom = 1 + (z * z) / Math.max(1, n);
  const center = n > 0 ? (p + (z * z) / (2 * n)) / denom : 0.5;
  const half = n > 0 ? (z * Math.sqrt((p * (1 - p) + (z * z) / (4 * n)) / n)) / denom : 0.5;
  const lo = n > 0 ? Math.max(0, center - half) : 0;
  const hi = n > 0 ? Math.min(1, center + half) : 1;
  return {
    wins,
    losses,
    draws,
    p,
    lo,
    hi,
    betaLo: Math.max(0, lo * 0.96),
    betaHi: Math.min(1, hi * 1.02),
  };
}

function outcomeOf(m: MatchEntry): "W" | "L" | "D" {
  if (m.result === "W" || m.result === "L" || m.result === "D") return m.result;
  let w = 0;
  let l = 0;
  for (const ch of m.games.toUpperCase()) {
    if (ch === "W") w++;
    else if (ch === "L") l++;
  }
  return w > l ? "W" : l > w ? "L" : "D";
}

function tallyRecord(list: MatchEntry[]): WinRecord {
  let w = 0;
  let l = 0;
  let d = 0;
  for (const m of list) {
    const out = outcomeOf(m);
    if (out === "W") w++;
    else if (out === "L") l++;
    else d++;
  }
  return makeWinRecord(w, l, d);
}

function buildMatchLog(deckId: number | null): MatchLog {
  const filtered = deckId == null ? [...matchesState] : matchesState.filter((m) => m.deckId === deckId);
  const total = tallyRecord(filtered);
  const first = tallyRecord(filtered.filter((m) => m.first === true));
  const second = tallyRecord(filtered.filter((m) => m.first === false));

  let g1w = 0,
    g1l = 0,
    psw = 0,
    psl = 0;
  for (const m of filtered) {
    const g = m.games.toUpperCase();
    if (g[0] === "W") g1w++;
    else if (g[0] === "L") g1l++;
    for (let i = 1; i < g.length; i++) {
      if (g[i] === "W") psw++;
      else if (g[i] === "L") psl++;
    }
  }

  const byOpp = new Map<string, MatchEntry[]>();
  for (const m of filtered) {
    const k = m.opponent || "Sconosciuto";
    byOpp.set(k, [...(byOpp.get(k) ?? []), m]);
  }
  const opponents: NamedRecord[] = [...byOpp.entries()]
    .map(([name, list]) => ({
      name,
      record: tallyRecord(list),
      share: filtered.length > 0 ? list.length / filtered.length : 0,
    }))
    .sort((a, b) => b.share - a.share);

  const matrix: [string, WinRecord, WinRecord][] = opponents.map((o) => {
    const list = byOpp.get(o.name) ?? [];
    return [
      o.name,
      tallyRecord(list.filter((m) => m.first === true)),
      tallyRecord(list.filter((m) => m.first === false)),
    ];
  });

  let cumWins = 0;
  let cumDecided = 0;
  const recentOutcomes: number[] = [];
  const trend = filtered.map((m, idx) => {
    const out = outcomeOf(m);
    if (out === "W" || out === "L") {
      const val = out === "W" ? 1 : 0;
      cumWins += val;
      cumDecided++;
      recentOutcomes.push(val);
    }
    const windowSlice = recentOutcomes.slice(-5);
    const rolling = windowSlice.length > 0 ? windowSlice.reduce((a, b) => a + b, 0) / windowSlice.length : 0.5;
    const cumulative = cumDecided > 0 ? cumWins / cumDecided : 0.5;
    return {
      index: idx + 1,
      playedAt: m.playedAt ?? "2026-10-20 12:00",
      rolling,
      cumulative,
    };
  });

  const byTag = new Map<string, MatchEntry[]>();
  for (const m of filtered) {
    for (const t of m.tags) {
      byTag.set(t, [...(byTag.get(t) ?? []), m]);
    }
  }
  const tags: NamedRecord[] = [...byTag.entries()].map(([name, list]) => ({
    name,
    record: tallyRecord(list),
    share: filtered.length > 0 ? list.length / filtered.length : 0,
  }));

  const v1List = filtered.filter((m) => m.version === "v1");
  const v2List = filtered.filter((m) => (m.version ?? "v2") === "v2");
  const versionsReport = [
    ...(v1List.length > 0
      ? [
          {
            version: "v1",
            name: "v1 · Lista iniziale",
            firstPlayed: v1List[0].playedAt ?? "2026-10-12",
            lastPlayed: v1List[v1List.length - 1].playedAt ?? "2026-10-15",
            record: tallyRecord(v1List),
          },
        ]
      : []),
    ...(v2List.length > 0
      ? [
          {
            version: "v2",
            name: "v2 · +1 Forbidden Droplet",
            firstPlayed: v2List[0].playedAt ?? "2026-10-16",
            lastPlayed: v2List[v2List.length - 1].playedAt ?? "2026-10-20",
            record: tallyRecord(v2List),
          },
        ]
      : []),
  ];

  const logVersions = [
    { version: "v1", name: "v1 · Lista iniziale", changes: null },
    {
      version: "v2",
      name: "v2 · +1 Forbidden Droplet",
      changes: [
        { id: 54693926, zone: "main" as const, delta: 1 },
        { id: 94145021, zone: "main" as const, delta: -1 },
      ],
    },
  ];

  return {
    matches: filtered,
    report: {
      matches: filtered.length,
      total,
      first,
      second,
      orderFisherP: 0.34,
      game1: makeWinRecord(g1w, g1l),
      postSide: makeWinRecord(psw, psl),
      opponents,
      matrix,
      metaExpected: total.p,
      trend,
      versions: versionsReport,
      latestComparison: versionsReport.length >= 2 ? { fisherP: 0.28, probNewerBetter: 0.79 } : null,
      tags,
    },
    versions: logVersions,
    current: logVersions[1],
    currentIsNew: false,
    opponents: ["Tenpai Dragon", "Yubel Fiendsmith", "Tearlaments", "Branded Despia", "Centur-Ion"],
    tags: ["Master Duel", "Master I", "Rating", "Torneo"],
  };
}

export const mockApi: Api = {
  dataStatus: async () => structuredClone(statusState),

  updateData: async (force = false) => {
    const items: { item: Progress["item"]; total: number }[] = [
      { item: "cards", total: 26_214_400 },
      { item: "scripts", total: 41_943_040 },
      { item: "cdb", total: 15_728_640 },
    ];
    for (const { item, total } of items) {
      progressListeners.forEach((fn) => fn({ item, stage: "checking", done: 0, total }));
      await delay(60);
      if (!force && statusState.cards) {
        progressListeners.forEach((fn) => fn({ item, stage: "downloading", done: Math.round(total * 0.55), total }));
        await delay(70);
      }
      progressListeners.forEach((fn) => fn({ item, stage: "indexing", done: total, total }));
      await delay(50);
      progressListeners.forEach((fn) => fn({ item, stage: force ? "done" : "upToDate", done: total, total }));
    }
    return structuredClone(statusState);
  },

  search: async (query: string) => searchCatalog(query),

  getCards: async (ids: number[]) => ids.map((id) => CARDS.get(id)).filter((c): c is Card => c != null),

  facets: async (): Promise<Facets> => ({
    attributes: ["FIRE", "DARK", "LIGHT", "EARTH", "WIND", "WATER"],
    races: ["Pyro", "Spellcaster", "Winged Beast", "Beast", "Dragon", "Fiend", "Warrior", "Cyberse", "Machine", "Zombie", "Insect", "Rock", "Fairy"],
    archetypes: ["Snake-Eye", "Sinful Spoils", "Fire King", "Diabellstar", "Salamangreat", "Fiendsmith", "Invoked", "Witchcrafter", "Ancient Gear"],
    formats: ["Master Duel", "TCG", "OCG"],
  }),

  listFavorites: async () => [...favoritesState],

  setFavorite: async (id: number, favorite: boolean) => {
    if (favorite) favoritesState.add(id);
    else favoritesState.delete(id);
  },

  listSavedSearches: async () => structuredClone(savedSearchesState),

  saveSearch: async (name: string, query: string) => {
    savedSearchesState = [{ name, query }, ...savedSearchesState.filter((s) => s.name !== name)];
    return structuredClone(savedSearchesState);
  },

  deleteSavedSearch: async (name: string) => {
    savedSearchesState = savedSearchesState.filter((s) => s.name !== name);
    return structuredClone(savedSearchesState);
  },

  onProgress: async (handler: (p: Progress) => void): Promise<UnlistenFn> => {
    progressListeners.add(handler);
    return () => {
      progressListeners.delete(handler);
    };
  },

  listDecks: async () => decksState.map(toSummary),

  shelfDecks: async (): Promise<ShelfDeck[]> =>
    decksState.map((d) => {
      const leg = validateDeckInternal(d);
      return {
        ...toSummary(d),
        legal: leg.legal,
        errors: leg.issues.filter((i) => i.severity === "error").length,
      };
    }),

  setDeckBox: async (id: number, deckBox: DeckBox) => {
    const deck = decksState.find((d) => d.id === id);
    if (deck) deck.box = structuredClone(deckBox);
  },

  duplicateDeck: async (id: number) => {
    const orig = decksState.find((d) => d.id === id) ?? decksState[0];
    const copy: Deck = {
      ...structuredClone(orig),
      id: nextDeckId++,
      name: `${orig.name} (copia)`,
      updatedAt: new Date().toISOString().slice(0, 10),
    };
    decksState = [copy, ...decksState];
    return structuredClone(copy);
  },

  getDeck: async (id: number) => {
    const found = decksState.find((d) => d.id === id);
    return found ? structuredClone(found) : null;
  },

  saveDeck: async (deck: Deck) => {
    const today = new Date().toISOString().slice(0, 10);
    if (deck.id != null) {
      const idx = decksState.findIndex((d) => d.id === deck.id);
      const updated: Deck = { ...structuredClone(deck), updatedAt: today };
      if (idx >= 0) decksState[idx] = updated;
      else decksState.unshift(updated);
      return structuredClone(updated);
    }
    const created: Deck = { ...structuredClone(deck), id: nextDeckId++, updatedAt: today };
    decksState.unshift(created);
    return structuredClone(created);
  },

  deleteDeck: async (id: number) => {
    decksState = decksState.filter((d) => d.id !== id);
  },

  validateDeck: async (deck: Deck) => validateDeckInternal(deck),

  importDeck: async (text: string): Promise<Imported> => {
    const main: number[] = [];
    const extra: number[] = [];
    const side: number[] = [];
    const unknown: number[] = [];
    let section: "main" | "extra" | "side" = "main";
    for (const raw of text.split(/\r?\n/)) {
      const line = raw.trim();
      if (!line) continue;
      if (line.startsWith("#main")) section = "main";
      else if (line.startsWith("#extra")) section = "extra";
      else if (line.startsWith("!side")) section = "side";
      else if (/^\d+$/.test(line)) {
        const id = Number(line);
        if (!CARDS.has(id)) unknown.push(id);
        else if (section === "extra") extra.push(id);
        else if (section === "side") side.push(id);
        else main.push(id);
      }
    }
    if (main.length === 0 && extra.length === 0) {
      return {
        main: [...decksState[0].main],
        extra: [...decksState[0].extra],
        side: [...decksState[0].side],
        unknown: [],
      };
    }
    return { main, extra, side, unknown };
  },

  exportDeck: async (deck: Deck) => {
    const ydk = [
      "#created by DuelForge (mock)",
      "#main",
      ...deck.main.map(String),
      "#extra",
      ...deck.extra.map(String),
      "!side",
      ...deck.side.map(String),
      "",
    ].join("\n");
    return {
      ydk,
      ydke: `ydke://mock-${deck.id ?? 0}!`,
    };
  },

  labOverview: async (main: number[], roles: Record<string, number[]>): Promise<LabOverview> => {
    const N = Math.max(1, main.length);
    const roleCounts: Record<string, number> = {};
    for (const [k, list] of Object.entries(roles)) {
      roleCounts[k] = countRoleCopies(main, list);
    }
    const copyMap = new Map<number, number>();
    for (const id of main) copyMap.set(id, (copyMap.get(id) ?? 0) + 1);
    const cards = [...copyMap.entries()].map(([id, copies]) => ({
      id,
      copies,
      first: pAtLeastOne(N, copies, 5),
      second: pAtLeastOne(N, copies, 6),
    }));
    const seen = [5, 6, 7, 8, 9, 10];
    const byCopies = [1, 2, 3].map((copies) => seen.map((s) => pAtLeastOne(N, copies, s)));
    const opponent = Array.from({ length: 21 }, (_, k) => pAtLeastOne(40, k, 5));

    return {
      size: main.length,
      roles: roleCounts,
      first: buildHandStats(main, roles, 5),
      second: buildHandStats(main, roles, 6),
      cards,
      seen,
      byCopies,
      opponent,
    };
  },

  labProbability: async (main: number[], roles: Record<string, number[]>, cond: Cond, hand: number) => ({
    p: evalCondProb(main, roles, cond, hand),
    error: null,
  }),

  setDeckRoles: async (id: number, roles: Record<string, number[]>) => {
    const d = decksState.find((x) => x.id === id);
    if (d) d.roles = structuredClone(roles);
  },

  guessDeckRoles: async (id: number): Promise<RoleGuess[]> => {
    const d = decksState.find((x) => x.id === id) ?? decksState[0];
    const unique = [...new Set(d.main)];
    for (let i = 1; i <= unique.length; i++) {
      rolesProgressListeners.forEach((fn) => fn({ done: i, total: unique.length }));
      await delay(20);
    }
    return unique.map((code) => {
      const assigned: string[] = [];
      const reasons: string[] = [];
      if (d.roles.starter?.includes(code)) {
        assigned.push("starter");
        reasons.push("Starter: da sola avvia la linea principale e porta alle interruzioni a fine turno.");
      }
      if (d.roles.extender?.includes(code)) {
        assigned.push("extender");
        reasons.push("Estensore: permette di continuare la giocata dopo che lo starter viene negato.");
      }
      if (d.roles.handtrap?.includes(code) || HANDTRAP_IDS.has(code)) {
        if (!assigned.includes("handtrap")) assigned.push("handtrap");
        reasons.push("Handtrap: si attiva dalla mano durante il primo turno avversario.");
      }
      if (d.roles.breaker?.includes(code)) {
        assigned.push("breaker");
        reasons.push("Board breaker: neutralizza o rimuove più minacce su una board già stabilita.");
      }
      if (d.roles.brick?.includes(code)) {
        assigned.push("brick");
        reasons.push("Brick: carta del motore che preferisci evocare o mandare dal Deck invece di pescare.");
      }
      return { code, roles: assigned, reasons };
    });
  },

  onRolesProgress: async (handler: (p: { done: number; total: number }) => void): Promise<UnlistenFn> => {
    rolesProgressListeners.add(handler);
    return () => {
      rolesProgressListeners.delete(handler);
    };
  },

  labSimulate: async (main: number[], roles: Record<string, number[]>, cond: Cond, hand: number, rules: SimRule[], hands: number) => {
    simCancelled = false;
    const baseP = evalCondProb(main, roles, cond, hand);
    const boost = Math.min(0.11, rules.filter((r) => r.effect != null).length * 0.032);
    const targetP = Math.min(0.985, baseP + boost);
    const steps = 5;
    const checkpoints: Checkpoint[] = [];

    for (let i = 1; i <= steps; i++) {
      if (simCancelled) break;
      await delay(55);
      const n = Math.round((hands * i) / steps);
      const jitter = (steps - i) * 0.0018;
      const p = Math.max(0, Math.min(1, targetP + (i % 2 === 0 ? jitter : -jitter)));
      const half = 1.96 * Math.sqrt((p * (1 - p)) / Math.max(1, n));
      const cp: Checkpoint = { hands: n, p, lo: Math.max(0, p - half), hi: Math.min(1, p + half) };
      checkpoints.push(cp);
      simProgressListeners.forEach((fn) => fn(cp));
    }

    const last = checkpoints[checkpoints.length - 1] ?? { hands, p: targetP, lo: targetP - 0.003, hi: targetP + 0.003 };
    const effects: [number, number][] = rules
      .filter((r) => r.effect != null)
      .map((r, idx) => [r.id, Math.max(0.08, 0.28 - idx * 0.05)]);

    return {
      ...last,
      hits: Math.round(last.hands * last.p),
      checkpoints,
      effects,
      cancelled: simCancelled,
    };
  },

  labCancelSimulation: async () => {
    simCancelled = true;
  },

  onSimProgress: async (handler: (c: Checkpoint) => void): Promise<UnlistenFn> => {
    simProgressListeners.add(handler);
    return () => {
      simProgressListeners.delete(handler);
    };
  },

  labAnalyze: async (
    main: number[],
    roles: Record<string, number[]>,
    cond: Cond,
    hand: number,
    roleKeys: string[],
    limits: Record<number, number>,
    roleA: string | null,
    roleB: string | null,
  ): Promise<LabAnalysis> => {
    const N = Math.max(40, main.length);
    const base = evalCondProb(main, roles, cond, hand);

    const roleDeltas = roleKeys.map((key) => {
      const k = countRoleCopies(main, roles[key]);
      const weight = key === "starter" ? 0.034 : key === "handtrap" ? 0.026 : key === "extender" ? 0.019 : -0.008;
      return {
        key,
        plus: k < N ? weight : null,
        minus: k > 0 ? -weight * 1.08 : null,
      };
    });

    const uniqueIds = [...new Set(main)];
    const cardDeltas = uniqueIds.map((id) => {
      const copies = main.filter((c) => c === id).length;
      const limit = limits[id] ?? 3;
      const isStarter = roles.starter?.includes(id);
      const isHt = roles.handtrap?.includes(id);
      const isExt = roles.extender?.includes(id);
      const delta = isStarter ? 0.032 : isHt ? 0.024 : isExt ? 0.016 : -0.011;
      return {
        key: String(id),
        plus: copies < limit ? delta : null,
        minus: copies > 0 ? -delta * 1.05 : null,
      };
    });

    const size = Array.from({ length: Math.max(1, 61 - N) }, (_, i) => {
      const sz = N + i;
      return Math.max(0.05, base * (N / sz));
    });

    let heatmap: LabAnalysis["heatmap"] = null;
    if (roleA && roleB && roleA !== roleB) {
      const curA = countRoleCopies(main, roles[roleA]);
      const curB = countRoleCopies(main, roles[roleB]);
      const aVals = Array.from({ length: 7 }, (_, i) => Math.max(0, curA - 3 + i));
      const bVals = Array.from({ length: 7 }, (_, i) => Math.max(0, curB - 3 + i));
      const pGrid = aVals.map((ka) =>
        bVals.map((kb) => {
          if (ka + kb > N) return null;
          return pBoth(N, ka, kb, Math.min(N, ka + kb), hand);
        }),
      );
      heatmap = { a: aVals, b: bVals, p: pGrid, current: [curA, curB] };
    }

    return {
      sensitivity: {
        base,
        roles: roleDeltas,
        cards: cardDeltas,
        grew: false,
      },
      size,
      heatmap,
      error: null,
    };
  },

  labOptimize: async (args): Promise<OptimizeResult> => {
    await delay(120);
    const beforeFirst = evalCondProb(args.main, args.roles, args.cond, 5);
    const beforeSecond = evalCondProb(args.main, args.roles, args.cond, 6);
    const w = args.weightFirst;
    const beforeObj = w * beforeFirst + (1 - w) * beforeSecond;

    const addId = args.maybe[0] ?? 94145021;
    const removeId = args.main.find((id) => args.roles.brick?.includes(id)) ?? args.main[args.main.length - 1] ?? 11264180;
    const fromAdd = args.main.filter((id) => id === addId).length;
    const fromRem = args.main.filter((id) => id === removeId).length;

    const afterFirst = Math.min(0.98, beforeFirst + 0.031);
    const afterSecond = Math.min(0.99, beforeSecond + 0.027);
    const afterObj = w * afterFirst + (1 - w) * afterSecond;

    return {
      before: { first: beforeFirst, second: beforeSecond, objective: beforeObj },
      after: { first: afterFirst, second: afterSecond, objective: afterObj },
      changes:
        addId !== removeId
          ? [
              { id: addId, from: fromAdd, to: fromAdd + 1 },
              { id: removeId, from: fromRem, to: Math.max(0, fromRem - 1) },
            ]
          : [],
      size: args.main.length,
    };
  },

  listRecipes: async (deckId: number): Promise<Recipe[]> =>
    structuredClone(recipesState.filter((r) => r.deckId == null || r.deckId === deckId)),

  saveRecipe: async (deckId, name, cond): Promise<Recipe> => {
    const rec: Recipe = { id: nextRecipeId++, deckId, name, cond: structuredClone(cond) };
    recipesState = [...recipesState, rec];
    return structuredClone(rec);
  },

  deleteRecipe: async (id: number) => {
    recipesState = recipesState.filter((r) => r.id !== id);
  },

  matchLog: async (deckId: number | null): Promise<MatchLog> => buildMatchLog(deckId),

  logMatch: async (entry: MatchEntry): Promise<MatchEntry> => {
    const deck = decksState.find((d) => d.id === entry.deckId);
    const saved: MatchEntry = {
      ...structuredClone(entry),
      id: nextMatchId++,
      deckName: entry.deckName ?? deck?.name ?? "Snake-Eye Fire King",
      version: entry.version || "v2",
      playedAt: entry.playedAt || new Date().toISOString().slice(0, 16).replace("T", " "),
      result: outcomeOf(entry),
    };
    matchesState = [...matchesState, saved];
    return structuredClone(saved);
  },

  updateMatch: async (entry: MatchEntry): Promise<MatchEntry> => {
    const updated: MatchEntry = { ...structuredClone(entry), result: outcomeOf(entry) };
    matchesState = matchesState.map((m) => (m.id === updated.id ? updated : m));
    return structuredClone(updated);
  },

  deleteMatch: async (id: number) => {
    matchesState = matchesState.filter((m) => m.id !== id);
  },

  exportMatches: async (_deckId: number | null, name: string) => `C:\\Users\\valer\\Downloads\\DuelForge\\${name.replace(/\s+/g, "-").toLowerCase()}-matches.csv`,

  importMatches: async (deckId: number, text: string) => {
    const lines = text.split(/\r?\n/).filter((l) => l.trim().length > 0);
    const added = Math.max(1, lines.length - 1);
    const deck = decksState.find((d) => d.id === deckId);
    matchesState.push({
      id: nextMatchId++,
      deckId,
      deckName: deck?.name ?? "Deck",
      version: "v2",
      playedAt: "2026-10-20 16:00",
      opponent: "Tenpai Dragon",
      first: true,
      games: "WW",
      result: "W",
      tags: ["Importato"],
      notes: "Partita importata da CSV",
    });
    return added;
  },

  sampleSize: async (p0: number, p1: number): Promise<[number, number]> => {
    const diff = Math.max(0.02, Math.abs(p1 - p0));
    const n80 = Math.round((0.5 / (diff * diff)) * 3.1);
    const n90 = Math.round((0.5 / (diff * diff)) * 4.2);
    return [n80, n90];
  },

  // Step 3 stubs (populated in full in Step 3)
  comboSearch: async (): Promise<ComboOutcome> => ({
    stop: "Exhausted",
    elapsedMs: 1200,
    nodes: 420,
    hands: [],
    boards: [],
    complete: true,
  }),
  comboContinue: async (): Promise<ComboOutcome> => ({
    stop: "Exhausted",
    elapsedMs: 1800,
    nodes: 640,
    hands: [],
    boards: [],
    complete: true,
  }),
  comboScore: async () => 0,
  comboScores: async (): Promise<ComboScore[]> => [],
  onComboScore: async (_handler: (s: ComboScore) => void): Promise<UnlistenFn> => () => {},
  comboCancel: async () => {},
  comboExplain: async () => [],
  onComboProgress: async (_handler: (p: ComboProgress) => void): Promise<UnlistenFn> => () => {},
};
