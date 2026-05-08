use std::io;
use std::io::{Write, Result};
use std::env;
use std::time::{Duration, Instant};

use crossterm::{execute, queue};
use crossterm::style::{Color, Colors, SetColors, ResetColor, Print};
use crossterm::cursor::{Hide, Show, MoveTo};
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size,
    BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen, DisableLineWrap, EnableLineWrap, Clear, ClearType,
};

// there are 176-32=144 printable ascii chars not counting space.
// show statistics after every game.
// support a flag that allows appending single game statistics to file.

const MIN_COLS: usize = 144;
const MIN_ROWS: usize = 48;

#[derive(Clone, Copy, Debug)]
enum GridCellRef {
    Raindrop(usize), // in addition to the object, store an array index.
    Cloud(usize),
}

// a copy of a grid cell which Game may pass to an object about to update.
enum GridCellSnapshot{
    // renamed Randrop to Rd just to remind that enum names don't
    // have to correspond to objects.
    Rd(Raindrop),
    Cd(Cloud),
}

struct CardinalCells {
    n: Option<GridCellSnapshot>,
    s: Option<GridCellSnapshot>,
    e: Option<GridCellSnapshot>,
    w: Option<GridCellSnapshot>,
}

impl CardinalCells {
    fn iter(&self) -> impl Iterator<Item = &GridCellSnapshot> {
        [&self.n, &self.s, &self.e, &self.w]
            .into_iter()
            .flatten()
    }
}

// game pieces that update themselves should return this to 
// the Game object if they want to be relocated.
struct UpdateResult<T> {
    object: T,
    move_to: Option<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq)]
struct CanvasCell {
    ch: char,
    fg: Color,
    bg: Color,
}

impl Default for CanvasCell{
    fn default() -> Self{
        Self {ch:'.', fg:Color::Blue, bg:Color::Black}
    }
}

type Sprite = Vec<Vec<CanvasCell>>;

fn make_sprite(rows: &[&[(char, Color, Color)]]) -> Sprite {
    rows.iter().map(|row| {
        row.iter().map(|&(ch, fg, bg)| {
            CanvasCell { ch, fg, bg }
        }).collect()
    }).collect()
}

#[derive(Debug, Clone)]
struct Raindrop { r: usize, c: usize, anim_state: i32, delete: bool }

#[derive(Debug, Clone)]
struct Cloud { r: usize, c: usize, density: i32, delete: bool }

impl Raindrop {
    fn get_sprite(&self) -> Sprite {
        let fg = Color::Red;
        let bg = Color::Black;
        make_sprite(&[
            &[('F', fg, bg), ('#', fg, bg), ('#', fg, bg)],
            &[('#', fg, bg), ('F', fg, bg), ('#', fg, bg)],
            &[('#', fg, bg), ('#', fg, bg), ('F', fg, bg)],
        ])
    }


    fn update(mut self, cardinal: &CardinalCells) -> UpdateResult<Raindrop> {
        let mut move_to = None;

        for contents in cardinal.iter() {
            match contents {
                GridCellSnapshot::Rd(b) => {
                    self.anim_state -= b.anim_state as i32;
                    // move away from an adjacent Bar by stepping south
                    move_to = Some((self.c, self.r + 1));
                }
                GridCellSnapshot::Cd(f) => {
                    self.anim_state += f.density / 10;
                }
            }
        }

        if self.anim_state <= 0 {
            self.delete = true;
        }

        UpdateResult { object: self, move_to }
    }
}

impl Cloud {
    fn get_sprite(&self) -> Sprite {
        let fg = Color::Green;
        let bg = Color::Black;
        make_sprite(&[
            &[('B', fg, bg), ('B', fg, bg), ('B', fg, bg)],
            &[('B', fg, bg), ('.', fg, bg), ('B', fg, bg)],
            &[('B', fg, bg), ('B', fg, bg), ('B', fg, bg)],
        ])
    }

    fn update(mut self, cardinal: &CardinalCells) -> UpdateResult<Cloud> {
        let mut move_to = None;

        for contents in cardinal.iter() {
            match contents {
                GridCellSnapshot::Rd(_) => {
                    self.density += 2;
                    // chase adjacent Foo by stepping east
                    move_to = Some((self.c + 1, self.r));
                }
                GridCellSnapshot::Cd(b) => {
                    self.density += b.density * 2;
                }
            }
        }

        if self.density > 10 {
            self.delete = true;
        }

        UpdateResult { object: self, move_to }
    }

}

struct Game {
    r: usize,
    c: usize,
    out: Box<dyn Write>,
    grid: Vec<Vec<Option<GridCellRef>>>,
    canvas: Vec<Vec<CanvasCell>>,
    raindrops: Vec<Raindrop>,
    clouds: Vec<Cloud>,
}

impl Game{
    fn new(r: usize, c: usize, out: Box<dyn Write>) -> Self{
        Game {
            r,
            c,
            out,
            grid: vec![vec![None; c]; r],
            canvas: vec![vec![CanvasCell::default(); c]; r],
            raindrops: Vec::new(),
            clouds: Vec::new(),
        }
    }
    
