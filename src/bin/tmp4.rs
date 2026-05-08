use std::io::{self, Write};
use crossterm::style::Color;

// ── Sprite ────────────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
struct CanvasCell {
    ch: char,
    fg: Color,
    bg: Color,
}

type Sprite = Vec<Vec<CanvasCell>>;

fn make_sprite(rows: &[&[(char, Color, Color)]]) -> Sprite {
    rows.iter().map(|row| {
        row.iter().map(|&(ch, fg, bg)| {
            CanvasCell { ch, fg, bg }
        }).collect()
    }).collect()
}

// ── Cell reference ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
enum CellRef {
    Foo(usize),
    Bar(usize),
}

// ── Cell contents — owned snapshot of whatever is at a grid cell ──────────────

#[derive(Clone, Debug)]
enum CellContents {
    Foo(Foo),
    Bar(Bar),
}

// ── Cardinal cells ────────────────────────────────────────────────────────────

struct CardinalCells {
    n: Option<CellContents>,
    s: Option<CellContents>,
    e: Option<CellContents>,
    w: Option<CellContents>,
}

impl CardinalCells {
    fn iter(&self) -> impl Iterator<Item = &CellContents> {
        [&self.n, &self.s, &self.e, &self.w]
            .into_iter()
            .flatten()
    }
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Foo { x: usize, y: usize, health: i32, dead: bool }

#[derive(Clone, Debug)]
struct Bar { x: usize, y: usize, speed: f32, dead: bool }

impl Foo {
    fn get_sprite(&self) -> Sprite {
        let fg = Color::Red;
        let bg = Color::Black;
        make_sprite(&[
            &[('F', fg, bg), ('#', fg, bg), ('#', fg, bg)],
            &[('#', fg, bg), ('F', fg, bg), ('#', fg, bg)],
            &[('#', fg, bg), ('#', fg, bg), ('F', fg, bg)],
        ])
    }

    fn update(mut self, cardinal: &CardinalCells) -> Self {
        for contents in cardinal.iter() {
            match contents {
                CellContents::Bar(b) => {
                    self.health -= b.speed as i32;
                }
                CellContents::Foo(f) => {
                    self.health += f.health / 10;
                }
            }
        }
        if self.health <= 0 {
            self.dead = true;
        }
        self
    }
}

impl Bar {
    fn get_sprite(&self) -> Sprite {
        let fg = Color::Blue;
        let bg = Color::Black;
        make_sprite(&[
            &[('B', fg, bg), ('B', fg, bg), ('B', fg, bg)],
            &[('B', fg, bg), ('.', fg, bg), ('B', fg, bg)],
            &[('B', fg, bg), ('B', fg, bg), ('B', fg, bg)],
        ])
    }

    fn update(mut self, cardinal: &CardinalCells) -> Self {
        for contents in cardinal.iter() {
            match contents {
                CellContents::Foo(_) => {
                    self.speed += 0.5;
                }
                CellContents::Bar(b) => {
                    self.speed += b.speed * 0.1;
                }
            }
        }
        if self.speed > 10.0 {
            self.dead = true;
        }
        self
    }
}

// ── Game ──────────────────────────────────────────────────────────────────────

struct Game {
    w:      usize,
    h:      usize,
    grid:   Vec<Vec<Option<CellRef>>>,
    canvas: Vec<Vec<CanvasCell>>,
    out:    Box<dyn Write>,
    foos:   Vec<Foo>,
    bars:   Vec<Bar>,
}

impl Game {
    fn new(w: usize, h: usize, out: Box<dyn Write>) -> Self {
        let blank = CanvasCell { ch: '.', fg: Color::White, bg: Color::Black };
        Game {
            w,
            h,
            grid:   vec![vec![None; w]; h],
            canvas: vec![vec![blank; w]; h],
            out,
            foos:   Vec::new(),
            bars:   Vec::new(),
        }
    }

