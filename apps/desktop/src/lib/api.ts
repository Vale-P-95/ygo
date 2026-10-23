import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const isMock = import.meta.env.VITE_MOCK === "1" || import.meta.env.VITE_MOCK === "true";

/** False when the UI runs in a plain browser (vite dev without Tauri). */
export const isDesktop = isTauri() || isMock;

export type CardKind = "monster" | "spell" | "trap" | "skill" | "token";

/** Mirrors `ygo_data::Card`. */
export interface Card {
  id: number;
  name: string;
  kind: CardKind;
  frame: string;
  humanType: string;
  typeLine: string[];
  desc: string;
  pendDesc: string | null;
  race: string;
  attribute: string | null;
  archetype: string | null;
  level: number | null;
  atk: number | null;
  def: number | null;
  scale: number | null;
  linkRating: number | null;
  linkMarkers: string[];
  mdRarity: string | null;
  formats: string[];
  tcgDate: string | null;
  ocgDate: string | null;
  banTcg: string | null;
  banOcg: string | null;
  /** Master Duel status, from DuelForge's own list. */
  banMd: string | null;
  imageIds: number[];
  views: number;
}

export interface Chip {
  token: string;
  label: string;
}

export interface SearchResult {
  ids: number[];
  chips: Chip[];
  errors: string[];
}

export interface Facets {
  attributes: string[];
  races: string[];
  archetypes: string[];
  formats: string[];
}

/** A card whose Forbidden & Limited status changed with a list update. */
export interface BanChange {
  format: "tcg" | "ocg" | "md";
  id: number;
  /** `YYYY-MM-DD`: when the list took effect (Master Duel) or when the card data showed the change (TCG, OCG). */
  date: string;
  /** "Forbidden", "Limited" or "Semi-Limited"; `null` is unlimited. */
  before: string | null;
  after: string | null;
}

export interface DataStatus {
  dataDir: string;
  cards: { version: string; count: number } | null;
  engine: {
    apiVersion: string;
    scripts: number | null;
    scriptsVersion: string | null;
    databases: number;
    cdbVersion: string | null;
  };
  imagesCached: number;
  /** Engine data download failure after an update (cards may still be fine). */
  engineError: string | null;
  /** Effective date (YYYY-MM-DD) of the Master Duel Forbidden & Limited list in use. */
  banlistMd: string | null;
}

export type ProgressItem = "cards" | "scripts" | "cdb";
export type ProgressStage = "checking" | "downloading" | "extracting" | "indexing" | "done" | "upToDate";

export interface Progress {
  item: ProgressItem;
  stage: ProgressStage;
  done: number;
  total: number | null;
}

export type DeckFormat = "md" | "tcg" | "ocg";

/** How a deck looks on the shelf (`ygo_data::DeckBox`); empty means default. */
export interface DeckBox {
  color: string;
  material: string;
  shape: string;
  showcase: number[];
}

/** Mirrors `ygo_data::Deck`. */
export interface Deck {
  id: number | null;
  name: string;
  format: DeckFormat;
  main: number[];
  extra: number[];
  side: number[];
  /** Cards being considered: not part of the deck. */
  maybe: number[];
  /** Cards by role ("starter", "handtrap", ...). */
  roles: Record<string, number[]>;
  box: DeckBox;
  updatedAt: string | null;
}

export interface DeckSummary {
  id: number;
  name: string;
  format: DeckFormat;
  mainCount: number;
  extraCount: number;
  sideCount: number;
  updatedAt: string;
  cover: number | null;
  /** Showcase cards, else the first three Main Deck cards. */
  preview: number[];
  box: DeckBox;
}

/** A deck on the shelf, with its legality under the current lists. */
export interface ShelfDeck extends DeckSummary {
  legal: boolean | null;
  errors: number;
}

export interface Issue {
  severity: "error" | "warning";
  message: string;
  cards: number[];
}

export interface Legality {
  legal: boolean;
  issues: Issue[];
  /** Effective date of the list used (Master Duel). */
  banlist: string | null;
}

/** "2026-10-06" → "6 ottobre 2026". */
export function formatDate(iso: string): string {
  const d = new Date(`${iso}T12:00:00`);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString("it-IT", { day: "numeric", month: "long", year: "numeric" });
}

/** Lab condition (`ygo_stats::Cond`). */
export type LabTarget = { kind: "role"; value: string } | { kind: "card"; value: number };
export type LabOp = "atLeast" | "atMost" | "exactly";
export type Cond =
  | { type: "all"; items: Cond[] }
  | { type: "any"; items: Cond[] }
  | { type: "not"; item: Cond }
  | { type: "count"; target: LabTarget; op: LabOp; n: number };

