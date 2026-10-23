use rand::seq::SliceRandom;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Stylize};
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Painter, Shape};
use ratatui::{DefaultTerminal, Frame};
use std::f64::consts::TAU;
use std::time::{Duration, Instant};

const N: usize = 4;
const SWAP_TIME: Duration = Duration::from_millis(250);
const FRAME_TIME: Duration = Duration::from_millis(16);

/// Two vertices that just traded slots, animating toward their new positions.
struct Swap {
    a: usize,
    b: usize,
    started: Instant,
}

struct Game {
    /// slot[v] = the position on the circle where vertex v sits
    slot: Vec<usize>,
    /// each edge is a pair of vertices
    edges: Vec<(usize, usize)>,
    /// the triangles of the graph as vertex triples, stacked bottom to top like sheets of paper
    flaps: Vec<[usize; 3]>,
    /// the slot the cursor is on
    cursor: usize,
    /// the slot picked as the first half of a swap
    picked: Option<usize>,
    moves: usize,
    swap: Option<Swap>,
}

impl Game {
    fn new(n: usize) -> Self {
        let mut candidates = Vec::new();
        for a in 0..n {
            for b in a + 1..n {
                candidates.push((a, b));
            }
        }
        candidates.shuffle(&mut rand::rng());

        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (a, b) in candidates {
            if !edges.iter().any(|&e| crosses(e, (a, b))) {
                edges.push((a, b))
            }
        }

        // the graph is a triangulation, so every three mutually connected vertices form a face
        let has = |a: usize, b: usize| edges.contains(&(a, b));
        let mut flaps = Vec::new();
        for a in 0..n {
            for b in a + 1..n {
                for c in b + 1..n {
                    if has(a, b) && has(b, c) && has(a, c) {
                        flaps.push([a, b, c]);
                    }
                }
            }
        }
        flaps.shuffle(&mut rand::rng());

        // the identity arrangement has no crossings, so this always shuffles at least once
        let mut game = Game {
            slot: (0..n).collect(),
            edges,
            flaps,
            cursor: 0,
            picked: None,
            moves: 0,
            swap: None,
        };
        while game.crossings() == 0 {
            game.slot.shuffle(&mut rand::rng());
        }
        game
    }

