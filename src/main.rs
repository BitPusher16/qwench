#![allow(unused)]

use std::io;
use std::io::{Write, Result};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::cmp::max;
use std::panic;

use crossterm::{execute, queue};
use crossterm::style::{Color, Colors, SetColors, ResetColor, Print, PrintStyledContent, Stylize};
use crossterm::cursor::{Hide, Show, MoveTo};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size,
    BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen, DisableLineWrap, EnableLineWrap, Clear, ClearType,
};
use crossterm::cursor;

use rand::prelude::*;
use rand::SeedableRng;

mod symbol_list;
use crate::symbol_list::get_symbol_list;

mod word_list;
use crate::word_list::get_word_list;

mod args;
use crate::args::{CommandLineArgs, parse_args};

const GAME_COLS: usize = 144;
const GAME_ROWS: usize = 48;

const MAX_WORD_LEN: usize = 10;
const MIN_WORD_LEN: usize = 6;
const WORD_LEN_BUFFER: usize = 2;
//const WORD_LEN_W_BUFFER: usize = MAX_WORD_LEN + WORD_LEN_BUFFER;

const MAX_SYMBOLS_LEN: usize = 6;
const MIN_SYMBOLS_LEN: usize = 4;

const GAME_OVER_I: usize = 20;
const GAME_OVER_J: usize = 64;

const MS_PER_TICK: u64 = 250;
//const MS_PER_TICK: u64 = 25;
//const GAME_LENGTH_SEC: u64 = 2 * 60;

// at or above hydration level WATERLOGGED,
// fire does not affect fire_risk,
// and update() decrements fire_risk.
const WATERLOGGED: u8 = 6;
const START_HYDRATION: u8 = 2;

// how many grass tiles on each side of a landing raindrop get watered.
const SPLASH_RADIUS: u8 = 3;
const SPLASH_VAL:u8 = 12;

#[derive(Debug, Clone)]
enum GameState {
    Play,
    Quit,
    GameLost,
    GameWon,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct CanvasCell {
    ch: char,
    fg: Color,
    bg: Color,
}

impl Default for CanvasCell{
    fn default() -> Self {
        Self {ch:'.', fg:Color::Blue, bg:Color::Black}
    }
}

type Canvas = Vec<Vec<CanvasCell>>;

//type Sprite = Vec<Vec<CanvasCell>>;

#[derive(Debug, Clone, PartialEq)]
struct Sprite {
    data: Vec<Vec<CanvasCell>>,
    shift_up: usize,
    shift_left: usize,
}

impl Sprite{
    fn push(&mut self, vec: Vec<CanvasCell>){
        self.data.push(vec);
    }

    fn new() -> Self{
        //self.data = Vec::new();
        Sprite{
            data: Vec::new(),
            shift_up: 0,
            shift_left: 0,
        }
    }
}

// TODO:
// not convinced i need to support sprite shift_up, shift_left.
// game logic is definitely easier if i do without it.
// leaving it for now, can clean up later.
fn draw_onto(
    canvas: &mut Canvas,
    sprite: &Sprite,
    start_row: usize,
    start_col: usize,
) {
    let canv_rows = canvas.len() as isize;
    if canv_rows == 0 || sprite.data.is_empty() {
        return;
    }

    let origin_row = start_row as isize - sprite.shift_up as isize;
    let origin_col = start_col as isize - sprite.shift_left as isize;

    for (i, sprite_row) in sprite.data.iter().enumerate() {
        let target_row = origin_row + i as isize;
        if target_row < 0 { continue; }
        if target_row >= canv_rows { break; }

        let canv_row = &mut canvas[target_row as usize];
        let canv_cols = canv_row.len() as isize;

        for (j, &cell) in sprite_row.iter().enumerate() {
            let target_col = origin_col + j as isize;
            if target_col < 0 { continue; }
            if target_col >= canv_cols { break; }

            canv_row[target_col as usize] = cell;
        }
    }
}

// call like:
//   let sprite = string_to_sprite(r#"
//       xbr $br zgr•
//       cbr xbr xbr•
//       xbr $br zgr•
//   "#);
fn string_to_sprite(input: &str, shift_up: usize, shift_left: usize) -> Sprite {
    fn char_to_color(c: char) -> Color {
        match c {
            'r' => Color::Red,
            'g' => Color::Green,
            'b' => Color::Blue,
            'y' => Color::Yellow,
            'm' => Color::Magenta,
            'c' => Color::Cyan,
            'w' => Color::White,
            'k' => Color::Black,
            'e' => Color::Grey,
            'R' => Color::DarkRed,
            'G' => Color::DarkGreen,
            'B' => Color::DarkBlue,
            'Y' => Color::DarkYellow,
            'M' => Color::DarkMagenta,
            'C' => Color::DarkCyan,
            'E' => Color::DarkGrey,
            _ => Color::Reset,
        }
    }

    //let mut sprite: Sprite = Vec::new();
    let mut sprite = Sprite::new();
    sprite.shift_up = shift_up;
    sprite.shift_left = shift_left;
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    for row_str in cleaned.split('•') {
        if row_str.is_empty() { continue; }
        let chars: Vec<char> = row_str.chars().collect();
        let mut row = Vec::new();
        for i in (0..chars.len()).step_by(3) {
            if i + 2 >= chars.len() { break; }
            let ch = chars[i];
            let fg = char_to_color(chars[i + 1]);
            let bg = char_to_color(chars[i + 2]);
            row.push(CanvasCell { ch, fg, bg });
        }
        if !row.is_empty() { sprite.push(row); }
    }
    //Sprite { cells }
    sprite
}

#[derive(Debug, Copy, Clone)]
struct Raindrop { anim_state: i32, delete: bool }

#[derive(Debug, Clone)]
struct Cloud { anim_state: i32, word:String, idx:usize, delete: bool }

#[derive(Debug, Copy, Clone)]
struct Empty { anim_state: i32, delete: bool }

#[derive(Debug, Copy, Clone)]
struct Badkey { anim_state: i32, delete: bool }

#[derive(Debug, Copy, Clone)]
struct Grass { anim_state: i32, fire_resist: u8, delete: bool }

#[derive(Debug, Clone)]
struct GameOverMessage { anim_state: i32, word:String, idx:usize, delete: bool }

#[derive(Debug, Clone)]
struct Clock { anim_state: i32, ms_remaining:u64, delete: bool }

#[derive(Debug, Clone)]
struct Score { anim_state: i32, hit:u64, miss:u64, delete: bool }

impl Raindrop {
    fn get_sprite(&self) -> Sprite {
        // TODO: some savings are possible here if we precompute the sprite.
        // (same for other pieces.)

        string_to_sprite(r#"
            Uwb
        "#, 0, 0)
    }

    fn update(&mut self) {
        self.anim_state = match self.anim_state {
            0 => 1,
            1 => 0,
            _ => self.anim_state
        }
    }

    fn get_delete(& self) -> bool { self.delete }
}

impl Cloud {
    fn get_sprite(&self) -> Sprite {

        let mut ret = String::new();
        for (i, c) in self.word.chars().enumerate() {
            ret += &c.to_string();
            if i < self.idx {
                ret += "ry "
            }
            else{
                ret += "bw "
            }
        }
        ret += "•";
        for i in 0..self.word.len() {
            //if self.idx == 0{ ret += "~bw "; }
            //else{ ret += "~rw "; }

            //ret += "~bw "
            ret += match self.anim_state {
                0 => "~bw",
                1 => "*bw",
                _ => "~bw"
            }
        }

        string_to_sprite(&ret, 0, 0)
    }

