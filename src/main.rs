#[allow(dead_code)]
mod util;

use crate::util::event::Config;
use nalgebra::{DMatrix, DVector, Vector2};
use std::time::Duration;
use std::{error::Error, io};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::widgets::Widget;
use tui::{backend::TermionBackend, Terminal};
use util::event::{Event, Events};

#[derive(Copy, Clone, Debug, PartialEq)]
enum Square {
    Empty,
    Wall,
}

struct Map {
    grid: DMatrix<Square>,
}

impl Map {
    fn new(desc: &str) -> Self {
        let lines = (desc.split('\n').map(|line| {
            line.chars()
                .map(|c| match c {
                    ' ' => Square::Empty,
                    '#' => Square::Wall,
                    _ => panic!(),
                })
                .collect::<Vec<Square>>()
        }))
        .collect::<Vec<Vec<Square>>>();
        let h = lines.len();
        let w = lines[0].len();
        eprintln!("w: {}, h: {}", w, h);
        Map {
            grid: DMatrix::from_iterator(w, h, lines.into_iter().flatten()).transpose(),
        }
    }
}

impl Widget for &Map {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        for (y, row) in self.grid.row_iter().enumerate() {
            for (x, sq) in row.iter().enumerate() {
                let c = buf.get_mut(x as u16, y as u16);
                c.set_symbol(match sq {
                    Square::Empty => " ",
                    Square::Wall => "#",
                });
            }
        }
    }
}

struct GameState {
    enemies: Vec<usize>,
    map: Map,
    path: Vec<Vector2<i32>>,
}

impl GameState {
    fn advance(&mut self) {
        let n = self.path.len();
        for enemy in self.enemies.iter_mut() {
            *enemy += 1;
            if *enemy >= n {
                *enemy = 0;
            }
        }
    }
}

impl Widget for &GameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.map.render(area, buf);
        for enemy in self.enemies.iter() {
            let pos = self.path[*enemy];
            let c = buf.get_mut(pos[0] as u16, pos[1] as u16);
            c.set_symbol("*");
        }
    }
}

static MAP: &str = r#"### ###################
### ###################
### ###################
###    ################
###### ################
###### ################
###### ################"#;

fn main() -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game_state = GameState {
        enemies: vec![0, 5],
        map: Map::new(MAP),
        path: vec![
            Vector2::new(3, 0),
            Vector2::new(3, 1),
            Vector2::new(3, 2),
            Vector2::new(3, 3),
            Vector2::new(4, 3),
            Vector2::new(5, 3),
            Vector2::new(6, 3),
            Vector2::new(6, 4),
            Vector2::new(6, 5),
            Vector2::new(6, 6),
        ],
    };

    // Setup event handlers
    let events = Events::with_config(Config {
        exit_key: Key::Char('q'),
        tick_rate: Duration::from_millis(1000),
    });

    loop {
        terminal.draw(|f| {
            f.render_widget(&game_state, f.size());
        })?;

        match events.next()? {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    break;
                }
                _ => {}
            },
            Event::Tick => {
                game_state.advance();
            }
        }
    }

    Ok(())
}