export interface HandStats {
  hand: number;
  starter: number;
  starter2: number;
  handtrap: number;
  ideal: number;
  resist: number;
  solid: number;
  brick: number;
  garnet: number;
  breaker: number;
  expected: Record<string, number>;
  starterDist: number[];
  handtrapDist: number[];
}

/** Mirrors `ygo_stats::Overview`. */
export interface LabOverview {
  size: number;
  roles: Record<string, number>;
  first: HandStats;
  second: HandStats;
  cards: { id: number; copies: number; first: number; second: number }[];
  seen: number[];
  byCopies: number[][];
  opponent: number[];
}

/** Simulator rules for one card (`ygo_stats::CardRule`). */
export type SimEffect = { type: "draw"; n: number; banishTop: number } | { type: "excavate"; n: number } | { type: "search"; targets: number[] };
export interface SimRule {
  id: number;
  effect: SimEffect | null;
  oncePerTurn: boolean;
  normalSummon: boolean;
}
export interface Checkpoint {
  hands: number;
  p: number;
  lo: number;
  hi: number;
}
export interface SimResult extends Checkpoint {
  hits: number;
  checkpoints: Checkpoint[];
  /** [passcode, share of hands where its effect resolved] */
  effects: [number, number][];
  cancelled: boolean;
}
export interface Delta {
  key: string;
  plus: number | null;
  minus: number | null;
}
export interface LabAnalysis {
  sensitivity: { base: number; roles: Delta[]; cards: Delta[]; grew: boolean } | null;
  size: (number | null)[];
  heatmap: { a: number[]; b: number[]; p: (number | null)[][]; current: [number, number] } | null;
  error: string | null;
}
export interface OptimizeScore {
  first: number;
  second: number;
  objective: number;
}
export interface OptimizeResult {
  before: OptimizeScore;
  after: OptimizeScore;
  changes: { id: number; from: number; to: number }[];
  size: number;
}

export interface Recipe {
  id: number;
  deckId: number | null;
  name: string;
  cond: Cond;
}

/** One logged match. Outcomes are "W", "L" or "D". */
export interface MatchEntry {
  id?: number | null;
  deckId: number | null;
  deckName?: string;
  /** Fingerprint of the list played. */
  version?: string;
  /** "YYYY-MM-DD HH:MM"; now when empty. */
  playedAt?: string;
  opponent: string;
  /** Went first in game 1; null when not recorded. */
  first: boolean | null;
  /** Each game in order: "WLW". */
  games: string;
  /** Worked out from the games when empty. */
  result?: string;
  tags: string[];
  notes: string;
}
export interface WinRecord {
  wins: number;
  losses: number;
  draws: number;
  /** Over decided matches. */
  p: number;
  /** 95% Wilson interval. */
  lo: number;
  hi: number;
  /** 95% credible interval, Beta(1 + wins, 1 + losses). */
  betaLo: number;
  betaHi: number;
}
export interface NamedRecord {
  name: string;
  record: WinRecord;
  share: number;
}
export interface MatchReport {
  matches: number;
  total: WinRecord;
  first: WinRecord;
  second: WinRecord;
  orderFisherP: number | null;
  game1: WinRecord;
  postSide: WinRecord;
  opponents: NamedRecord[];
  /** [opponent, going first, going second] */
  matrix: [string, WinRecord, WinRecord][];
  metaExpected: number;
  trend: { index: number; playedAt: string; rolling: number; cumulative: number }[];
  versions: { version: string; name: string; firstPlayed: string; lastPlayed: string; record: WinRecord }[];
  latestComparison: { fisherP: number; probNewerBetter: number } | null;
  tags: NamedRecord[];
}
export interface LogVersion {
  version: string;
  name: string;
  /** From the version before; null when a list is unknown. */
  changes: { id: number; zone: "main" | "extra" | "side"; delta: number }[] | null;
}
export interface MatchLog {
  matches: MatchEntry[];
  report: MatchReport;
  versions: LogVersion[];
  /** The list as saved now (log of one deck). */
  current: LogVersion | null;
  /** No match has been played with the current list yet. */
  currentIsNew: boolean;
  opponents: string[];
  tags: string[];
}

export interface SavedSearch {
  name: string;
  query: string;
}

export interface Imported {
  main: number[];
  extra: number[];
  side: number[];
  unknown: number[];
}

/** A card on the field at the end of a combo (mirrors `ZoneCard`). */
export interface ZoneCard {
  code: number;
  faceUp: boolean;
  defense: boolean;
  link: number;
  arrows: number;
  level: number;
  rank: number;
  attack: number;
  def: number;
  overlay: number[];
  counters: number;
  negated: boolean;
}