    fn cell_at(&self, x: usize, y: usize) -> Option<GridCellRef> {
        self.grid[y][x]
    }

    
    fn contents_at(&self, x: usize, y: usize) -> Option<GridCellSnapshot> {
        if x >= self.c || y >= self.r { return None; }
        match self.grid[y][x] {
            Some(GridCellRef::Raindrop(i)) => Some(GridCellSnapshot::Rd(self.raindrops[i].clone())),
            Some(GridCellRef::Cloud(i)) => Some(GridCellSnapshot::Cd(self.clouds[i].clone())),
            None                  => None,
        }
    }

    fn cardinal_contents(&self, x: usize, y: usize) -> CardinalCells {
        CardinalCells {
            n: if y > 0          { self.contents_at(x, y - 1) } else { None },
            s: if y + 1 < self.r { self.contents_at(x, y + 1) } else { None },
            e: if x + 1 < self.c { self.contents_at(x + 1, y) } else { None },
            w: if x > 0          { self.contents_at(x - 1, y) } else { None },
        }
    }

    fn blit_sprite(&mut self, x: usize, y: usize, sprite: &Sprite) {
        for (sy, row) in sprite.iter().enumerate() {
            for (sx, cell) in row.iter().enumerate() {
                let cx = x + sx;
                let cy = y + sy;

                // let '.' character in a sprite represent transparency.
                if cx < self.c && cy < self.r && cell.ch != '.' {
                    self.canvas[cy][cx] = *cell;
                }
            }
        }
    }

    fn draw(&mut self) -> Result<()>{
        for row in self.canvas.iter_mut(){
            row.fill(CanvasCell::default()); // all cells have pointer to same object?
        }

        for i in 0..self.raindrops.len() {
            let (x, y) = (self.raindrops[i].c, self.raindrops[i].r);
            let sprite = self.raindrops[i].get_sprite();
            self.blit_sprite(x, y, &sprite);
        }
        for i in 0..self.clouds.len() {
            let (x, y) = (self.clouds[i].c, self.clouds[i].r);
            let sprite = self.clouds[i].get_sprite();
            self.blit_sprite(x, y, &sprite);
        }

        let (curr_cols, curr_rows) = size()?;
        let curr_cols = curr_cols as usize;
        let curr_rows = curr_rows as usize;
        if curr_rows < self.r || curr_cols < self.c {
            execute!(self.out, Clear(ClearType::All), MoveTo(0,0))?;
            queue!(self.out, Print(format!(
                "game dims set to {} rows, {} cols, current terminal dims are {} rows, {} cols.", 
                self.r, MIN_COLS, curr_rows, curr_cols)))?;
            self.out.flush()?;
        }
        else{
            let top_left_r = (curr_rows - self.r) / 2;
            let top_left_c = (curr_cols - self.c) / 2;

            queue!(self.out, BeginSynchronizedUpdate)?;
            queue!(self.out, Clear(ClearType::All))?;

            // simple optimization to reduce writes to stdout:
            // only print color change chars when necessary.
            let mut last_colors = None;

            for i in 0..self.r {
                queue!(self.out, MoveTo(top_left_c as u16, (top_left_r + i) as u16))?;
                for j in 0..self.c {
                    let colors = Colors::new(self.canvas[i][j].fg, self.canvas[i][j].bg);
                    if Some(colors) != last_colors {
                        queue!(self.out, SetColors(colors))?;
                        last_colors = Some(colors);
                    }
                    queue!(self.out, Print(self.canvas[i][j].ch))?;
                }
            }

            // prepare to write out any post-grid contents.
            execute!(self.out, ResetColor)?;
            queue!(self.out, MoveTo(top_left_c as u16, (top_left_r + self.r) as u16))?;

            // print score, timer here?

            queue!(self.out, EndSynchronizedUpdate)?;
            self.out.flush()?;
        }

        Ok(())
    }

    fn purge_raindrops(&mut self) {
        self.raindrops.retain(|f| {
            if f.delete { self.grid[f.r][f.c] = None; }
            !f.delete
        });
        for (i, f) in self.raindrops.iter().enumerate() {
            self.grid[f.r][f.c] = Some(GridCellRef::Raindrop(i));
        }
    }

    fn purge_clouds(&mut self) {
        self.clouds.retain(|f| {
            if f.delete { self.grid[f.r][f.c] = None; }
            !f.delete
        });
        for (i, f) in self.clouds.iter().enumerate() {
            self.grid[f.r][f.c] = Some(GridCellRef::Cloud(i));
        }
    }