    fn update(&mut self) {
        //self.anim_state = match self.anim_state {
        //    0 => 1,
        //    1 => 0,
        //    _ => self.anim_state
        //}
        self.anim_state = (self.anim_state + 1) % 2;
    }

    fn get_delete(& self) -> bool { self.delete }
}

impl Empty {
    fn get_sprite(&self) -> Sprite {
        // empty string has effect of making draw() do no-op.
        string_to_sprite(r#""#, 0, 0) 
    }

    fn update(&mut self) {
    }

    fn get_delete(& self) -> bool { self.delete }
}

impl Badkey {
    fn get_sprite(&self) -> Sprite {
        //string_to_sprite(r#" .bk "#)
        string_to_sprite(r#"
            █rr █rr █rr █rr
        "#, 0, 0) // empty string has effect of making draw() do no-op.
    }

    fn update(&mut self) {
    }

    fn get_delete(& self) -> bool { self.delete }
}

impl Grass {
    fn get_sprite(&self) -> Sprite {

        match self.fire_resist {
            0 => match self.anim_state {
                0 => string_to_sprite(r#"
                    #rk•
                    "rk
                "#, 0, 0),
                1 => string_to_sprite(r#"
                    &yk•
                    "rk
                "#, 0, 0),
                2 => string_to_sprite(r#"
                    %yk•
                    "rk
                "#, 0, 0),
                3 => string_to_sprite(r#"
                    *yk•
                    "rk
                "#, 0, 0),
                _ => string_to_sprite(r#" "#, 0, 0),
            },
            //1..=2 => string_to_sprite(r#"
            1 => string_to_sprite(r#"
                vyk•
                "yk
            "#, 0, 0),
            WATERLOGGED.. => string_to_sprite(r#"
                vgk•
                "gb
            "#, 0, 0),
            _ => string_to_sprite(r#"
                vgk•
                "gk
            "#, 0, 0),
        }
    }

    fn update(&mut self) {
        self.anim_state = (self.anim_state + 1) % 4;
        if self.fire_resist >= WATERLOGGED { self.fire_resist -= 1; }
    }

    fn get_delete(& self) -> bool { self.delete }
}

impl GameOverMessage {
    fn get_sprite(&self) -> Sprite {

        let mut ret = String::new();
        for i in 0..self.word.len() { ret += "~bw " }
        ret += "•";
        for (i, c) in self.word.chars().enumerate() {
            ret += &c.to_string();
            ret += "rw "
        }
        ret += "•";
        for i in 0..self.word.len() { ret += "~bw " }

        string_to_sprite(&ret, 0, 0)
    }

    fn update(&mut self) {
    }

    fn get_delete(& self) -> bool { self.delete }
}

impl Clock {
    fn get_sprite(&self) -> Sprite {
        let total_seconds = self.ms_remaining / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;

        let time_str = format!("{:02}:{:02}", minutes, seconds);

        let mut ret = String::new();
        for ch in time_str.chars() {
            ret.push(ch);
            ret.push('w');  // white foreground
            ret.push('k');  // black background
        }

        string_to_sprite(&ret, 0, 0)
    }

    fn update(&mut self) { }

    fn get_delete(& self) -> bool { self.delete }
}

impl Score {
    fn get_sprite(&self) -> Sprite {
        let score_str = format!("{:04}:{:04}", self.hit, self.miss);

        let mut ret = String::new();
        let mut is_hit_section = true;

        for ch in score_str.chars() {
            if ch == ':' {
                is_hit_section = false;
                // colon stays neutral white on black
                ret.push(ch);
                ret.push('w');
                ret.push('k');
                continue;
            }

            let fg = if is_hit_section { 'g' } else { 'r' }; // green for hits, red for misses

            ret.push(ch);
            ret.push(fg);
            ret.push('k'); // black background for everything
        }

        string_to_sprite(&ret, 0, 0)
    }

    fn update(&mut self) { }

    fn get_delete(& self) -> bool { self.delete }
}

#[derive(Debug, Clone)]
enum GridCell {
    Rd(Raindrop),
    Cd(Cloud),
    Em(Empty),
    Bk(Badkey),
    Gs(Grass),
    Gm(GameOverMessage),
    Ck(Clock),
    Sc(Score),
}

impl Default for GridCell{
    fn default() -> Self {
        GridCell::Em(Empty {anim_state:0, delete:false})
    }
}

trait GetSprite {
    fn get_sprite(&self) -> Sprite;
}

// implementing a trait on an enum allows us to do things like this:
//
//    // Build a mixed collection
//    let mixed: Vec<Thing> = vec![
//        Thing::Foo(Foo),
//        Thing::Bar(Bar),
//        Thing::Foo(Foo),
//        Thing::Bar(Bar),
//    ];
//
//    // Option A – static dispatch, but no extra helper needed
//    for item in &mixed {
//        item.bif();
//        println!("{:?}", item);
//    }
impl GetSprite for GridCell {
    fn get_sprite(&self) -> Sprite {
        match self {
            GridCell::Rd(rd) => rd.get_sprite(),
            GridCell::Cd(cd) => cd.get_sprite(),
            GridCell::Em(em) => em.get_sprite(),
            GridCell::Bk(bk) => bk.get_sprite(),
            GridCell::Gs(gs) => gs.get_sprite(),
            GridCell::Gm(gm) => gm.get_sprite(),
            GridCell::Ck(ck) => ck.get_sprite(),
            GridCell::Sc(sc) => sc.get_sprite(),
        }
    }
}

trait Update {
    fn update(&mut self);
}

impl Update for GridCell {
    fn update(&mut self) {
        match self {
            GridCell::Rd(rd) => rd.update(),
            GridCell::Cd(cd) => cd.update(),
            GridCell::Em(em) => em.update(),
            GridCell::Bk(bk) => bk.update(),
            GridCell::Gs(gs) => gs.update(),
            GridCell::Gm(gm) => gm.update(),
            GridCell::Ck(ck) => ck.update(),
            GridCell::Sc(sc) => sc.update(),
        }
    }
}

trait GetDelete{
    fn get_delete(&self) -> bool;
}

impl GetDelete for GridCell {
    fn get_delete(& self) -> bool {
        match self {
            GridCell::Rd(rd) => rd.get_delete(),
            GridCell::Cd(cd) => cd.get_delete(),
            GridCell::Em(em) => em.get_delete(),
            GridCell::Bk(bk) => bk.get_delete(),
            GridCell::Gs(gs) => gs.get_delete(),
            GridCell::Gm(gm) => gm.get_delete(),
            GridCell::Ck(ck) => ck.get_delete(),
            GridCell::Sc(sc) => sc.get_delete(),
        }
    }
}

fn read_words_from_disk() -> Vec<String>{
    vec![
        "cloud".to_string(),
        "rain".to_string(),
        "storm".to_string(),
        "thunder".to_string(),
        "lightning".to_string(),
        "mist".to_string(),
        "fog".to_string(),
        "sky".to_string(),
        "cumulus".to_string(),
        "nimbus".to_string(),
    ]
}

#[derive(Debug)]
pub struct WordPool {
    groups: BTreeMap<char, Vec<String>>,     // keys are always lowercase
    available_starts: Vec<char>,             // contains BOTH lowercase and uppercase for each letter
}

impl WordPool {
    pub fn new(words: Vec<String>) -> Self {
        let mut groups: BTreeMap<char, Vec<String>> = BTreeMap::new();

        for word in words {
            if let Some(first_char) = word.chars().next() {
                let lower = first_char.to_ascii_lowercase();
                groups.entry(lower).or_default().push(word);
            }
        }

        // Build available_starts with BOTH lowercase and uppercase versions
        // (preserves deterministic order similar to original: a,A,b,B,…)
        let mut available_starts: Vec<char> = Vec::new();
        for &lower in groups.keys() {
            if lower.is_ascii_lowercase() {
                available_starts.push(lower);
                available_starts.push(lower.to_ascii_uppercase());
            } else {
                available_starts.push(lower); // fallback for non-ascii
            }
        }

        Self {
            groups,
            available_starts,
        }
    }

    pub fn get(&mut self, rng: &mut impl Rng) -> Option<String> {
        if self.available_starts.is_empty() {
            return None;
        }

        // Pick a random available start (may be lowercase or uppercase)
        let idx = rng.random_range(0..self.available_starts.len());
        let chosen_char = self.available_starts.swap_remove(idx);

        let lookup_key = chosen_char.to_ascii_lowercase();

        if let Some(word_list) = self.groups.get(&lookup_key) {
            if !word_list.is_empty() {
                let word_idx = rng.random_range(0..word_list.len());
                let mut word = word_list[word_idx].clone();

                // If the chosen start was uppercase, capitalize the first letter of the word
                if chosen_char.is_ascii_uppercase() && !word.is_empty() {
                    let mut chars: Vec<char> = word.chars().collect();
                    chars[0] = chars[0].to_ascii_uppercase();
                    word = chars.into_iter().collect();
                }

                return Some(word);
            }
        }

        None
    }

    pub fn put(&mut self, c: char) {
        let lower = c.to_ascii_lowercase();
        if self.groups.contains_key(&lower) && !self.available_starts.contains(&c) {
            self.available_starts.push(c); // push the exact case that was taken
        }
    }

    pub fn available_count(&self) -> usize {
        self.available_starts.len()
    }

    pub fn has_available(&self) -> bool {
        !self.available_starts.is_empty()
    }
}

#[derive(Debug)]
pub struct SymbolPool {
    groups: BTreeMap<char, Vec<char>>,
    available_starts: Vec<char>,
    min_symbols_len: usize,
    max_symbols_len: usize,
}

impl SymbolPool {
    pub fn new(symbols: Vec<char>, min_length: usize, max_length: usize) -> Self {
        let mut groups: BTreeMap<char, Vec<char>> = BTreeMap::new();

        for sym in symbols {
            groups.entry(sym).or_default().push(sym);
        }

        // Same deterministic order as WordPool (BTreeMap keys are sorted)
        let available_starts: Vec<char> = groups.keys().cloned().collect();

        Self {
            groups,
            available_starts,
            min_symbols_len: min_length,
            max_symbols_len: max_length,
        }
    }

    fn pick_random_symbol(&self, rng: &mut impl Rng) -> Option<char> {
        // no better way to get size of btree?
        let total: usize = self.groups.values().map(|v| v.len()).sum();
        if total == 0 {
            return None;
        }

        let mut pick = rng.random_range(0..total);
        for (&ch, list) in &self.groups {
            let len = list.len();
            if pick < len {
                return Some(ch);
            }
            pick -= len;
        }
        None
    }

    pub fn get(&mut self, rng: &mut impl Rng) -> Option<String> {
        if self.available_starts.is_empty() {
            return None;
        }

        // Exactly as in WordPool: pre-select and consume one available start
        let idx = rng.random_range(0..self.available_starts.len());
        let chosen_char = self.available_starts.swap_remove(idx);

        // Generate the word: starts with the chosen start symbol
        let length = rng.random_range(self.min_symbols_len..=self.max_symbols_len);
        let mut word = String::with_capacity(length);
        word.push(chosen_char);

        // Remainder symbols are drawn (sampled with multiplicity) from the full symbol pool.
        // Groups are never mutated — only the starting symbol is "in-use" (mirrors WordPool exactly).
        for _ in 1..length {
            if let Some(sym) = self.pick_random_symbol(rng) {
                word.push(sym);
            } else {
                break;
            }
        }

        Some(word)
    }

    pub fn put(&mut self, c: char) {
        if self.groups.contains_key(&c) && !self.available_starts.contains(&c) {
            self.available_starts.push(c);
        }
    }

    pub fn available_count(&self) -> usize {
        self.available_starts.len()
    }

    pub fn has_available(&self) -> bool {
        !self.available_starts.is_empty()
    }

    pub fn symbol_exists(&self, c: char) -> bool {
        self.groups.contains_key(&c)
    }
}

struct Game {
    chars_per_min: u64,
    game_length_sec: u64,
    letter_multiple: u64,
    symbol_multiple: u64,

    game_state: GameState,
    ticks: u64,
    out: Box<dyn Write>,
    grid: Vec<Vec<GridCell>>,
    update_applied: Vec<Vec<bool>>,
    canvas: Vec<Vec<CanvasCell>>,
    top_left_r: usize,
    top_left_c: usize,
    elts_updated: u32,
    char_fifo: VecDeque<char>,
    first_char_to_grid_coords: HashMap<char, (usize, usize)>,
    active_cloud_coords: (usize, usize),
    quit: bool,
    key_hit: u64,
    key_miss: u64,
    bad_press: bool,
    letters_printed: u64,
    symbols_printed: u64,
    rng: StdRng,
    word_pool: WordPool,
    symbol_pool: SymbolPool,
    debug_vec: Vec<String>,
}

impl Game{
    fn new(
        m: usize, n: usize, out: Box<dyn Write>, word_list: Vec<String>, symbol_list: Vec<char>,
        chars_per_minute: u64, game_length_sec: u64, alphabet_multiple: u64, symbol_multiple: u64,
    ) -> Self{
        Game {
            chars_per_min: chars_per_minute,
            game_length_sec: game_length_sec,
            letter_multiple: alphabet_multiple,
            symbol_multiple: symbol_multiple,

            game_state: GameState::Play,
            ticks: 0,
            out,
            grid: vec![vec![GridCell::default(); n]; m],
            update_applied: vec![vec![false; n]; m],
            canvas: vec![vec![CanvasCell::default(); n]; m],
            top_left_r: 0,
            top_left_c: 0,
            elts_updated: 0,
            char_fifo: VecDeque::new(),
            first_char_to_grid_coords: HashMap::new(),
            // NOTE: (0, 0) means DNE. no cloud will spawn with those coords.
            active_cloud_coords: (0, 0), 
            quit: false,
            key_hit: 0,
            key_miss: 0,
            bad_press: false,
            letters_printed: 0,
            symbols_printed: 0,
            //rng: SeedableRng::seed_from_u64(8),
            rng: SeedableRng::seed_from_u64(rand::random()),
            word_pool: WordPool::new(word_list),
            symbol_pool: SymbolPool::new(symbol_list, MIN_SYMBOLS_LEN, MAX_SYMBOLS_LEN),
            debug_vec: Vec::new()
        }
    }

    fn place_raindrop(&mut self){
        let (m, n) = (self.grid.len(), self.grid[0].len());
        let raindrop_search_beg: usize = self.rng.random_range(0..n);
        //self.debug_vec.push(format!("{raindrop_search_beg}"));
        let mut raindrop_search_cur = raindrop_search_beg;

        loop {
            if let GridCell::Em(em) = &self.grid[0][raindrop_search_cur] {
                self.grid[0][raindrop_search_cur] = 
                    GridCell::Rd(Raindrop{anim_state:0, delete:false});
                self.update_applied[0][raindrop_search_cur] = true;
                break;
            }

            raindrop_search_cur = (raindrop_search_cur + 1) % n;
            if raindrop_search_cur == raindrop_search_beg{ break; }
        }
    }

    fn place_cloud(&mut self, use_symbols:bool) -> u64 {

        let mut word:String = "".to_string();
        if use_symbols {
            if !self.symbol_pool.has_available(){ return 0; }
            //let word = self.symbol_pool.get(&mut self.rng).unwrap_or("error".to_string());
            word = self.symbol_pool.get(&mut self.rng).unwrap_or("error".to_string());
        }
        else{
            if !self.word_pool.has_available(){ return 0; }
            //let word = self.word_pool.get(&mut self.rng).unwrap_or("error".to_string());
            word = self.word_pool.get(&mut self.rng).unwrap_or("error".to_string());
        }

        let (m, n) = (self.grid.len(), self.grid[0].len());

        // no clouds first row. no clouds last 8 rows.
        let cloud_min: usize = n;
        let cloud_max: usize = (m*n) - (8*n);

        let cloud_search_begin: usize = self.rng.random_range(cloud_min..cloud_max);
        let mut cloud_search_cur = cloud_search_begin;

        //let (p, q) = (cloud_search_cur / n, cloud_search_cur % n);
        //self.debug_vec.push(format!("cur{p},{q}").to_string());

        // just once, walk backwards and confirm no object preceding.
        // if cloud found, check that cloud's length and jump to the end of it.
        let mut cloud_search_rev = cloud_search_cur;
        loop{
            let (i, j) = (cloud_search_rev / n, cloud_search_rev % n);

            // search reached beginning of row and it is empty.
            if( 
                //(cloud_search_rev % n) == 0
                j == 0
                && matches!(self.grid[i][j], GridCell::Em(_))
            ){ 
                //self.debug_vec.push(format!("reached row beg at {i},{j}").to_string());
                break;
            }

            // search reached first possible cloud position, is empty.
            if( 
                cloud_search_rev == cloud_min
                && matches!(self.grid[i][j], GridCell::Em(_))
            ){ 
                //self.debug_vec.push(format!("first {i},{j}").to_string());
                break; 
            }

            // encountered raindrop.
            // assume that raindrop can not exist in area covered by any cloud.
            // safe to stop backward search.
            //if let GridCell::Rd(rd) = &self.grid[i][j]{
            if matches!(self.grid[i][j], GridCell::Rd(_)){
                //self.debug_vec.push(format!("raindrop {i},{j}").to_string());
                break;
            }

            // searched back far enough to account for max possible word.
            if( 
                cloud_search_cur - cloud_search_rev == MAX_WORD_LEN + WORD_LEN_BUFFER
            ){ 
                //self.debug_vec.push(format!("enough {i},{j}").to_string());
                break;
            }

            // encountered cloud;
            if let GridCell::Cd(cd) = &self.grid[i][j]{
                //self.debug_vec.push(format!("hit cloud at {i},{j}").to_string());
                //cloud_search_cur = max(cloud_search_cur, cloud_search_rev + cd.word.len() + WORD_LEN_BUFFER);
                //cloud_search_cur += MAX_WORD_LEN + WORD_LEN_BUFFER;

                //cloud_search_cur = cloud_search_rev + cd.word.len() + WORD_LEN_BUFFER;
                cloud_search_cur = max(cloud_search_cur, cloud_search_rev + cd.word.len() + WORD_LEN_BUFFER);

                //let (u,v) = (cloud_search_begin / n, cloud_search_begin % n);
                //let (p,q) = (cloud_search_cur / n, cloud_search_cur % n);
                //self.debug_vec.push(format!("started {u},{v} bumped to {p},{q}").to_string());

                break;
            }

            cloud_search_rev -= 1;
        }

        'iter_i_j: loop{
            let (i, j) = (cloud_search_cur / n, cloud_search_cur % n);
            //self.debug_vec.push(format!("considering {i},{j}").to_string());

            if let GridCell::Em(em) = &self.grid[i][j] {
                // check if space exists for word.
                // if obstacle encountered (wall, non-empty item),
                // bump index up to the location of obstacle.

                // BUG? setting k = j+1 means we never look for a cloud in j?
                let mut k = j + 1;
                //let mut k = j;
                'iter_k: loop {

                    if (k - j) == word.len() + WORD_LEN_BUFFER {
                        // we have space for the word.
                        // place word now? or better after loop?

                        self.grid[i][j] = GridCell::Cd(Cloud{
                            anim_state:0, word:word.clone(), idx:0, delete:false});
                        if let Some(ch) = word.chars().next(){
                            self.first_char_to_grid_coords.insert(ch, (i, j));
                        }
                        break 'iter_i_j;
                    }

                    // reached end of row.
                    if k == n{ 
                        cloud_search_cur += (k - j - 1);
                        break;
                    }

                    if i*n + k == cloud_search_begin {
                        // we are about to run out of places to fit new cloud.
                        // technically still possible the word could fit.
                        // but quit early to simplify bump logic.
                        // decrement by one so the outer loop increment hits exit.
                        cloud_search_cur += (k - j - 1);
                        break;
                    }

                    if let GridCell::Cd(cd) = &self.grid[i][k]{
                        // decrement to account for increment that comes after iter_k.
                        cloud_search_cur += cd.word.len() + WORD_LEN_BUFFER - 1;
                    }

                    if !matches!(self.grid[i][k], GridCell::Em(_)) {
                        // CAUTION: this bump may jump over original search point.
                        cloud_search_cur += (k - j);
                        break;
                    }

                    k += 1;
                }
            }
            if let GridCell::Cd(cd) = &self.grid[i][j]{
                cloud_search_cur += cd.word.len() + WORD_LEN_BUFFER;
            }

            cloud_search_cur += 1;
            cloud_search_cur %= cloud_max;
            cloud_search_cur = max(cloud_search_cur, cloud_min);
            if cloud_search_cur == cloud_search_begin{ 
                //self.debug_vec.push("no space for cloud".to_string());
                break; 
            }
        }
        word.len() as u64
    }

    fn update(&mut self){
        match self.game_state {
            GameState::Play => { self.update_play(); },
            GameState::GameLost => { self.update_game_lost(); },
            GameState::GameWon => { self.update_game_won(); },
            GameState::Quit => { },
            //_ => { }
        }
    }

    fn update_game_lost(&mut self){
        self.grid[GAME_OVER_I][GAME_OVER_J] = GridCell::Gm(GameOverMessage{
            anim_state:0, idx:0, word:String::from("Game_Lost"), delete:false});
    }

    fn update_game_won(&mut self){
        self.grid[GAME_OVER_I][GAME_OVER_J] = GridCell::Gm(GameOverMessage{
            anim_state:0, idx:0, word:String::from("Game_Won"), delete:false});
    }

    fn update_play(&mut self) {
        self.ticks += 1;

        // NOTE: update_applied must be cleared before placing raindrops or clouds.
        // placing a raindrop or a cloud will both update a cell.
        for row in &mut self.update_applied { row.fill(false); }

        if self.ticks % 2 == 0{ self.place_raindrop(); }

        //if self.ticks % 12 == 0{ self.place_cloud(true);}
        //else if self.ticks % 2 == 0{ self.place_cloud(false); }

        // determine whether to add a cloud.

        let expected_chars = self.chars_per_min * self.ticks * MS_PER_TICK / 60000;
        let curr_chars = self.letters_printed + self.symbols_printed;
        if curr_chars < expected_chars {
            // determine whether to place letters or symbols.
            //self.letters_printed += self.place_cloud(false);


            // NOTE: to prevent division by zero errors, we add a small float to denominators.
            // this sometimes causes symbol clouds to appear even when symbol ratio set to 0.
            // acceptable bug for now.

            let current_letter_to_symbol_ratio = 
                self.letters_printed as f64 / (self.symbols_printed as f64 + 0.001);
            let desired_letter_to_symbol_ratio = 
                self.letter_multiple as f64 / (self.symbol_multiple as f64 + 0.001);
            //self.debug_vec.push(format!("{desired_letter_to_symbol_ratio}"));

            if current_letter_to_symbol_ratio < desired_letter_to_symbol_ratio {
                self.letters_printed += self.place_cloud(false);
            }
            else{
                self.symbols_printed += self.place_cloud(true);
            }
        }

        let (m, n) = (self.grid.len(), self.grid[0].len());
        for i in 0..m {
            for j in 0..n {
                ////if let Some(mut elt) = self.grid[i][j] {  // hm, yuck... this makes a copy.
                ////if let Some(ref mut elt) = self.grid[i][j] {  // also works.
                //if let Some(elt) = &mut self.grid[i][j] {  // works.
                //    elt.update(); 
                //    self.elts_updated += 1;
                //}

                // if we have already finalized this cell (possibly while examining previous cell),
                // do nothing.
                if self.update_applied[i][j]{ continue; }
                self.update_applied[i][j] = true;

                if self.grid[i][j].get_delete(){
                    self.grid[i][j] = GridCell::default();
                }
                self.grid[i][j].update();

                // piece interaction logic.
                match &self.grid[i][j] {
                    GridCell::Rd(rd) => {
                        // if raindrop has reached bottom of grid, erase it.
                        // (this should never happen. raindrop should hit either grass or cloud.)
                        if i + 1 == m{ 
                            self.grid[i][j] = GridCell::default(); 
                            continue;
                        }

                        // search the row below raindrop.
                        // start with the cell immediately below raindrop and search left.
                        // if encountered object is cloud, check cloud length,
                        // and if raindrop collides with cloud, delete raindrop.

                        let (mut i_search, mut j_search) = (i+1, j);
                        while !matches!(&self.grid[i_search][j_search], GridCell::Cd(_)) && j_search > 0 {
                            j_search -= 1;
                            //j_search -= 1000; // uncomment to test panic hook in terminal.
                        }

                        if let GridCell::Cd(cd) = &self.grid[i_search][j_search] {
                            let word_len = cd.word.len();
                            // CAUTION: if by some chance word_len is 0 and j_search is 0,
                            // we will subtract 1 from a usize and cause a panic.
                            if(j_search + word_len - 1 >= j){
                                self.grid[i][j] = GridCell::default();
                                continue;
                            }
                        }

                        // search for grass immediately below raindrop.
                        // if found, hydrate within splash radius.

                        if i+1 < m && matches!(&self.grid[i+1][j], GridCell::Gs(_)){
                            let (i_gs, mut j_gs) = (i+1, j.saturating_sub(SPLASH_RADIUS as usize));
                            while j_gs < n && j_gs <= j.saturating_add(SPLASH_RADIUS as usize){
                                if let GridCell::Gs(mut gs) = self.grid[i_gs][j_gs].clone(){
                                    gs.fire_resist = SPLASH_VAL;
                                    self.grid[i_gs][j_gs] = GridCell::Gs(gs);
                                    self.update_applied[i_gs][j_gs] = true;
                                }

                                j_gs += 1;
                            }
                            // do not continue. fall through so that raindrop gets deleted.
                        }

                        // search for any non-empty object below raindrop.
                        if i +1 < m && !matches!(&self.grid[i+1][j], GridCell::Em(_)){
                            // delete raindrop.
                            self.grid[i][j] = GridCell::default();
                            continue;
                        }

                        // no objects encountered. move raindrop down one row.

                        //self.grid[i+1][j] = self.grid[i][j];
                        //self.grid[i+i][j] = GridCell::Rd(rd);
                        self.grid[i+1][j] = self.grid[i][j].clone();
                        self.grid[i][j] = GridCell::default();

                        // we have moved raindrop into next cell down,
                        // and we should not update the destination cell when we update next row.
                        self.update_applied[i+1][j] = true;
                    }
                    GridCell::Cd(cd) => {
                    }
                    GridCell::Em(em) => {
                    }
                    GridCell::Bk(bk) => {
                    }
                    GridCell::Gs(gs) => {
                        // can't find a way to edit gs directly. clone instead.
                        let mut tmp = gs.clone();
                        if j+1 < n && let GridCell::Gs(gs_r) = self.grid[i][j+1] {
                            if tmp.fire_resist > 0 && tmp.fire_resist < WATERLOGGED && gs_r.fire_resist == 0 {
                                tmp.fire_resist -= 1;
                                //self.grid[i][j] = GridCell::Gs(tmp);
                            }
                        }
                        if j > 0 && let GridCell::Gs(gs_l) = self.grid[i][j-1] {
                            if tmp.fire_resist > 0 && tmp.fire_resist < WATERLOGGED && gs_l.fire_resist == 0 {
                                tmp.fire_resist -= 1;
                                //self.grid[i][j] = GridCell::Gs(tmp);
                            }
                        }
                        self.grid[i][j] = GridCell::Gs(tmp);
                    }
                    GridCell::Gm(gm) => {
                    }
                    GridCell::Ck(ck) => {
                        let mut tmp = ck.clone();
                        tmp.ms_remaining = (self.game_length_sec * 1000) - (self.ticks * MS_PER_TICK);
                        self.grid[i][j] = GridCell::Ck(tmp);
                    }
                    GridCell::Sc(sc) => {
                        let mut tmp = sc.clone();
                        tmp.hit = self.key_hit;
                        tmp.miss = self.key_miss;
                        self.grid[i][j] = GridCell::Sc(tmp);
                    }
                }
            }
        }

        // make sure the rightmost grass cell is always on fire.
        if let GridCell::Gs(gs) = &self.grid[m-2][n-1] {
            let mut tmp = gs.clone();
            tmp.fire_resist = 0;
            self.grid[m-2][n-1] = GridCell::Gs(tmp);
        }

        // check condition for game lost.
        if let GridCell::Gs(gs) = &self.grid[m-2][0] {
            if gs.fire_resist == 0{
                self.game_state = GameState::GameLost;
            }
        }

        // check condition for game won.
        if self.ticks * MS_PER_TICK >= (self.game_length_sec * 1000){
            self.game_state = GameState::GameWon
        }
    }

    fn draw(&mut self) -> Result<()> {
        for row in self.canvas.iter_mut(){
            row.fill(CanvasCell::default());
        }

        // copy object sprites to canvas.
        let (m, n) = (self.grid.len(), self.grid[0].len());
        for i in 0..m {
            for j in 0..n {
                let sprite = self.grid[i][j].get_sprite();
                draw_onto(&mut self.canvas,& sprite, i, j);
            }
        }

        if (
            matches!(self.game_state, GameState::GameWon) 
            || matches!(self.game_state, GameState::GameLost)
        ){
            let sprite = self.grid[GAME_OVER_I][GAME_OVER_J].get_sprite();
            draw_onto(&mut self.canvas,& sprite, GAME_OVER_I, GAME_OVER_J);
        }

        let (curr_cols, curr_rows) = size()?;
        let curr_cols = curr_cols as usize;
        let curr_rows = curr_rows as usize;
        if curr_rows < m || curr_cols < n {
            //execute!(self.out, Clear(ClearType::All), MoveTo(0,0))?;
            queue!(self.out, Clear(ClearType::All), MoveTo(0,0))?;
            queue!(self.out, Print(format!(
                "minimum dims {} rows, {} cols, terminal dims at {} rows, {} cols. \
                resize to continue. esc to quit.",
                m, n, curr_rows, curr_cols)))?;
            self.out.flush()?;
        }
        else{
            // we will center the game grid in the available terminal space.
            self.top_left_r = (curr_rows - m) / 2;
            self.top_left_c = (curr_cols - n) / 2;

            queue!(self.out, BeginSynchronizedUpdate)?;
            queue!(self.out, Clear(ClearType::All))?;

            // simple optimization to reduce writes to stdout:
            // only print color change chars when necessary.
            let mut last_colors = None;

            for i in 0..m {
                queue!(self.out, MoveTo(self.top_left_c as u16, (self.top_left_r + i) as u16))?;
                for j in 0..n {
                    let colors = Colors::new(self.canvas[i][j].fg, self.canvas[i][j].bg);
                    if Some(colors) != last_colors {
                        queue!(self.out, SetColors(colors))?;
                        last_colors = Some(colors);
                    }
                    queue!(self.out, Print(self.canvas[i][j].ch))?;
                }
            }

            // prepare to write out any post-grid contents.
            //execute!(self.out, ResetColor)?;
            queue!(self.out, ResetColor)?;
            queue!(self.out, MoveTo(self.top_left_c as u16, (self.top_left_r + m) as u16))?;

            // print score, timer here?
            //execute!(self.out, Print("foo"))?;
            //execute!(self.out, Print(format!("{:?}", self.char_fifo)))?;
            //queue!(self.out, Print(format!("{:?}", self.char_fifo)))?;

            //queue!(self.out, Print(format!("{:?}", self.debug_vec)))?;

            let n = 16;
            let inner: String = self.debug_vec .iter() .enumerate() .map(|(i, item)| {
                if i > 0 && i % n == 0 {
                    // Start a new line + indentation when we hit every nth element
                    format!("\r\n  {:?}", item)
                } else {
                    format!("{:?}", item)
                }
            }) .collect::<Vec<_>>() .join(", ");
            let formatted_debug_info = format!("\r[{}]", inner);
            //queue!(self.out, Print(formatted_debug_info))?;

            queue!(self.out, EndSynchronizedUpdate)?;
            self.out.flush()?;
        }
        Ok(())
    }

    fn handle_key_press(&mut self, c:char) -> Result<()> {
        //self.debug_vec.push("k".to_string());

        // setting active_cloud_coords to 0, 0 signifies no active cloud.

        // BUG: on returning a first_char to pool,
        // this function needs to first check if the char returns to 
        // symbol_pool or word_pool.

        let mut inc_index = false;

        if self.active_cloud_coords == (0, 0){
            //if let Some(coords) = self.first_char_to_grid_coords.get(&c) {
            if let Some((key, coords)) = self.first_char_to_grid_coords.remove_entry(&c) {

                // no cloud is active,
                // but user has entered a char with an available cloud.

                // anything better than deref?
                //let (active_i, active_j) = *coords;
                let (active_i, active_j) = (coords.0, coords.1);

                self.active_cloud_coords = (active_i, active_j);

                if let GridCell::Cd(cd) = &mut self.grid[active_i][active_j]{
                    inc_index = true;
                }
                self.key_hit += 1;
            }
            else{
                // no cloud is active,
                // and user typed a char with no mapped cloud.
                self.bad_press = true;
                self.key_miss += 1;
            }
        }
        else{
            // there is an active cloud.
            let (active_i, active_j) = self.active_cloud_coords;
            if let GridCell::Cd(cd) = &mut self.grid[active_i][active_j] {
                if let Some(ch) = cd.word.chars().nth(cd.idx) {
                    if ch == c {
                        // the user's press matches the next letter in the word.
                        inc_index = true;
                        self.key_hit += 1;
                    }
                    else{
                        self.bad_press = true;
                        self.key_miss += 1;
                    }
                }
            }
        }

        if inc_index {
            let (active_i, active_j) = self.active_cloud_coords;
            if let GridCell::Cd(cd) = &mut self.grid[active_i][active_j] {
                if let Some(ch) = cd.word.chars().nth(cd.idx) {
                    // a bit of extra math needed to find where current letter is drawn.

                    //let (char_i, char_j) = (self.top_left_r+active_i+1, self.top_left_c+active_j+cd.idx);
                    let (char_i, char_j) = (self.top_left_r+active_i, self.top_left_c+active_j+cd.idx);

                    execute!(self.out, 
                        // MoveTo is (column, row).
                        cursor::MoveTo(char_j as u16, char_i as u16),
                        PrintStyledContent(ch.with(Color::Red).on(Color::Yellow))
                    )?;

                    cd.idx += 1;
                    if cd.idx == cd.word.len(){
                        cd.delete = true;
                        self.active_cloud_coords = (0, 0);
                        if let Some(ch) = cd.word.chars().next(){
                            if self.symbol_pool.symbol_exists(ch){
                                self.symbol_pool.put(ch);
                            }
                            else{
                                self.word_pool.put(ch);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn set_up(&mut self) -> Result<()>{
        enable_raw_mode()?;
        execute!(self.out, EnterAlternateScreen, Hide, DisableLineWrap)?;
        self.out.flush()?;
        Ok(())
    }

    fn tear_down(&mut self) -> Result<()>{
        disable_raw_mode()?;
        execute!(self.out, LeaveAlternateScreen, Show, EnableLineWrap,)?;
        queue!(self.out, Clear(ClearType::All), MoveTo(0,0))?;
        self.out.flush()?;
        println!("elts updated: {}", self.elts_updated);
        Ok(())
    }

    fn game_loop(&mut self) -> Result<()>{

        // add some objects for debugging.
        //self.grid[2][2] = Some(GridCell::Rd(Raindrop{r:0, c:0, anim_state:1, delete:false}));
        //self.grid[8][8] = Some(GridCell::Cd(Cloud{r:0, c:0, anim_state:0, delete:false}));

        //self.grid[2][2] = GridCell::Rd(Raindrop{r:0, c:0, anim_state:1, delete:false});
        //self.grid[2][10] = GridCell::Rd(Raindrop{r:0, c:0, anim_state:1, delete:false});
        //self.grid[2][14] = GridCell::Rd(Raindrop{r:0, c:0, anim_state:1, delete:false});
        //self.grid[8][8] = GridCell::Cd(Cloud{r:0, c:0, anim_state:0, 
        //    word:String::from("foobar"), idx:0, delete:false});
        //self.grid[16][2] = GridCell::Cd(Cloud{r:0, c:0, anim_state:0, 
        //    word:String::from("bar"), idx:0, delete:false});
        //self.grid[16][22] = GridCell::Cd(Cloud{r:0, c:0, anim_state:0, 
        //    word:String::from("cat"), idx:0, delete:false});

        //self.first_char_to_grid_coords.insert('f', (8, 8));
        //self.first_char_to_grid_coords.insert('b', (16, 2));
        //self.first_char_to_grid_coords.insert('c', (16, 22));

        //self.grid[0][32] = GridCell::Bk(Badkey{r:0, c:0, anim_state:0, delete:false});

        //self.grid[2][2] = GridCell::Gs(Grass{r:0, c:0, anim_state:0, delete:false});

        // TODO: board setup should itself be a game state.
        // this state should precede PlayGame.

        let (m, n) = (self.grid.len(), self.grid[0].len());
        for j in 0..n {
            self.grid[m-2][j] = GridCell::Gs(Grass{
                anim_state:self.rng.random_range(0..3), fire_resist:START_HYDRATION, delete:false});
            //let idx = rng.random_range(0..self.available_starts.len());
            //self.grid[m-2][j] = GridCell::Gs(Grass{r:0, c:0, anim_state:0, fire_resist:1, delete:false});
        }
        //self.grid[m-2][n-1] = GridCell::Gs(Grass{r:0, c:0, anim_state:0, fire_resist:0, delete:false});

        //for j in n-10..n-4{
        //    self.grid[m-2][j] = GridCell::Gs(Grass{
        //        r:0, c:0, anim_state:0, fire_resist:SPLASH_VAL, delete:false});
        //}

        // this line will create fire near the left side, for testing game over.
        //self.grid[m-2][2] = GridCell::Gs(Grass{r:0, c:0, anim_state:0, fire_resist:0, delete:false});

        // create clock
        self.grid[0][0] = GridCell::Ck(Clock{anim_state:0, ms_remaining:0, delete:false});

        // create score
        self.grid[0][n-9] = GridCell::Sc(Score{anim_state:0, hit:0, miss:0, delete:false});

        //let tick_dur = Duration::from_millis(600);
        let tick_dur = Duration::from_millis(MS_PER_TICK);
        let time_beg = Instant::now();
        //let mut quit = false;
        let mut remaining_dur = tick_dur;

        //let mut key_presses = 0;

        //while !quit {
        while !matches!(self.game_state, GameState::Quit) {
            // TODO: implement a delay for when bad press happens.
            // the delay should not allow key presses to accumulate.
            // keys pressed during a bad press delay should be discarded.
            // for now, just reset bad_press back to false.
            self.bad_press = false;

            // the terminal will queue keys ready for reading.
            // we can retrieve all available keys by repeatedly calling poll.
            // if a key press is available, poll probably returns immediately, not confirmed.
            if event::poll(remaining_dur)? {
                if let Event::Key(event) = event::read()? {
                    // \r is required because in raw mode the cursor doesn't automatically return.
                    //print!("Key pressed: {:?}\r\n", event.code);
                    //key_presses += 1;
                    //if event.code == KeyCode::Char('q') {

                    if event.code == KeyCode::Esc {
                        //quit = true;
                        self.game_state = GameState::Quit;
                    }
                    else{
                        if event.kind == KeyEventKind::Press{
                            if let KeyCode::Char(c) = event.code{
                                //self.char_fifo.push_back(c);
                                if matches!(self.game_state, GameState::Play){
                                    self.handle_key_press(c);
                                }
                            }
                        }
                    }
                }
                // as_millis() returns a u128, but from_millis() requires a u64.
                let rem = tick_dur.as_millis() - (time_beg.elapsed().as_millis() % tick_dur.as_millis());
                remaining_dur = Duration::from_millis(rem as u64);
            }
            else{
                self.update();
                self.draw()?;

                let rem = tick_dur.as_millis() - (time_beg.elapsed().as_millis() % tick_dur.as_millis());
                remaining_dur = Duration::from_millis(rem as u64);
            }
        }
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        
        // set up panic hook before enabling raw mode.
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            // best effort teardown. ignore errors.
            let _ = disable_raw_mode();
            let _ = execute!(
                // NOTE: stderr, not stdout.
                // stdout also works.
                io::stderr(),
                LeaveAlternateScreen,
                Show,
                EnableLineWrap
            );
            // let the default hook print the trace.
            default_hook(info);
        }));

        self.set_up()?;
        self.game_loop()?;
        self.tear_down()?;
        
        // restore the original hook.
        panic::set_hook(Box::new(|info| {
            eprintln!("{info}");
        }));

        Ok(())
    }
}

fn main() -> Result<()> {
    // TODO: parse command line args.
    let args = parse_args();
    
    //let word_list = read_words_from_disk();
    let word_list = word_list::get_word_list();
    let symbol_list = symbol_list::get_symbol_list();
    let mut game = Game::new(
        GAME_ROWS, GAME_COLS, Box::new(io::stdout()), word_list, symbol_list,
        args.chars_per_min, args.game_length_sec, args.letter_multiple, args.symbol_multiple,
    );
    game.run()?;
    Ok(())
}