/** A board reached by the combo search, with what it took. */
export interface ComboBoard {
  /** With `hand`, names the board for `comboExplain`. */
  key: string;
  hand: number;
  quickScore: number;
  cardsUsed: number;
  choices: number;
  foundAfterMs: number;
  /** Played to the opponent's turn; otherwise read when the turn was ended, before the End Phase. */
  exact: boolean;
  /** Cards of the goal on the field. */
  goalHits: number;
  /** Main Monster Zones 0-4, then the Extra Monster Zones. */
  monsters: (ZoneCard | null)[];
  /** Spell & Trap Zones 0-4, then the Field Zone. */
  spells: (ZoneCard | null)[];
  handCards: number[];
  grave: number[];
  banished: number[];
  deckCount: number;
  extraCount: number;
  /** Face-up Pendulum monsters in the Extra Deck. */
  extraFaceUp: number[];
}

export interface ComboHand {
  cards: number[];
  /** Share of the hands with the card that this one stands for. */
  weight: number;
  /** The goal is reached (true), impossible (false) or unknown (null). */
  goal: boolean | null;
  /** Every line from this hand was tried. */
  complete: boolean;
}

export interface ComboProgress {
  elapsedMs: number;
  nodes: number;
  replays: number;
  duplicates: number;
  queued: number;
  boards: number;
  handIndex: number;
  hands: ComboHand[];
  top: ComboBoard[];
}

export interface ComboOutcome {
  stop: string;
  elapsedMs: number;
  nodes: number;
  hands: ComboHand[];
  boards: ComboBoard[];
  /** Every line of every hand was tried: going on finds nothing new. */
  complete: boolean;
}

/** Where a monster of the goal must be. */
export type GoalZone = "any" | "extra" | "main";

export interface GoalCard {
  code: number;
  zone: GoalZone;
}

export interface ComboGoal {
  cards: GoalCard[];
  /** All of them (else one is enough). */
  all: boolean;
  /** Stop at the first board that reaches it. */
  first: boolean;
}

export interface ComboRequest {
  deckId: number;
  /** A precise hand, or empty with `card` for "card + rest". */
  hand: number[];
  card: number | null;
  seconds: number;
  hands: number | null;
  keep: number[];
  forbid: number[];
  useHandtraps: boolean;
  goal: ComboGoal | null;
}

/** One of your effects that stops the opponent in the probe turn. */
export interface ComboInterruption {
  code: number;
  /** "mano", "campo", "Cimitero", "bandita". */
  place: string;
  effect: string;
  targeted: boolean;
  uses: number;
  coverage: number;
  /** With your other effects; `alone` with none of them used (less when they share a cost). */
  value: number;
  alone: number;
  covers: string[];
}

export interface ComboBreaker {
  name: string;
  notApplicable: boolean;
  stoppedBy: number | null;
  remaining: number;
}

/** What one kind of handtrap takes away from a line (`ComboScore.exposureDetail.losses`). */
export interface ExposureLoss {
  /** "ash" (Ash Blossom), "fieldEffect" (Effect Veiler, Infinite Impermanence, Ghost Ogre), "nibiru", "droll". */
  kind: "ash" | "fieldEffect" | "nibiru" | "droll";
  loss: number;
  chance: number;
}

/** What the probe turn measured on a board (mirrors `ScoreView`). */
export interface ComboScore {
  hand: number;
  key: string;
  interruptions: number;
  handTraps: number;
  floodgates: number;
  resilience: number;
  resources: number;
  followup: number;
  cost: number;
  details: ComboInterruption[];
  floodgateList: string[];
  breakers: ComboBreaker[];
  resourcesNote: string;
  followupNote: string;
  /**
   * What the handtraps are expected to take away from the line, in interruptions: for each kind, the value lost at the
   * worst moment times the chance the opponent holds one (`exposureDetail.losses`), plus Maxx "C". The choke points are
   * counted in `exposureDetail.chokePoints`.
   */
  exposure: number;
  exposureNote: string;
  exposureDetail: {
    ash: number;
    onField: number;
    summons: number;
    specialSummons: number;
    searches: number;
    /** Moments of the line where a handtrap stops it (what `exposure` was before). */
    chokePoints?: number;
    /** Per kind of handtrap the line meets: interruptions lost at the worst moment, and the chance (0-1) it is in the opponent's hand. */
    losses?: ExposureLoss[];
    /** Interruptions given to the opponent by Maxx "C" or a Mulcharmy, and the chance it holds one. */
    maxx?: number;
    maxxChance?: number;
  } | null;
  /** Against the deck chosen with `comboScore(vsDeckId)`. */
  matchup?: ComboMatchup | null;
  /**
   * Share (0-1) of what your effects stop together that they still stop when the opponent answers the first of them it can,
   * with Infinite Impermanence. `null` when it can answer none, or they stop nothing. Shown, not weighed.
   */
  countered: number | null;
  error: string | null;
  /**
   * Only the cheap measure: the board was not among the best by it, so it has no breakers (`resilience` is all its field),
   * follow-up, exposure nor matchup. Rank it after the boards with the full measure (`false` or missing). The backend does
   * not send these yet (`SEND_CHEAP_SCORES` in `lib.rs`): it will once the page ranks them apart.
   */
  cheap?: boolean;
  /**
   * Level 2 against the 3 decks of the meta played the most (L2-3), for the best boards only: set by the `combo_meta`
   * command (`invoke("combo_meta", { boards })`, `boards` an optional list of `{ hand, key }`; without it the best 4 by
   * the "Bilanciato" profile), which sends the scores again as `combo-score` events. The boards that have it are ranked
   * by `meta.value` ahead of the others. Missing or `null` when not measured.
   */
  meta?: ComboMeta | null;
}

