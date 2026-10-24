use rand::RngExt;
use rand::seq::SliceRandom;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Stylize};
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Painter, Shape};
use ratatui::{DefaultTerminal, Frame};
use std::f64::consts::TAU;
use std::time::{Duration, Instant};

/// a muted red (256-color palette) for the bits of edges right around a crossing
const CROSSED_COLOR: Color = Color::Indexed(174);
/// how far the red reaches from a crossing, in braille dots (2 per cell across, 4 down)
const CROSSED_RADIUS: usize = 4;
/// dots on level 1; each level adds one
const N: usize = 4;
const SWAP_TIME: Duration = Duration::from_millis(250);
/// how long the cursor stays solid after a keypress before it starts blinking
const BLINK_DELAY: Duration = Duration::from_millis(800);
/// how long each hollow or filled phase of the blink lasts
const BLINK_TIME: Duration = Duration::from_millis(900);
const FRAME_TIME: Duration = Duration::from_millis(16);
const UNFOLD_TIME: Duration = Duration::from_millis(1200);
/// chance that a new flap grows off the one just added, which makes snakes and spirals instead of blobs
const STRIP_BIAS: f64 = 0.6;

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
    /// where each vertex sits in the flat sheet the flaps were grown as
    shape: Vec<Point>,
    /// the shape laid over the solved circle, which the vertices glide to on a win
    unfolded: Vec<Point>,
    /// the slot the cursor is on
    cursor: usize,
    /// the slot picked as the first half of a swap
    picked: Option<usize>,
    moves: usize,
    swap: Option<Swap>,
    /// when the winning swap was made
    solved_at: Option<Instant>,
    /// when the cursor last moved or picked, which keeps it solid for a moment
    touched_at: Instant,
}

impl Game {
    fn new(n: usize) -> Self {
        let (shape, mut flaps) = grow(n);
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for &[a, b, c] in &flaps {
            for (u, v) in [(a, b), (b, c), (a, c)] {
                let e = (u.min(v), u.max(v));
                if !edges.contains(&e) {
                    edges.push(e);
                }
            }
        }
        flaps.shuffle(&mut rand::rng());

        // shuffle until there's something to untangle
        let mut game = Game {
            slot: (0..n).collect(),
            edges,
            flaps,
            shape,
            unfolded: Vec::new(),
            cursor: 0,
            picked: None,
            moves: 0,
            swap: None,
            solved_at: None,
            touched_at: Instant::now(),
        };
        while game.crossings() == 0 {
            game.slot.shuffle(&mut rand::rng());
        }
        game
    }

