#[allow(dead_code)]
mod util;

use crate::util::event::Config;
use nalgebra::{DMatrix, Vector2};
use rand::seq::IteratorRandom;
use std::cmp::max;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::iter;
use std::ops::Index;
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
    SpawnPoint,
    Destination,
}

impl Square {
    fn to_char(&self) -> char {
        match self {
            Square::Empty => ' ',
            Square::Wall => '#',
            Square::SpawnPoint => '^',
            Square::Destination => '$',
        }
    }
    fn fr_char(c: char) -> Self {
        match c {
            ' ' => Square::Empty,
            '#' => Square::Wall,
            '^' => Square::SpawnPoint,
            '$' => Square::Destination,
            _ => panic!(),
        }
    }
}

struct Map {
    grid: DMatrix<Square>,
}

const NEIGHBOR4: [Vector2<i32>; 4] = [
    Vector2::new(0, 1),
    Vector2::new(1, 0),
    Vector2::new(-1, 0),
    Vector2::new(0, -1),
];

const NEIGHBOR8: [Vector2<i32>; 8] = [
    Vector2::new(0, 1),
    Vector2::new(1, 0),
    Vector2::new(0, -1),
    Vector2::new(-1, 0),
    Vector2::new(1, 1),
    Vector2::new(1, -1),
    Vector2::new(-1, -1),
    Vector2::new(-1, -1),
];

impl Map {
    fn new(desc: &str) -> Self {
        let lines = desc
            .split('\n')
            .filter(|l| l.len() > 0)
            .map(|l| l.chars().map(Square::fr_char));
        let (w, h) = lines
            .clone()
            .fold((0, 0), |(w, h), l| (max(w, l.count()), h + 1));
        let lines = lines.map(|l| {
            l.clone()
                .chain(iter::repeat(Square::Empty).take(w - l.count()))
        });
        Map {
            grid: DMatrix::from_iterator(w, h, lines.flatten()).transpose(),
        }
    }
    fn in_bounds(&self, s: Vector2<i32>) -> bool {
        s.x >= 0 && s.y >= 0 && s.x < self.grid.ncols() as i32 && s.y < self.grid.nrows() as i32
    }
    fn neighbors_offsets<'a>(
        &'a self,
        s: Vector2<usize>,
        offsets: &'a [Vector2<i32>],
    ) -> impl Iterator<Item = Vector2<usize>> + 'a {
        let s = s.map(|x| x as i32);
        offsets
            .iter()
            .map(move |t| s + t)
            .filter(move |t| self.in_bounds(*t))
            .map(|t| t.map(|x| x as usize))
    }
    fn neighbors_4(&self, s: Vector2<usize>) -> impl Iterator<Item = Vector2<usize>> + '_ {
        self.neighbors_offsets(s, &NEIGHBOR4)
    }
    fn neighbors_8(&self, s: Vector2<usize>) -> impl Iterator<Item = Vector2<usize>> + '_ {
        self.neighbors_offsets(s, &NEIGHBOR8)
    }
}

impl Index<Vector2<usize>> for Map {
    type Output = Square;

    fn index<'a>(&'a self, i: Vector2<usize>) -> &'a Square {
        &self.grid.index((i.y, i.x))
    }
}

impl Widget for &Map {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        for (y, row) in self.grid.row_iter().enumerate() {
            for (x, sq) in row.iter().enumerate() {
                let c = buf.get_mut(x as u16, y as u16);
                c.set_char(sq.to_char());
            }
        }
    }
}

struct GameState {
    enemies: Vec<Vector2<usize>>,
    map: Map,
}

impl GameState {
    fn advance(&mut self) {
        for enemy in self.enemies.iter_mut() {
            *enemy = pf_search(&self.map, *enemy);
        }
    }
}

impl Widget for &GameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.map.render(area, buf);
        for enemy in self.enemies.iter() {
            let c = buf.get_mut(enemy.x as u16, enemy.y as u16);
            c.set_symbol("*");
        }
    }
}

fn pf_random(m: &Map, s: Vector2<usize>) -> Vector2<usize> {
    let mut rng = rand::thread_rng();
    m.neighbors_4(s)
        .filter(|t| m[*t] == Square::Empty)
        .choose(&mut rng)
        .unwrap_or(s)
}

fn first_move(
    parents: &HashMap<Vector2<usize>, Option<Vector2<usize>>>,
    end: Vector2<usize>,
) -> Vector2<usize> {
    let mut cur = end;
    let mut prev = end;
    while let Some(&Some(parent)) = parents.get(&cur) {
        prev = cur;
        cur = parent;
    }
    prev
}

fn pf_search(m: &Map, s: Vector2<usize>) -> Vector2<usize> {
    let mut parents = HashMap::new();
    let mut q = VecDeque::new();
    let mut cur = s;
    let mut parent: Option<Vector2<usize>> = None;
    while m[cur] != Square::Destination {
        parents.insert(cur, parent);
        q.extend(
            m.neighbors_4(cur)
                .filter(|t| {
                    (m[*t] == Square::Empty || m[*t] == Square::Destination)
                        && !parents.contains_key(t)
                })
                .map(|t| (t, Some(cur))),
        );
        let next = q.pop_front().unwrap();
        cur = next.0;
        parent = next.1;
    }
    parents.insert(cur, parent);
    first_move(&parents, cur)
}

static MAP: &str = r#"
### #############
### #############
### #############
###         ###########
###### #### ###########
###### #### #########
######$####$###########
"#;

fn main() -> Result<(), Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game_state = GameState {
        enemies: vec![Vector2::new(3, 0), Vector2::new(3, 2)],
        map: Map::new(MAP),
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