/** A board against the most played decks of the meta (mirrors `MetaView`). */
export interface ComboMeta {
  /** Σ weight · (1 − left) over the decks counted, weighed among them: 0-1, higher is better. `null` when none could be counted. */
  value: number | null;
  /** The same weighed share of the decks' moves still made with all your answers. */
  left: number | null;
  perDeck: ComboMetaDeck[];
}

/** One deck of the meta against a board (mirrors `MetaDeckView`). */
export interface ComboMetaDeck {
  name: string;
  /** Its `popRank` on Master Duel Meta: share of the ranked top decks, in %. */
  weight: number;
  lines: number;
  /** Share of its moves made on an empty board: under 0.8 its lines did not play as found, and the deck is not counted. */
  made: number;
  /** Share of its moves still made with all your answers; `null` when not counted. */
  left: number | null;
}

/** A card of the board that interrupts, against removals aimed at it with only its own effects to answer (mirrors `PieceView`). */
export interface ComboPiece {
  code: number;
  /** "mano", "campo", "Cimitero", "bandita". */
  place: string;
  /** Removals it is still usable after: "distruggere", "bandire", "girare coperto", "negare". */
  holds: string[];
  falls: string[];
}

/** One of the opponent deck's board breakers against the board. */
export interface ComboRivalBreaker {
  code: number;
  /** Chance it is among the 6 cards the opponent has in its turn. */
  chance: number;
  /** Interruption value of the field left after it. */
  remaining: number;
  stoppedBy: number | null;
  notApplicable: boolean;
}

/** One of the opponent deck's handtraps against the line. */
export interface ComboRivalHandTrap {
  code: number;
  /** Chance it is among the 5 cards the opponent holds during your turn. */
  chance: number;
  /** Moments of the line it can be used. */
  windows: number;
  /** Those where using it changes the field or hand you end with. */
  stops: number;
  /** Your cards being played at those moments. */
  at: number[];
  /** The most cards the opponent draws using it (Maxx "C", the Mulcharmy: they stop nothing but draw). Always sent by the backend. */
  drawn?: number;
  /** Each moment tried, in the order of the line (at most 24). Always sent by the backend. */
  moments?: ComboHandTrapMoment[];
}

/** A point where a board's line can stop while the opponent draws from Maxx "C" or a Mulcharmy (mirrors `StopPointView`). */
export interface ComboStopPoint {
  /** Steps of the line played before ending the turn (as `comboExplain` lists them; all of them for the whole line). */
  steps: number;
  /** Your last card played then. */
  playing: number | null;
  /** Interruption and floodgate value of the board you end on. */
  fieldValue: number;
  floodgateValue: number;
  /** Cards the opponent has drawn by the end of your turn. */
  drawn: number;
  /** Value expected in its turn, holding 6 cards plus those drawn (more chances of a breaker). */
  expected: number;
}

/** Where to stop the line against one handtrap that draws (mirrors `DrawTradeOffView`). */
export interface ComboDrawTradeOff {
  /** The handtrap (Maxx "C", a Mulcharmy), used at the first moment it can be. */
  code: number;
  /** In the order of the line, the whole line last. */
  stops: ComboStopPoint[];
  /** Index in `stops` of the one that keeps the most for its turn. */
  best: number;
  /** The deck has no breaker the probe turn can play: the standard ones are used, assuming about 5 in its 40 cards. */
  standardBreakers: boolean;
}

/** A move of the rival: a card, how it is played and from where (mirrors `RivalPlayView`). */
export interface ComboRivalPlay {
  code: number;
  kind: "activate" | "normalSummon" | "specialSummon";
  place: "mano" | "campo" | "Cimitero" | "bandita" | "Extra Deck" | "Deck";
}

/** One of the rival deck's best lines, from one opening hand (mirrors `RivalLineView`). */
export interface ComboRivalLine {
  hand: number[];
  /** Its moves in order; a line that only Sets cards has none. */
  moves: ComboRivalPlay[];
}

