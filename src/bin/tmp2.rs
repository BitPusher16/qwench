use std::io;
use std::time::Duration;
use std::time::Instant;

use crossterm::{execute, queue};
use crossterm::cursor::MoveTo;
use crossterm::style::{Print, Color, SetForegroundColor, ResetColor};
use crossterm::terminal::{Clear, ClearType};
use std::io::{Write, Result};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

fn main() -> Result<()> {
    enable_raw_mode()?;

    // initialize the render grid.
    //const GRID_W: usize = 24;
    //const GRID_H: usize = 24;
    //let mut grid = [['.'; GRID_W]; GRID_H];

    let mut buffer: Vec<(char, Option<Color>)> = vec![];

    buffer.push(('H', Some(Color::Red)));
    buffer.push(('e', None));
    buffer.push(('l', None));
    buffer.push(('l', None));
    buffer.push(('o', Some(Color::Green)));
    buffer.push((' ', None));
    buffer.push(('W', Some(Color::Blue)));
    buffer.push(('o', None));
    buffer.push(('r', Some(Color::Yellow)));
    buffer.push(('l', None));
    buffer.push(('d', Some(Color::Magenta)));
    buffer.push(('!', None));
    buffer.push(('\r', None));
    buffer.push(('\n', None));

    let mut stdout = io::stdout();
    let tick_ms = Duration::from_millis(200);
    let time_beg = Instant::now();
    let mut quit = false;
    let mut remaining_ms = tick_ms;
    while !quit {
        if event::poll(remaining_ms)? {
            if let Event::Key(event) = event::read()? {
                // \r is required because in raw mode the cursor doesn't automatically return.
                print!("Key pressed: {:?}\r\n", event.code);
                if event.code == KeyCode::Char('q') {
                    quit = true;
                }
            }
            // as_millis() returns a u128, but from_millis() requires a u64.
            let rem = tick_ms.as_millis() - (time_beg.elapsed().as_millis() % tick_ms.as_millis());
            remaining_ms = Duration::from_millis(rem as u64);
        }
        else{
            print!("drawing...\r\n");

            execute!(stdout, Clear(ClearType::All), MoveTo(0,0))?;
            
            for (ch, maybe_color) in &buffer {
                if let Some(color) = maybe_color {
                    queue!(
                        stdout,
                        SetForegroundColor(*color),
                        Print(ch),
                        ResetColor
                    )?;
                } else {
                    queue!(stdout, Print(ch))?;
                }
            }

            stdout.flush()?;

            let rem = tick_ms.as_millis() - (time_beg.elapsed().as_millis() % tick_ms.as_millis());
            remaining_ms = Duration::from_millis(rem as u64);
        }
    }

    disable_raw_mode()?;
    Ok(())
}