    // Return an owned clone of whatever object is at the given cell, if any.
    fn contents_at(&self, x: usize, y: usize) -> Option<CellContents> {
        if x >= self.w || y >= self.h { return None; }
        match self.grid[y][x] {
            Some(CellRef::Foo(i)) => Some(CellContents::Foo(self.foos[i].clone())),
            Some(CellRef::Bar(i)) => Some(CellContents::Bar(self.bars[i].clone())),
            None                  => None,
        }
    }

    // Return owned snapshots of the four cardinal neighbours.
    fn cardinal_contents(&self, x: usize, y: usize) -> CardinalCells {
        CardinalCells {
            n: if y > 0          { self.contents_at(x, y - 1) } else { None },
            s: if y + 1 < self.h { self.contents_at(x, y + 1) } else { None },
            e: if x + 1 < self.w { self.contents_at(x + 1, y) } else { None },
            w: if x > 0          { self.contents_at(x - 1, y) } else { None },
        }
    }

    fn blit_sprite(&mut self, x: usize, y: usize, sprite: &Sprite) {
        for (sy, row) in sprite.iter().enumerate() {
            for (sx, cell) in row.iter().enumerate() {
                let cx = x + sx;
                let cy = y + sy;
                if cx < self.w && cy < self.h && cell.ch != '.' {
                    self.canvas[cy][cx] = *cell;
                }
            }
        }
    }

    fn draw(&mut self) {
        let blank = CanvasCell { ch: '.', fg: Color::White, bg: Color::Black };
        for row in self.canvas.iter_mut() {
            row.fill(blank);
        }
        for i in 0..self.foos.len() {
            let (x, y) = (self.foos[i].x, self.foos[i].y);
            let sprite = self.foos[i].get_sprite();
            self.blit_sprite(x, y, &sprite);
        }
        for i in 0..self.bars.len() {
            let (x, y) = (self.bars[i].x, self.bars[i].y);
            let sprite = self.bars[i].get_sprite();
            self.blit_sprite(x, y, &sprite);
        }
        for row in &self.canvas {
            let line: String = row.iter().map(|c| c.ch).collect();
            writeln!(self.out, "{}", line).unwrap();
        }
    }
}

// ── Purge helpers ─────────────────────────────────────────────────────────────

impl Game {
    fn purge_foos(&mut self) {
        for f in &self.foos {
            if f.dead { self.grid[f.y][f.x] = None; }
        }
        self.foos.retain(|f| !f.dead);
        for (i, f) in self.foos.iter().enumerate() {
            self.grid[f.y][f.x] = Some(CellRef::Foo(i));
        }
    }

    fn purge_bars(&mut self) {
        for b in &self.bars {
            if b.dead { self.grid[b.y][b.x] = None; }
        }
        self.bars.retain(|b| !b.dead);
        for (i, b) in self.bars.iter().enumerate() {
            self.grid[b.y][b.x] = Some(CellRef::Bar(i));
        }
    }
}

// ── Update loop ───────────────────────────────────────────────────────────────

impl Game {
    fn update_foos(&mut self) {
        for i in 0..self.foos.len() {
            let (x, y)   = (self.foos[i].x, self.foos[i].y);
            let cardinal = self.cardinal_contents(x, y);
            // clone() gives update() an owned copy to work with; the result
            // is written back, replacing the original.
            self.foos[i] = self.foos[i].clone().update(&cardinal);
        }
    }

    fn update_bars(&mut self) {
        for i in 0..self.bars.len() {
            let (x, y)   = (self.bars[i].x, self.bars[i].y);
            let cardinal = self.cardinal_contents(x, y);
            self.bars[i] = self.bars[i].clone().update(&cardinal);
        }
    }

    fn update(&mut self) {
        self.update_foos();
        self.purge_foos();
        self.update_bars();
        self.purge_bars();
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut game = Game::new(8, 8, Box::new(io::stdout()));

    game.foos.push(Foo { x: 2, y: 2, health: 100, dead: false });
    game.grid[2][2] = Some(CellRef::Foo(0));

    game.bars.push(Bar { x: 3, y: 2, speed: 8.0, dead: false });
    game.grid[2][3] = Some(CellRef::Bar(0));

    println!("-- before --");
    game.draw();

    game.update();

    println!("-- after --");
    game.draw();
}