    /// Every pair of edges (by index) that cross in the current arrangement.
    fn crossing_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for i in 0..self.edges.len() {
            for j in i + 1..self.edges.len() {
                let (a, b) = self.edges[i];
                let (c, d) = self.edges[j];
                if crosses((self.slot[a], self.slot[b]), (self.slot[c], self.slot[d])) {
                    pairs.push((i, j));
                }
            }
        }
        pairs
    }

    fn crossings(&self) -> usize {
        self.crossing_pairs().len()
    }

    /// Moves the cursor one slot around the circle: +1 is counterclockwise, -1 clockwise.
    fn step_cursor(&mut self, dir: isize) {
        let n = self.slot.len() as isize;
        self.cursor = (self.cursor as isize + dir).rem_euclid(n) as usize;
        self.touched_at = Instant::now();
    }

    /// First press picks the cursor's slot, second press swaps it with the picked one.
    fn select(&mut self) {
        self.touched_at = Instant::now();
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
                self.solved_at = (self.crossings() == 0).then(Instant::now);
                if self.solved_at.is_some() {
                    self.unfolded = self.unfold_target();
                }
            }
        }
    }

    /// 0..1 progress of a phase that starts `delay` after the winning swap and lasts `length`.
    fn since_win(&self, delay: Duration, length: Duration) -> f64 {
        let Some(at) = self.solved_at else { return 0.0 };
        let t = at.elapsed().saturating_sub(delay).as_secs_f64() / length.as_secs_f64();
        t.min(1.0)
    }

    /// 0..1 progress of the vertices gliding from the circle to the real shape, once the winning swap has landed.
    fn unfold(&self) -> f64 {
        self.since_win(SWAP_TIME, UNFOLD_TIME)
    }

    /// 0..1 progress of the creases dissolving, which happens while the shape unfolds.
    fn fade(&self) -> f64 {
        self.unfold()
    }

    /// Where vertex v is drawn: on the circle while playing, then gliding out to the real shape.
    fn vertex_pos(&self, v: usize) -> Point {
        let from = self.circle_pos(v);
        let t = self.unfold();
        if t == 0.0 {
            return from;
        }
        lerp(from, self.unfolded[v], ease(t))
    }

    /// Where vertex v sits on the circle: its slot, or partway there if it's mid-swap.
    fn circle_pos(&self, v: usize) -> Point {
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
        lerp(from, to, ease(t))
    }

    /// The shape turned (and mirrored, if the player's outline runs the other way)
    /// to best match the solved circle, then centered and scaled to fit the board.
    fn unfold_target(&self) -> Vec<Point> {
        let n = self.shape.len();
        let (mx, my) = self.shape.iter().fold((0.0, 0.0), |s, p| (s.0 + p.0 / n as f64, s.1 + p.1 / n as f64));
        // treating points as complex numbers, the best rotation is the angle of sum(circle * conj(shape))
        let fit = |flip: f64| {
            let pts: Vec<Point> = self.shape.iter().map(|p| ((p.0 - mx) * flip, p.1 - my)).collect();
            let (re, im) = pts.iter().enumerate().fold((0.0, 0.0), |(re, im), (v, s)| {
                let c = pos(self.slot[v], n);
                (re + c.0 * s.0 + c.1 * s.1, im + c.1 * s.0 - c.0 * s.1)
            });
            (re.hypot(im), pts, im.atan2(re))
        };
        let (a, b) = (fit(1.0), fit(-1.0));
        let (_, pts, angle) = if a.0 >= b.0 { a } else { b };
        let (sin, cos) = angle.sin_cos();
        let pts: Vec<Point> = pts.iter().map(|p| (p.0 * cos - p.1 * sin, p.0 * sin + p.1 * cos)).collect();
        let (lo, hi) = pts.iter().fold(((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)), |(lo, hi), p| {
            ((lo.0.min(p.0), lo.1.min(p.1)), (hi.0.max(p.0), hi.1.max(p.1)))
        });
        let center = ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0);
        let radius = pts.iter().map(|p| (p.0 - center.0).hypot(p.1 - center.1)).fold(0.0, f64::max);
        pts.iter().map(|p| ((p.0 - center.0) / radius, (p.1 - center.1) / radius)).collect()
    }

    /// Whether the cursor is drawn filled: always right after a keypress, then alternating with hollow.
    fn cursor_filled(&self) -> bool {
        let Some(idle) = self.touched_at.elapsed().checked_sub(BLINK_DELAY) else { return true };
        (idle.as_millis() / BLINK_TIME.as_millis()) % 2 == 1
    }

    /// How long until the cursor next flips between filled and hollow.
    fn until_blink(&self) -> Duration {
        let elapsed = self.touched_at.elapsed();
        match elapsed.checked_sub(BLINK_DELAY) {
            None => BLINK_DELAY - elapsed,
            Some(idle) => BLINK_TIME - Duration::from_nanos((idle.as_nanos() % BLINK_TIME.as_nanos()) as u64),
        }
    }

    fn animating(&self) -> bool {
        self.swap.is_some() || (self.solved_at.is_some() && self.fade() < 1.0)
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

/// Grows a flat sheet of triangles: each new vertex folds out from an edge on the outline,
/// so the result is a triangulated polygon of any outline, not just a convex one.
fn grow(n: usize) -> (Vec<Point>, Vec<[usize; 3]>) {
    let mut rng = rand::rng();
    'restart: loop {
        let mut pts: Vec<Point> = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 3f64.sqrt() / 2.0)];
        let mut tris = vec![[0, 1, 2]];
        while pts.len() < n {
            // outline edges belong to exactly one flap; keep the flap's third corner to know which side is outside
            let mut outline = Vec::new();
            for (i, &[a, b, c]) in tris.iter().enumerate() {
                for (u, v, w) in [(a, b, c), (b, c, a), (c, a, b)] {
                    let shared = tris.iter().any(|t| t != &tris[i] && t.contains(&u) && t.contains(&v));
                    if !shared {
                        outline.push((u, v, w, i == tris.len() - 1));
                    }
                }
            }
            let fresh: Vec<_> = outline.iter().copied().filter(|e| e.3).collect();
            let placed = (0..200).find_map(|_| {
                let pool = if rng.random::<f64>() < STRIP_BIAS { &fresh } else { &outline };
                let (a, b, c, _) = pool[rng.random_range(0..pool.len())];
                let (p, q, o) = (pts[a], pts[b], pts[c]);
                let d = (q.0 - p.0, q.1 - p.1);
                let len = d.0.hypot(d.1);
                let mid = ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
                let mut normal = (-d.1 / len, d.0 / len);
                if normal.0 * (o.0 - mid.0) + normal.1 * (o.1 - mid.1) > 0.0 {
                    normal = (-normal.0, -normal.1);
                }
                let height = len * 3f64.sqrt() / 2.0 * rng.random_range(0.7..1.3);
                let slide = rng.random_range(-0.25..0.25);
                let apex = (
                    mid.0 + normal.0 * height + d.0 * slide,
                    mid.1 + normal.1 * height + d.1 * slide,
                );
                fits(&pts, &tris, apex, p, q).then_some((apex, a, b))
            });
            let Some((apex, a, b)) = placed else { continue 'restart };
            pts.push(apex);
            tris.push([a, b, pts.len() - 1]);
        }
        return (pts, tris);
    }
}