/** A move of a line measured against the board (mirrors `MeasuredMoveView`). */
export interface ComboMeasuredMove extends ComboRivalPlay {
  /** The line it belongs to (index in `ComboMoves.lines`). */
  line: number;
  /** Its share of all the moves: every line weighs the same, and every move of a line the same. */
  weight: number;
  /**
   * "made": made against the board with no answer; "stopped": the board stops it with no answer (a floodgate);
   * "lost": lost with an earlier move the board stopped; "notMade": not made even on an empty board.
   */
  state: "made" | "stopped" | "lost" | "notMade";
}

/** One of your effects against the rival's lines (mirrors `MoveStopView`). */
export interface ComboMoveStop {
  code: number;
  /** "mano", "campo", "Cimitero", "bandita". */
  place: string;
  effect: string;
  /** The moves it stops itself (indices in `ComboMoves.moves`). */
  moves: number[];
  /** Share of the moves it takes away at its best moment, averaged over the lines. */
  coverage: number;
  value: number;
}

/** The board against what a deck does in its turn: its best lines played again after yours (mirrors `MovesView`). */
export interface ComboMoves {
  /** The deck's best lines from 12 random hands. */
  lines: ComboRivalLine[];
  /** Lines measured: those with moves (a line whose run failed is left out). */
  measured: number;
  moves: ComboMeasuredMove[];
  stops: ComboMoveStop[];
  /** Share of the moves the lines make on an empty board (the rest the probe turn could not play again). */
  madeShare: number;
  /** Share of those moves your board stops with no answer. */
  floodgateShare: number;
  /** Interruption value against these lines. */
  value: number;
  /** Share of the moves the board lets the lines make that they still make with all your answers used together. */
  left: number;
}

/** A moment of the line the opponent uses a handtrap at (mirrors `HandTrapMomentView`). */
export interface ComboHandTrapMoment {
  /** Your card being played then (the last you used). */
  playing: number | null;
  /** Using it then changes the field or hand you end with. */
  stops: boolean;
  /** Cards the opponent draws from then to the end of your turn, less those it shuffles back into the Deck. */
  drawn: number;
}

/** The board and its line against a deck of the Bottega (mirrors `MatchupView`). */
export interface ComboMatchup {
  deckId: number;
  deckName: string;
  /** Interruption value of the field expected after the most harmful breaker the opponent has (all of it when it has none). */
  resilience: number;
  /** The deck has no breaker the probe turn can play: the resilience is the standard one. */
  standardBreakers: boolean;
  breakerChance: number;
  breakers: ComboRivalBreaker[];
  /** Chance the opponent has a handtrap that stops something in the line. */
  handTrapChance: number;
  handTraps: ComboRivalHandTrap[];
  error: string | null;
}

/** What a choice of the line does (its icon). */
export type ComboStepKind = "normalSummon" | "specialSummon" | "activate" | "set" | "choose" | "place" | "pass" | "endTurn" | "other";

export type ComboHappeningKind =
  | "phase"
  | "draw"
  | "summon"
  | "activate"
  | "negate"
  | "set"
  | "toHand"
  | "toGrave"
  | "banish"
  | "toDeck"
  | "toExtra"
  | "material"
  | "toField"
  | "other";

/** Something that happened to a card after a choice. */
export interface ComboHappening {
  kind: ComboHappeningKind;
  /** 0 when there is no card (a new phase). */
  code: number;
  chainLink: number;
  mine: boolean;
  text: string;
}

/** The roles the engine found for a card, and why (Italian, one line per role). */
export interface RoleGuess {
  code: number;
  roles: string[];
  reasons: string[];
}

/** Your side of the duel, in the board's shape. */
export type ComboField = Pick<ComboBoard, "monsters" | "spells" | "handCards" | "grave" | "banished" | "deckCount" | "extraCount" | "extraFaceUp">;

export interface ComboStep {
  choice: string | null;
  events: string[];
  kind: ComboStepKind | null;
  /** The card the choice is about. */
  card: number | null;
  happened: ComboHappening[];
  /** Your side once the step is done: when the next choice is asked, or at the end of the line. */
  field?: ComboField | null;
}

/** How one card of the deck leads to another (`docs/plans/mappa-collegamenti.md`). */
export type CardLinkKind = "search" | "summon" | "toGrave" | "recover" | "material" | "set";

