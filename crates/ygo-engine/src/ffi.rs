//! Raw declarations of the engine's C API (`ocgapi.h`, `ocgapi_types.h`).

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

pub type OCG_Duel = *mut c_void;

pub const OCG_DUEL_CREATION_SUCCESS: c_int = 0;

pub const OCG_DUEL_STATUS_END: c_int = 0;
pub const OCG_DUEL_STATUS_AWAITING: c_int = 1;
pub const OCG_DUEL_STATUS_CONTINUE: c_int = 2;

#[repr(C)]
pub struct OCG_CardData {
    pub code: u32,
    pub alias: u32,
    /// Zero-terminated array, owned by the reader until `cardReaderDone`.
    pub setcodes: *mut u16,
    pub type_: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u64,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OCG_Player {
    pub starting_lp: u32,
    pub starting_draw_count: u32,
    pub draw_count_per_turn: u32,
}

pub type OCG_DataReader = unsafe extern "C" fn(payload: *mut c_void, code: u32, data: *mut OCG_CardData);
pub type OCG_DataReaderDone = unsafe extern "C" fn(payload: *mut c_void, data: *mut OCG_CardData);
pub type OCG_ScriptReader = unsafe extern "C" fn(payload: *mut c_void, duel: OCG_Duel, name: *const c_char) -> c_int;
pub type OCG_LogHandler = unsafe extern "C" fn(payload: *mut c_void, string: *const c_char, kind: c_int);

#[repr(C)]
pub struct OCG_DuelOptions {
    pub seed: [u64; 4],
    pub flags: u64,
    pub team1: OCG_Player,
    pub team2: OCG_Player,
    pub card_reader: Option<OCG_DataReader>,
    pub payload1: *mut c_void,
    pub script_reader: Option<OCG_ScriptReader>,
    pub payload2: *mut c_void,
    pub log_handler: Option<OCG_LogHandler>,
    pub payload3: *mut c_void,
    pub card_reader_done: Option<OCG_DataReaderDone>,
    pub payload4: *mut c_void,
    pub enable_unsafe_libraries: u8,
}

#[repr(C)]
pub struct OCG_NewCardInfo {
    pub team: u8,
    pub duelist: u8,
    pub code: u32,
    pub con: u8,
    pub loc: u32,
    pub seq: u32,
    pub pos: u32,
}

#[repr(C)]
pub struct OCG_QueryInfo {
    pub flags: u32,
    pub con: u8,
    pub loc: u32,
    pub seq: u32,
    pub overlay_seq: u32,
}

extern "C" {
    pub fn OCG_GetVersion(major: *mut c_int, minor: *mut c_int);
    pub fn OCG_CreateDuel(out_duel: *mut OCG_Duel, options: *const OCG_DuelOptions) -> c_int;
    pub fn OCG_DestroyDuel(duel: OCG_Duel);
    pub fn OCG_DuelNewCard(duel: OCG_Duel, info: *const OCG_NewCardInfo);
    pub fn OCG_StartDuel(duel: OCG_Duel);
    pub fn OCG_DuelProcess(duel: OCG_Duel) -> c_int;
    pub fn OCG_DuelGetMessage(duel: OCG_Duel, length: *mut u32) -> *mut c_void;
    pub fn OCG_DuelSetResponse(duel: OCG_Duel, buffer: *const c_void, length: u32);
    pub fn OCG_LoadScript(duel: OCG_Duel, buffer: *const c_char, length: u32, name: *const c_char) -> c_int;
    pub fn OCG_DuelQueryCount(duel: OCG_Duel, team: u8, loc: u32) -> u32;
    pub fn OCG_DuelQuery(duel: OCG_Duel, length: *mut u32, info: *const OCG_QueryInfo) -> *mut c_void;
    pub fn OCG_DuelQueryLocation(duel: OCG_Duel, length: *mut u32, info: *const OCG_QueryInfo) -> *mut c_void;
    pub fn OCG_DuelQueryField(duel: OCG_Duel, length: *mut u32) -> *mut c_void;

    // shim/compile.cpp
    pub fn df_lua_compile(source: *const c_char, length: usize, name: *const c_char, out_length: *mut usize) -> *mut u8;
    pub fn df_free(p: *mut c_void);

    // shim/reuse.cpp
    pub fn df_duel_snapshot(duel: OCG_Duel) -> c_int;
    pub fn df_duel_reset(duel: OCG_Duel, options: *const OCG_DuelOptions) -> c_int;
    pub fn df_duel_destroy(duel: OCG_Duel);
    pub fn df_duel_preload(duel: OCG_Duel, codes: *const u32, count: usize) -> c_int;

    // shim/arena.cpp
    pub fn df_arena_new() -> *mut c_void;
    pub fn df_arena_free(arena: *mut c_void);
    pub fn df_arena_enter(arena: *mut c_void) -> *mut c_void;
    pub fn df_arena_used(arena: *mut c_void) -> usize;
    pub fn df_arena_overflowed(arena: *mut c_void) -> c_int;
    pub fn df_arena_save(arena: *mut c_void, out: *mut c_void);
    pub fn df_arena_restore(arena: *mut c_void, data: *const c_void, size: usize);
    pub fn df_arena_selftest() -> c_int;

    // shim/hand.cpp
    pub fn df_deck_to_hand(duel: OCG_Duel, player: u8, code: u32) -> c_int;

    // shim/count.cpp
    pub fn df_lua_hook_install(duel: OCG_Duel);
    pub fn df_lua_request_start(max: u64);
    pub fn df_lua_request_end(entries: *mut u64, calls: *mut u64, instructions: *mut u64) -> c_int;
}