/// Whether a new flap with corners apex, p, q lies flat on the sheet without overlapping it or making slivers.
fn fits(pts: &[Point], tris: &[[usize; 3]], apex: Point, p: Point, q: Point) -> bool {
    let dist = |a: Point, b: Point| (a.0 - b.0).hypot(a.1 - b.1);
    if ![p, q].iter().all(|&e| (0.6..1.5).contains(&dist(apex, e))) {
        return false;
    }
    let new_tri = [apex, p, q];
    if pts.iter().any(|&v| dist(v, apex) < 0.5 || inside(&new_tri, v)) {
        return false;
    }
    tris.iter().all(|t| {
        let corners = t.map(|i| pts[i]);
        !inside(&corners, apex)
            && (0..3).all(|k| {
                let (u, w) = (corners[k], corners[(k + 1) % 3]);
                [p, q].iter().all(|&e| {
                    let touching = e == u || e == w;
                    touching || !segments_cross((apex, e), (u, w))
                }) && (u == p || u == q || segment_dist(u, (apex, p)).min(segment_dist(u, (apex, q))) > 0.25)
                    && segment_dist(apex, (u, w)) > 0.25
            })
    })
}

/// Whether two segments properly cross each other.
fn segments_cross((a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    let side = |u: Point, v: Point, p: Point| (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0);
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

/// Where two segments cross, if they do.
fn intersection((a, b): (Point, Point), (c, d): (Point, Point)) -> Option<Point> {
    let (r, s) = ((b.0 - a.0, b.1 - a.1), (d.0 - c.0, d.1 - c.1));
    let denom = r.0 * s.1 - r.1 * s.0;
    if denom == 0.0 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / denom;
    let u = ((c.0 - a.0) * r.1 - (c.1 - a.1) * r.0) / denom;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| lerp(a, b, t))
}

/// Distance from p to the nearest point of segment ab.
fn segment_dist(p: Point, (a, b): (Point, Point)) -> f64 {
    let d = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * d.0 + (p.1 - a.1) * d.1) / (d.0 * d.0 + d.1 * d.1)).clamp(0.0, 1.0);
    (a.0 + d.0 * t - p.0).hypot(a.1 + d.1 * t - p.1)
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Eases in and out.
fn ease(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}


/// Edges drawn as if each triangle were an opaque sheet of paper:
/// the parts of an edge that lie under a higher flap are skipped.
struct Flaps {
    /// triangle corners, bottom to top
    tris: Vec<[Point; 3]>,
    edges: Vec<Crease>,
    /// 0..1, how much of the internal creases has dissolved after a win
    fade: f64,
    /// the creases are shadows cast onto the table and lower flaps, so their own flap hides them too
    shadow: bool,
    /// where edges cross, in canvas coordinates; creases are tinted red around these
    crossings: Vec<Point>,
}

#[derive(Clone, Copy)]
struct Crease {
    p: Point,
    q: Point,
    /// index of the highest flap the edge belongs to
    z: usize,
    /// shared by two flaps, as opposed to part of the outline
    internal: bool,
    color: Color,
}