/** `from` leads to `to` (mirrors `CardLinkView`). */
export interface CardLink {
  from: number;
  to: number;
  /**
   * search: adds it from the Deck to the hand; summon: Special Summons it; toGrave: sends it from the Deck to the GY
   * (a Fusion Material from the Deck too); recover: gets it back from the GY or banishment to the hand; material: `from`
   * is a material of `to`; set: puts it from the Deck on the field (Set, or placed in a Spell & Trap Zone or the Field Zone).
   */
  kind: CardLinkKind;
  /** Where `to` comes from (a link from the GY or the hand brings no new card into play). */
  zone: "deck" | "extra" | "grave" | "banished" | "hand";
  /** The engine offered it: the effect can pick that card. */
  possible: boolean;
  /** Share of the Combo Lab's best lines that use it, 0 to 1 (0 when none does). */
  used: number;
  /**
   * The card's text names it. With `possible` false, the link comes from the text alone: an effect the lines found never
   * use (show it faint, «dal testo»). Always sent by the backend.
   */
  text?: boolean;
}

/** A card of the map (mirrors `LinkCardView`). */
export interface LinkCard {
  code: number;
  copies: number;
  /** In the Extra Deck. */
  extra: boolean;
  /**
   * Cards it leads to in one step (any link), and with the cards brought into play from there in any number of steps
   * (search, summon, material and set links from the Deck or the Extra Deck: sending to the GY or getting back brings no new card).
   */
  reach: number;
  reachAll: number;
  /**
   * Chance that an opening hand of 5 brings it into play: holds it, or its best line adds it to the hand or puts it on the
   * field. Holding it is exact; the rest is measured on the hands searched (an Extra Deck monster is all measured), so it
   * accounts for what one turn can do (one Normal Summon, once-per-turn effects). In the draft, before any hand is
   * searched: a hand holding it or a card that brings it in one step.
   */
  chance: number;
  /** `chance` is measured on the hands searched (an estimate, ±0.05 or so), not the draft's. Always sent by the backend. */
  measured?: boolean;
  /** Cards an opening hand reaches on average (sum of the chances) that are lost when this card's effects are negated: the key cards of the deck. */
  key: number;
}

/** The map so far while it is read (mirrors `LinksProgressView`). */
export interface LinksProgress {
  deckId?: number;
  /** Hands searched of `total`; 0 with the draft from the text. */
  done: number;
  total: number;
  map?: DeckLinks;
}

/** The map of the links between the cards of a deck (mirrors `DeckLinksView`). */
export interface DeckLinks {
  deckId: number;
  cards: LinkCard[];
  links: CardLink[];
  /** Hands played to read the links, and best lines counted for `used`. */
  hands: number;
  lines: number;
}

/** A card in both versions compared: its numbers before and after (mirrors `LinkCardChangeView`). */
export interface LinkCardChange {
  code: number;
  chanceBefore: number;
  chanceAfter: number;
  keyBefore: number;
  keyAfter: number;
  reachAllBefore: number;
  reachAllAfter: number;
}

/** Two versions of a deck on the links map (mirrors `LinksComparisonView`). */
export interface LinksComparison {
  /** The maps of the first deck and of the second. */
  before: DeckLinks;
  after: DeckLinks;
  /** Links only in the second deck, only in the first (a link is the same with the same `from`, `to` and `kind`). */
  gained: CardLink[];
  lost: CardLink[];
  /** Cards in both whose chance, key or reach changed (by 0.02 or more, or any reach), the most changed first. */
  changed: LinkCardChange[];
  /** Cards put in and taken out (codes, once each). */
  added: number[];
  removed: number[];
  /** Cards an opening hand reaches on average (sum of the chances), with the first deck and with the second. */
  reachedBefore: number;
  reachedAfter: number;
}

/** Score weights learned from the boards you preferred (mirrors `LearnedWeightsView`). */
export interface LearnedWeights {
  /** By metric key (`MetricKey` in `scoring.ts`), on the metrics of the prior given. */
  weights: Record<string, number>;
  choices: number;
  /** Share of your choices the learned weights explain (the board you preferred scores more), and the prior's. */
  agreement: number;
  priorAgreement: number;
}