    // Clear whatever is at (nx, ny), then move the object at (old_x, old_y)
    // into that cell, updating both the grid and the object's coordinates.
    fn move_object(&mut self, old_x: usize, old_y: usize, nx: usize, ny: usize) {
        if nx >= self.c || ny >= self.r { return; }

        // evict whatever is currently at the destination
        match self.grid[ny][nx] {
            Some(GridCellRef::Raindrop(j)) => { self.raindrops[j].delete = true; }
            Some(GridCellRef::Cloud(j)) => { self.clouds[j].delete = true; }
            None => {}
        }
        self.grid[ny][nx] = None;

        // move the object from its old cell to the new one
        let cell_ref = self.grid[old_y][old_x].take();
        match cell_ref {
            Some(GridCellRef::Raindrop(i)) => {
                self.raindrops[i].c = nx;
                self.raindrops[i].r = ny;
                self.grid[ny][nx] = Some(GridCellRef::Raindrop(i));
            }
            Some(GridCellRef::Cloud(i)) => {
                self.clouds[i].c = nx;
                self.clouds[i].r = ny;
                self.grid[ny][nx] = Some(GridCellRef::Cloud(i));
            }
            None => {}
        }
    }


    fn update_raindrops(&mut self) {
        for i in 0..self.raindrops.len() {
            let (x, y)   = (self.raindrops[i].c, self.raindrops[i].r);
            let cardinal = self.cardinal_contents(x, y);
            let result   = self.raindrops[i].clone().update(&cardinal);
            self.raindrops[i] = result.object;
            if let Some((nx, ny)) = result.move_to {
                self.move_object(x, y, nx, ny);
            }
        }
    }

    fn update_clouds(&mut self) {
        for i in 0..self.clouds.len() {
            let (x, y)   = (self.clouds[i].c, self.clouds[i].r);
            let cardinal = self.cardinal_contents(x, y);
            let result   = self.clouds[i].clone().update(&cardinal);
            self.clouds[i] = result.object;
            if let Some((nx, ny)) = result.move_to {
                self.move_object(x, y, nx, ny);
            }
        }
    }

    fn update(&mut self) {
        self.update_raindrops();
        self.purge_raindrops();

        self.update_clouds();
        self.purge_clouds();
    }

    fn set_up(&mut self) -> Result<()>{
        enable_raw_mode()?;
        execute!(self.out, EnterAlternateScreen, Hide, DisableLineWrap)?;
        Ok(())
    }

    fn tear_down(&mut self) -> Result<()>{
        disable_raw_mode()?;
        execute!(self.out, LeaveAlternateScreen, Show, EnableLineWrap,)?;
        self.out.flush()?;
        Ok(())
    }
}

fn main() -> Result<()> {

    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <rows> <cols>", args[0]);
        std::process::exit(1);
    }
    
    let rows: usize = args[1].parse().expect("first argument (rows) must be a positive integer.");
    let cols: usize = args[2].parse().expect("second argument (cols) must be a positive integer.");

    if rows < MIN_ROWS { print!("rows must be at least {}.\n", MIN_ROWS); return Ok(()); }
    if cols < MIN_COLS { print!("cols must be at least {}.\n", MIN_COLS); return Ok(()); }

    let mut game = Game::new(rows, cols, Box::new(io::stdout()));

    game.raindrops.push(Raindrop{r:1, c:1, anim_state:2, delete:false});
    game.clouds.push(Cloud{r:4, c:4, density:3, delete:false});

    game.set_up()?;

    let tick_ms = Duration::from_millis(200);
    let time_beg = Instant::now();
    let mut quit = false;
    let mut remaining_ms = tick_ms;

    let mut key_presses = 0;

    while !quit {
        if event::poll(remaining_ms)? {
            if let Event::Key(event) = event::read()? {
                // \r is required because in raw mode the cursor doesn't automatically return.
                print!("Key pressed: {:?}\r\n", event.code);
                key_presses += 1;
                if event.code == KeyCode::Char('q') {
                    quit = true;
                }
            }
            // as_millis() returns a u128, but from_millis() requires a u64.
            let rem = tick_ms.as_millis() - (time_beg.elapsed().as_millis() % tick_ms.as_millis());
            remaining_ms = Duration::from_millis(rem as u64);
        }
        else{
            //print!("drawing...\r\n");

            game.update();
            game.draw()?;

            //let (curr_cols, curr_rows) = size()?;
            //let curr_cols = curr_cols as usize;
            //let curr_rows = curr_rows as usize;
            //if curr_rows < rows || curr_cols < cols {
            //    execute!(stdout, Clear(ClearType::All), MoveTo(0,0))?;
            //    queue!(stdout, Print(format!(
            //        "min terminal dims are {} rows, {} cols, curr dims are {}, {}", 
            //        MIN_ROWS, MIN_COLS, curr_rows, curr_cols)))?;
            //    stdout.flush()?;
            //}
            //else{
            //    let top_left_r = (curr_rows - rows) / 2;
            //    let top_left_c = (curr_cols - cols) / 2;
            //    draw(&grid, &mut stdout, top_left_r, top_left_c)?;
            //    //draw(&grid, &mut stdout, top_left_c)?;
            //}

            let rem = tick_ms.as_millis() - (time_beg.elapsed().as_millis() % tick_ms.as_millis());
            remaining_ms = Duration::from_millis(rem as u64);
        }
    }

    game.tear_down()?;
    println!("key presses: {}", key_presses);
    Ok(())
}