impl Shape for Flaps {
    fn draw(&self, painter: &mut Painter) {
        let crossings: Vec<_> = self.crossings.iter().filter_map(|c| painter.get_point(c.0, c.1)).collect();
        // braille dots are about square on screen, so plain distance in dots looks round
        let near_crossing = |(x, y): (usize, usize)| {
            crossings.iter().any(|&(cx, cy)| {
                let (dx, dy) = (cx.abs_diff(x), cy.abs_diff(y));
                dx * dx + dy * dy <= CROSSED_RADIUS * CROSSED_RADIUS
            })
        };
        for (e, &Crease { p, q, z, internal, color }) in self.edges.iter().enumerate() {
            // one sample per braille dot along the longer axis
            let steps = match (painter.get_point(p.0, p.1), painter.get_point(q.0, q.1)) {
                (Some(a), Some(b)) => a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)).max(1),
                _ => 100,
            };
            for i in 0..=steps {
                // dissolve: each dot gets a fixed pseudo-random threshold and vanishes once fade passes it
                let threshold = (i.wrapping_mul(2654435761) ^ e.wrapping_mul(40503)) % 1000;
                if internal && (threshold as f64) < self.fade * 1000.0 {
                    continue;
                }
                let t = i as f64 / steps as f64;
                let pt = (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
                let above = if self.shadow { z } else { z + 1 };
                let covered = self.tris[above..].iter().any(|tri| inside(tri, pt));
                if !covered && let Some((x, y)) = painter.get_point(pt.0, pt.1) {
                    painter.paint(x, y, if near_crossing((x, y)) { CROSSED_COLOR } else { color });
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
    let level = match std::env::args().nth(1) {
        None => 1,
        Some(arg) => match arg.parse::<usize>() {
            Ok(level) if level >= 1 => level,
            _ => {
                eprintln!("usage: unfld [level]");
                std::process::exit(2);
            }
        },
    };
    ratatui::run(|terminal| app(terminal, level))
}

fn app(terminal: &mut DefaultTerminal, level: usize) -> std::io::Result<()> {
    let mut game = Game::new(level + N - 1);
    loop {
        if game.swap.as_ref().is_some_and(|s| s.started.elapsed() >= SWAP_TIME) {
            game.swap = None;
        }
        terminal.draw(|frame| render(frame, &game))?;
        // while animating, wake up every frame to redraw; otherwise sleep until a key or the next blink,
        // and once solved there's no cursor left to blink
        let wait = if game.animating() {
            Some(FRAME_TIME)
        } else if game.solved_at.is_none() {
            Some(game.until_blink())
        } else {
            None
        };
        if let Some(wait) = wait
            && !event::poll(wait)?
        {
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
                let owns = |f: &[usize; 3]| f.contains(&a) && f.contains(&b);
                Crease {
                    p: game.vertex_pos(a),
                    q: game.vertex_pos(b),
                    z: game.flaps.iter().rposition(owns).unwrap_or(0),
                    internal: game.flaps.iter().filter(|f| owns(f)).count() == 2,
                    color: edge_color,
                }
            })
            .collect(),
        fade: game.fade(),
        shadow: false,
        crossings: game
            .crossing_pairs()
            .into_iter()
            .filter_map(|(i, j)| {
                let seg = |(a, b): (usize, usize)| (game.vertex_pos(a), game.vertex_pos(b));
                intersection(seg(game.edges[i]), seg(game.edges[j]))
            })
            .collect(),
    };
    // each flap's free edges cast a shadow one whole cell down and right, onto the table and any
    // flaps beneath it; a cell holds one color, so a smaller offset would mostly land in cells the
    // creases already own and get painted over. A crease shared by two flaps joins them into one
    // sheet rather than lying on top, so it casts nothing.
    let cell = (2.0 * pad * aspect / board.width as f64, 2.0 * pad / board.height as f64);
    let shadow = Flaps {
        tris: flaps.tris.clone(),
        edges: flaps
            .edges
            .iter()
            .filter(|c| !c.internal)
            .map(|&c| Crease {
                p: (c.p.0 + cell.0, c.p.1 - cell.1),
                q: (c.q.0 + cell.0, c.q.1 - cell.1),
                color: Color::DarkGray,
                ..c
            })
            .collect(),
        fade: 0.0,
        shadow: true,
        crossings: Vec::new(),
    };
    let canvas = Canvas::default()
        .x_bounds([-pad * aspect, pad * aspect])
        .y_bounds([-pad, pad])
        .paint(|ctx| {
            ctx.draw(&shadow);
            ctx.layer();
            ctx.draw(&flaps);
            // the cursor goes away once the puzzle is solved
            for v in (0..n).filter(|_| game.solved_at.is_none()) {
                let s = game.slot[v];
                let (x, y) = game.vertex_pos(v);
                // plain vertices are just where edges meet; only the cursor and pick get a marker
                let color = if game.picked == Some(s) { Color::Yellow } else { Color::Cyan };
                if game.cursor == s {
                    let glyph = if game.cursor_filled() { "●" } else { "○" };
                    ctx.print(x, y, glyph.fg(color).bold());
                } else if game.picked == Some(s) {
                    ctx.print(x, y, "●".fg(color).bold());
                }
            }
        });
    frame.render_widget(canvas, board);

    let level = n + 1 - N;
    let text: Span = if crossings == 0 {
        format!(" level {level} solved in {} moves! enter: next level · q quit", game.moves).green().bold()
    } else {
        format!(
            " level {level}   crossings: {crossings}   moves: {}   ←/→ move · space swap · q quit",
            game.moves
        )
        .into()
    };
    frame.render_widget(text, status);
}