const tauriApi = {
  dataStatus: () => invoke<DataStatus>("data_status"),
  updateData: (force = false) => invoke<DataStatus>("update_data", { force }),
  search: (query: string) => invoke<SearchResult>("search_cards", { query }),
  getCards: (ids: number[]) => invoke<Card[]>("get_cards", { ids }),
  facets: () => invoke<Facets>("card_facets"),
  /** Status changes of every list update, newest first; only `format`'s when given. */
  banChanges: (format?: BanChange["format"]) => invoke<BanChange[]>("ban_changes", { format: format ?? null }),
  listFavorites: () => invoke<number[]>("list_favorites"),
  setFavorite: (id: number, favorite: boolean) => invoke<void>("set_favorite", { id, favorite }),
  listSavedSearches: () => invoke<SavedSearch[]>("list_saved_searches"),
  saveSearch: (name: string, query: string) => invoke<SavedSearch[]>("save_search", { name, query }),
  deleteSavedSearch: (name: string) => invoke<SavedSearch[]>("delete_saved_search", { name }),
  onProgress: (handler: (p: Progress) => void): Promise<UnlistenFn> => listen<Progress>("data-progress", (e) => handler(e.payload)),
  listDecks: () => invoke<DeckSummary[]>("list_decks"),
  labOverview: (main: number[], roles: Record<string, number[]>) => invoke<LabOverview>("lab_overview", { main, roles }),
  labProbability: (main: number[], roles: Record<string, number[]>, cond: Cond, hand: number) =>
    invoke<{ p: number | null; error: string | null }>("lab_probability", { main, roles, cond, hand }),
  setDeckRoles: (id: number, roles: Record<string, number[]>) => invoke<void>("set_deck_roles", { id, roles }),
  guessDeckRoles: (id: number) => invoke<RoleGuess[]>("guess_deck_roles", { id }),
  onRolesProgress: (handler: (p: { done: number; total: number }) => void): Promise<UnlistenFn> =>
    listen<{ done: number; total: number }>("lab-roles-progress", (e) => handler(e.payload)),
  labSimulate: (main: number[], roles: Record<string, number[]>, cond: Cond, hand: number, rules: SimRule[], hands: number, seed: number) =>
    invoke<SimResult>("lab_simulate", { main, roles, cond, hand, rules, hands, seed }),
  labCancelSimulation: () => invoke<void>("lab_cancel_simulation"),
  onSimProgress: (handler: (c: Checkpoint) => void): Promise<UnlistenFn> => listen<Checkpoint>("lab-sim-progress", (e) => handler(e.payload)),
  labAnalyze: (main: number[], roles: Record<string, number[]>, cond: Cond, hand: number, roleKeys: string[], limits: Record<number, number>, roleA: string | null, roleB: string | null) =>
    invoke<LabAnalysis>("lab_analyze", { main, roles, cond, hand, roleKeys, limits, roleA, roleB }),
  labOptimize: (args: { main: number[]; maybe: number[]; roles: Record<string, number[]>; limits: Record<number, number>; locked: number[]; cond: Cond; weightFirst: number; minSize: number; maxSize: number; seed: number }) =>
    invoke<OptimizeResult>("lab_optimize", args),
  listRecipes: (deckId: number) => invoke<Recipe[]>("list_recipes", { deckId }),
  saveRecipe: (deckId: number | null, name: string, cond: Cond) => invoke<Recipe>("save_recipe", { deckId, name, cond }),
  deleteRecipe: (id: number) => invoke<void>("delete_recipe", { id }),
  /** Of two boards compared, you preferred the one with `chosen` metrics (by metric key); returns how many choices there are. */
  recordBoardChoice: (deckId: number | null, chosen: Record<string, number>, other: Record<string, number>) =>
    invoke<number>("record_board_choice", { deckId, chosen, other }),
  /** The weights that explain your choices, starting from `prior` (the profile in use) and pulled towards it. */
  learnedWeights: (prior: Record<string, number>) => invoke<LearnedWeights>("learned_weights", { prior }),
  clearBoardChoices: () => invoke<void>("clear_board_choices"),
  matchLog: (deckId: number | null) => invoke<MatchLog>("match_log", { deckId }),
  logMatch: (entry: MatchEntry) => invoke<MatchEntry>("log_match", { entry }),
  updateMatch: (entry: MatchEntry) => invoke<MatchEntry>("update_match", { entry }),
  deleteMatch: (id: number) => invoke<void>("delete_match", { id }),
  exportMatches: (deckId: number | null, name: string) => invoke<string>("export_matches", { deckId, name }),
  importMatches: (deckId: number, text: string) => invoke<number>("import_matches", { deckId, text }),
  sampleSize: (p0: number, p1: number) => invoke<[number, number]>("lab_sample_size", { p0, p1 }),
  shelfDecks: () => invoke<ShelfDeck[]>("shelf_decks"),
  setDeckBox: (id: number, deckBox: DeckBox) => invoke<void>("set_deck_box", { id, deckBox }),
  duplicateDeck: (id: number) => invoke<Deck>("duplicate_deck", { id }),
  getDeck: (id: number) => invoke<Deck | null>("get_deck", { id }),
  saveDeck: (deck: Deck) => invoke<Deck>("save_deck", { deck }),
  deleteDeck: (id: number) => invoke<void>("delete_deck", { id }),
  validateDeck: (deck: Deck) => invoke<Legality>("validate_deck", { deck }),
  importDeck: (text: string) => invoke<Imported>("import_deck", { text }),
  exportDeck: (deck: Deck) => invoke<{ ydk: string; ydke: string }>("export_deck", { deck }),
  comboSearch: (request: ComboRequest) => invoke<ComboOutcome>("combo_search", { request }),
  comboContinue: (deckId: number, seconds: number) => invoke<ComboOutcome>("combo_continue", { deckId, seconds }),
  /** With `vsDeckId` (a deck of the Bottega), each board is also measured against it (`ComboScore.matchup`). */
  comboScore: (vsDeckId?: number | null) => invoke<number>("combo_score", { vsDeckId: vsDeckId ?? null }),
  comboScores: () => invoke<ComboScore[]>("combo_scores"),
  onComboScore: (handler: (s: ComboScore) => void): Promise<UnlistenFn> => listen<ComboScore>("combo-score", (e) => handler(e.payload)),
  comboCancel: () => invoke<void>("combo_cancel"),
  comboExplain: (hand: number, key: string) => invoke<ComboStep[]>("combo_explain", { hand, key }),
  /** Where to stop a board's line against the Maxx "C" or Mulcharmy of a deck of the Bottega: a few seconds. */
  comboStops: (hand: number, key: string, vsDeckId: number) =>
    invoke<ComboDrawTradeOff[]>("combo_stops", { hand, key, vsDeckId }),
  /** Level 2 of the probe turn against a deck, for one board: the deck's lines are read once (10-30 s), then a few seconds a board. */
  comboMoves: (hand: number, key: string, vsDeckId: number) => invoke<ComboMoves>("combo_moves", { hand, key, vsDeckId }),
  /** What each card of a board that interrupts holds against (destroy, banish, face-down, negate): a few seconds, for the board open. */
  boardProtection: (hand: number, key: string) => invoke<ComboPiece[]>("board_protection", { hand, key }),
  /** Level 2 against the meta's most played decks (L2-3): takes about a minute the first time, then ~20s a board. */
  comboMeta: (boards?: { hand: number; key: string }[]) => invoke<number>("combo_meta", { boards: boards ?? null }),
  onComboProgress: (handler: (p: ComboProgress) => void): Promise<UnlistenFn> => listen<ComboProgress>("combo-progress", (e) => handler(e.payload)),
  /** Laboratorio, "Collegamenti": the map of a deck, read by the engine (about half a minute the first time, then kept until its cards change). */
  deckLinks: (deckId: number) => invoke<DeckLinks>("deck_links", { deckId }),
  /**
   * What changes on the map from deck A to deck B (two versions of a deck). B is read reusing A's hands: a few seconds
   * when few cards change. Progress events come for both, each with its `deckId`.
   */
  deckLinksCompare: (deckIdA: number, deckIdB: number) =>
    invoke<LinksComparison>("deck_links_compare", { deckIdA, deckIdB }),
  /**
   * The map as it is read: first (`done` 0) the draft from the cards' text, at once; then after each hand searched,
   * the map so far. `deckId` and `map` are always sent by the backend.
   */
  onLinksProgress: (handler: (p: LinksProgress) => void): Promise<UnlistenFn> =>
    listen<LinksProgress>("deck-links-progress", (e) => handler(e.payload)),
};