    fn crossings(&self) -> usize {
        let mut count = 0;
        for i in 0..self.edges.len() {
            for j in i + 1..self.edges.len() {
                let (a, b) = self.edges[i];
                let (c, d) = self.edges[j];
                if crosses((self.slot[a], self.slot[b]), (self.slot[c], self.slot[d])) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Moves the cursor one slot around the circle: +1 is counterclockwise, -1 clockwise.
    fn step_cursor(&mut self, dir: isize) {
        let n = self.slot.len() as isize;
        self.cursor = (self.cursor as isize + dir).rem_euclid(n) as usize;
    }

    /// First press picks the cursor's slot, second press swaps it with the picked one.
    fn select(&mut self) {
        match self.picked {
            None => self.picked = Some(self.cursor),
            Some(s) if s == self.cursor => self.picked = None,
            Some(s) => {
                let a = self.vertex_at(s);
                let b = self.vertex_at(self.cursor);
                self.slot.swap(a, b);
                self.picked = None;
                self.moves += 1;
                self.swap = Some(Swap { a, b, started: Instant::now() });
            }
        }
    }

    /// Where vertex v is drawn: its slot, or partway there if it's mid-swap.
    fn vertex_pos(&self, v: usize) -> (f64, f64) {
        let n = self.slot.len();
        let to = pos(self.slot[v], n);
        let Some(swap) = &self.swap else { return to };
        // a swapped vertex starts where its partner has just landed
        let partner = if v == swap.a {
            swap.b
        } else if v == swap.b {
            swap.a
        } else {
            return to;
        };
        let from = pos(self.slot[partner], n);
        let t = (swap.started.elapsed().as_secs_f64() / SWAP_TIME.as_secs_f64()).min(1.0);
        let t = t * t * (3.0 - 2.0 * t); // ease in and out
        (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
    }

    fn animating(&self) -> bool {
        self.swap.is_some()
    }

    fn vertex_at(&self, s: usize) -> usize {
        self.slot.iter().position(|&x| x == s).unwrap()
    }
}

/// Edges are given as circle slots. Two chords cross exactly when one endpoint
/// of the second lies strictly between the endpoints of the first and the other doesn't.
fn crosses((a, b): (usize, usize), (c, d): (usize, usize)) -> bool {
    if a == c || a == d || b == c || b == d {
        return false; // sharing an endpoint isn't a crossing
    }
    let (lo, hi) = (a.min(b), a.max(b));
    let inside = |v: usize| lo < v && v < hi;
    inside(c) != inside(d)
}

type Point = (f64, f64);

/// Edges drawn as if each triangle were an opaque sheet of paper:
/// the parts of an edge that lie under a higher flap are skipped.
struct Flaps {
    /// triangle corners, bottom to top
    tris: Vec<[Point; 3]>,
    /// edge endpoints, plus the index of the highest flap the edge belongs to
    edges: Vec<(Point, Point, usize)>,
    color: Color,
}

impl Shape for Flaps {
    fn draw(&self, painter: &mut Painter) {
        for &(p, q, z) in &self.edges {
            // one sample per braille dot along the longer axis
            let steps = match (painter.get_point(p.0, p.1), painter.get_point(q.0, q.1)) {
                (Some(a), Some(b)) => a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)).max(1),
                _ => 100,
            };
            for i in 0..=steps {
                let t = i as f64 / steps as f64;
                let pt = (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
                let covered = self.tris[z + 1..].iter().any(|tri| inside(tri, pt));
                if !covered && let Some((x, y)) = painter.get_point(pt.0, pt.1) {
                    painter.paint(x, y, self.color);
                }
            }
        }
    }
}

/// Whether p is strictly inside the triangle, so points on its border don't count.
fn inside(&[a, b, c]: &[Point; 3], p: Point) -> bool {
    let side = |u: Point, v: Point| (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    let eps = 1e-9;
    (d1 > eps && d2 > eps && d3 > eps) || (d1 < -eps && d2 < -eps && d3 < -eps)
}

/// Canvas coordinates of a slot on the unit circle.
fn pos(slot: usize, n: usize) -> (f64, f64) {
    let angle = TAU * slot as f64 / n as f64;
    (angle.cos(), angle.sin())
}

fn main() -> std::io::Result<()> {
    ratatui::run(app)
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut game = Game::new(N);
    loop {
        if game.swap.as_ref().is_some_and(|s| s.started.elapsed() >= SWAP_TIME) {
            game.swap = None;
        }
        terminal.draw(|frame| render(frame, &game))?;
        // while animating, wake up every frame to redraw; otherwise sleep until a key
        if game.animating() && !event::poll(FRAME_TIME)? {
            continue;
        }
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Left | KeyCode::Char('h') => game.step_cursor(1),
                KeyCode::Right | KeyCode::Char('l') => game.step_cursor(-1),
                KeyCode::Enter | KeyCode::Char(' ') if game.crossings() == 0 => game = Game::new(game.slot.len() + 1),
                KeyCode::Char(' ') | KeyCode::Enter => game.select(),
                _ => {}
            }
        }
    }
}

fn render(frame: &mut Frame, game: &Game) {
    let [board, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let n = game.slot.len();
    let crossings = game.crossings();
    let edge_color = if crossings == 0 { Color::Green } else { Color::Gray };

    // terminal cells are ~twice as tall as wide, so widen the x range to keep the circle round
    let aspect = board.width as f64 / (board.height as f64 * 2.0);
    let pad = 1.2;
    let flaps = Flaps {
        tris: game.flaps.iter().map(|f| f.map(|v| game.vertex_pos(v))).collect(),
        edges: game
            .edges
            .iter()
            .map(|&(a, b)| {
                let z = game.flaps.iter().rposition(|f| f.contains(&a) && f.contains(&b));
                (game.vertex_pos(a), game.vertex_pos(b), z.unwrap_or(0))
            })
            .collect(),
        color: edge_color,
    };
    let canvas = Canvas::default()
        .x_bounds([-pad * aspect, pad * aspect])
        .y_bounds([-pad, pad])
        .paint(|ctx| {
            ctx.draw(&flaps);
            for v in 0..n {
                let s = game.slot[v];
                let (x, y) = game.vertex_pos(v);
                let dot = if game.picked == Some(s) {
                    "◉".yellow().bold()
                } else if game.cursor == s {
                    "○".cyan().bold()
                } else {
                    Span::raw("●")
                };
                ctx.print(x, y, dot);
            }
        });
    frame.render_widget(canvas, board);

    let text: Span = if crossings == 0 {
        format!(" solved in {} moves! enter: next level · q quit", game.moves).green().bold()
    } else {
        format!(
            " crossings: {crossings}   moves: {}   ←/→ move · space swap · q quit",
            game.moves
        )
        .into()
    };
    frame.render_widget(text, status);
}
