// https://claude.ai/chat/3d3548f3-4863-4fc7-87a0-ac843264114b
//
//
// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Foo { x: usize, y: usize, health: i32, dead: bool }

#[derive(Debug)]
struct Bar { x: usize, y: usize, speed: f32, dead: bool }

#[derive(Debug)]
struct Baz { x: usize, y: usize, energy: f32, dead: bool }

// ── Cell reference ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
enum CellRef {
    Foo(usize),
    Bar(usize),
    Baz(usize),
}

// ── Game ──────────────────────────────────────────────────────────────────────

struct Game {
    w: usize,
    h: usize,
    grid:   Vec<Vec<Option<CellRef>>>,
    canvas: Vec<Vec<char>>,
    foos: Vec<Foo>,
    bars: Vec<Bar>,
    bazs: Vec<Baz>,
}

impl Game {
    fn new(w: usize, h: usize) -> Self {
        Game {
            w,
            h,
            grid:   vec![vec![None; w]; h],
            canvas: vec![vec!['.'; w]; h],
            foos: Vec::new(),
            bars: Vec::new(),
            bazs: Vec::new(),
        }
    }

    fn draw(&mut self) {
        for row in self.canvas.iter_mut() {
            row.fill('.');
        }
        for y in 0..self.h {
            for x in 0..self.w {
                self.canvas[y][x] = match self.grid[y][x] {
                    Some(CellRef::Foo(_)) => 'F',
                    Some(CellRef::Bar(_)) => 'B',
                    Some(CellRef::Baz(_)) => 'Z',
                    None                  => '.',
                };
            }
        }
    }

    fn print_canvas(&self) {
        for row in &self.canvas {
            let line: String = row.iter().collect();
            println!("{}", line);
        }
    }
}

// ── Neighbour helper ──────────────────────────────────────────────────────────

fn neighbours(x: usize, y: usize, w: usize, h: usize, grid: &Vec<Vec<Option<CellRef>>>) -> Vec<CellRef> {
    let mut out = Vec::new();
    let (x, y) = (x as isize, y as isize);
    for dy in -1..=1_isize {
        for dx in -1..=1_isize {
            if dx == 0 && dy == 0 { continue; }
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < w as isize && ny < h as isize {
                if let Some(r) = grid[ny as usize][nx as usize] {
                    out.push(r);
                }
            }
        }
    }
    out
}

// ── Purge helpers ─────────────────────────────────────────────────────────────

impl Game {
    fn purge_foos(&mut self) {
        self.foos.retain(|f| {
            if f.dead { self.grid[f.y][f.x] = None; }
            !f.dead
        });
        for (i, f) in self.foos.iter().enumerate() {
            self.grid[f.y][f.x] = Some(CellRef::Foo(i));
        }
    }

    fn purge_bars(&mut self) {
        self.bars.retain(|b| {
            if b.dead { self.grid[b.y][b.x] = None; }
            !b.dead
        });
        for (i, b) in self.bars.iter().enumerate() {
            self.grid[b.y][b.x] = Some(CellRef::Bar(i));
        }
    }

    fn purge_bazs(&mut self) {
        self.bazs.retain(|b| {
            if b.dead { self.grid[b.y][b.x] = None; }
            !b.dead
        });
        for (i, b) in self.bazs.iter().enumerate() {
            self.grid[b.y][b.x] = Some(CellRef::Baz(i));
        }
    }
}

// ── Per-type update logic ─────────────────────────────────────────────────────

impl Game {
    fn update_foos(&mut self) {
        for i in 0..self.foos.len() {
            let (x, y) = (self.foos[i].x, self.foos[i].y);
            let nbrs = neighbours(x, y, self.w, self.h, &self.grid);

            for cell_ref in nbrs {
                match cell_ref {
                    CellRef::Bar(j) => {
                        let penalty = self.bars[j].speed as i32;
                        self.foos[i].health -= penalty;
                    }
                    CellRef::Baz(j) => {
                        let boost = self.bazs[j].energy as i32;
                        self.foos[i].health += boost;
                    }
                    CellRef::Foo(_) => {}
                }
            }

            if self.foos[i].health <= 0 {
                self.foos[i].dead = true;
            }
        }
    }

    fn update_bars(&mut self) {
        for i in 0..self.bars.len() {
            let (x, y) = (self.bars[i].x, self.bars[i].y);
            let nbrs = neighbours(x, y, self.w, self.h, &self.grid);

            for cell_ref in nbrs {
                match cell_ref {
                    CellRef::Baz(j) => {
                        self.bars[i].speed += self.bazs[j].energy * 0.1;
                    }
                    _ => {}
                }
            }

            if self.bars[i].speed > 10.0 {
                self.bars[i].dead = true;
            }
        }
    }

    fn update_bazs(&mut self) {
        for i in 0..self.bazs.len() {
            let (x, y) = (self.bazs[i].x, self.bazs[i].y);
            let nbrs = neighbours(x, y, self.w, self.h, &self.grid);

            let drain = nbrs.len() as f32 * 0.5;
            self.bazs[i].energy -= drain;

            if self.bazs[i].energy <= 0.0 {
                self.bazs[i].dead = true;
            }
        }
    }

    fn update(&mut self) {
        self.update_foos();
        self.purge_foos();

        self.update_bars();
        self.purge_bars();

        self.update_bazs();
        self.purge_bazs();
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut game = Game::new(8, 8);

    game.foos.push(Foo { x: 1, y: 1, health: 100, dead: false });
    game.grid[1][1] = Some(CellRef::Foo(0));

    game.bars.push(Bar { x: 2, y: 1, speed: 3.0, dead: false });
    game.grid[1][2] = Some(CellRef::Bar(0));

    game.bazs.push(Baz { x: 1, y: 2, energy: 10.0, dead: false });
    game.grid[2][1] = Some(CellRef::Baz(0));

    println!("-- before --");
    game.draw();
    game.print_canvas();

    game.update();

    println!("-- after --");
    game.draw();
    game.print_canvas();
}