type Api = typeof tauriApi;

let mockMod: typeof import("./mock") | null = null;
let mockLoad: Promise<typeof import("./mock")> | null = null;

function loadMock(): Promise<typeof import("./mock")> {
  if (import.meta.env.VITE_MOCK === "1" || import.meta.env.VITE_MOCK === "true") {
    return (mockLoad ??= import("./mock").then((m) => (mockMod = m)));
  }
  throw new Error("Mock disabled");
}

if (import.meta.env.VITE_MOCK === "1" || import.meta.env.VITE_MOCK === "true") {
  void loadMock();
}

export const api: Api = isMock
  ? (new Proxy({} as Api, {
      get(_target, prop: keyof Api) {
        return (...args: unknown[]) => loadMock().then((m) => (m.mockApi[prop] as (...a: unknown[]) => unknown)(...args));
      },
    }) as Api)
  : tauriApi;

export type ImageSize = "small" | "full" | "cropped";

const windowsHost = typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

/** URL of a card image served (and cached on first use) by the backend. */
export function cardImage(id: number, size: ImageSize): string {
  if (import.meta.env.VITE_MOCK === "1" || import.meta.env.VITE_MOCK === "true") {
    if (mockMod) return mockMod.mockCardImage(id, size);
    return `data:image/svg+xml;utf8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 350"><rect width="240" height="350" rx="10" fill="#2d160d"/><text x="120" y="180" text-anchor="middle" fill="#ff9d5c" font-family="sans-serif" font-size="14">#${id}</text></svg>`)}`;
  }
  return windowsHost ? `http://cardimg.localhost/${size}/${id}` : `cardimg://localhost/${size}/${id}`;
}
