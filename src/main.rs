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

/// dots on level 1; each level adds one
const LEVEL_1_DOTS: usize = 4;

/// how long two swapped dots take to trade places
const SWAP_TIME: Duration = Duration::from_millis(250);
/// how long the solved dots take to glide out to the real shape, while the creases dissolve
const UNFOLD_TIME: Duration = Duration::from_millis(1200);
/// how long the cursor stays solid after a keypress before it starts blinking
const BLINK_DELAY: Duration = Duration::from_millis(800);
/// how long each hollow or filled phase of the blink lasts
const BLINK_TIME: Duration = Duration::from_millis(900);
/// redraw interval while something is animating
const FRAME_TIME: Duration = Duration::from_millis(16);

/// chance that a new flap grows off the one just added, which makes snakes and spirals instead of blobs
const STRIP_BIAS: f64 = 0.6;

/// a muted red (256-color palette) for the bits of edges right around a crossing
const CROSSED_COLOR: Color = Color::Indexed(174);
/// how far the red reaches from a crossing, in braille dots (2 per cell across, 4 down)
const CROSSED_RADIUS: usize = 4;

// ---------------------------------------------------------------------------------------------
// Game state

/// Two vertices that just traded slots, animating toward their new positions.
struct Swap {
    a: usize,
    b: usize,
    started: Instant,
}

/// An edge of the graph, which is also a crease between flaps.
struct Edge {
    a: usize,
    b: usize,
    /// index of the highest flap the edge belongs to
    z: usize,
    /// shared by two flaps, as opposed to part of the outline
    internal: bool,
}

struct Game {
    /// slot[v] = the position on the circle where vertex v sits
    slot: Vec<usize>,
    edges: Vec<Edge>,
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
    fn new(level: usize) -> Self {
        let n = level + LEVEL_1_DOTS - 1;
        let (shape, mut flaps) = grow(n);
        flaps.shuffle(&mut rand::rng());

        let mut edges: Vec<Edge> = Vec::new();
        for &[a, b, c] in &flaps {
            for (u, v) in [(a, b), (b, c), (a, c)] {
                let (a, b) = (u.min(v), u.max(v));
                if edges.iter().any(|e| (e.a, e.b) == (a, b)) {
                    continue;
                }
                let owns = |f: &[usize; 3]| f.contains(&a) && f.contains(&b);
                edges.push(Edge {
                    a,
                    b,
                    z: flaps.iter().rposition(owns).unwrap(),
                    internal: flaps.iter().filter(|f| owns(f)).count() == 2,
                });
            }
        }

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
        // shuffle until there's something to untangle
        while game.crossings() == 0 {
            game.slot.shuffle(&mut rand::rng());
        }
        game
    }

    fn level(&self) -> usize {
        self.slot.len() + 1 - LEVEL_1_DOTS
    }

    fn solved(&self) -> bool {
        self.solved_at.is_some()
    }

    /// Every pair of edges (by index) that cross in the current arrangement.
    fn crossing_pairs(&self) -> Vec<(usize, usize)> {
        let slots = |e: &Edge| (self.slot[e.a], self.slot[e.b]);
        let mut pairs = Vec::new();
        for i in 0..self.edges.len() {
            for j in i + 1..self.edges.len() {
                if crosses(slots(&self.edges[i]), slots(&self.edges[j])) {
                    pairs.push((i, j));
                }
            }
        }
        pairs
    }

    fn crossings(&self) -> usize {
        self.crossing_pairs().len()
    }

    fn vertex_at(&self, s: usize) -> usize {
        self.slot.iter().position(|&x| x == s).unwrap()
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
                if self.crossings() == 0 {
                    self.solved_at = Some(Instant::now());
                    self.unfolded = self.unfold_target();
                }
            }
        }
    }

    /// Drops the swap animation once it has finished.
    fn tick(&mut self) {
        if self.swap.as_ref().is_some_and(|s| s.started.elapsed() >= SWAP_TIME) {
            self.swap = None;
        }
    }

    /// 0..1 progress of the vertices gliding from the circle to the real shape and the creases
    /// dissolving, which starts once the winning swap has landed.
    fn unfold(&self) -> f64 {
        self.solved_at.map_or(0.0, |at| progress(at, SWAP_TIME, UNFOLD_TIME))
    }

    /// How long the main loop can sleep before it has to redraw, or None to wait for a key:
    /// a frame while animating, the next blink while playing, and forever once it's all settled.
    fn wake_after(&self) -> Option<Duration> {
        if self.swap.is_some() || (self.solved() && self.unfold() < 1.0) {
            Some(FRAME_TIME)
        } else if !self.solved() {
            Some(self.until_blink())
        } else {
            None
        }
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
        let to = slot_pos(self.slot[v], n);
        let Some(swap) = &self.swap else { return to };
        // a swapped vertex starts where its partner has just landed
        let partner = if v == swap.a {
            swap.b
        } else if v == swap.b {
            swap.a
        } else {
            return to;
        };
        let from = slot_pos(self.slot[partner], n);
        lerp(from, to, ease(progress(swap.started, Duration::ZERO, SWAP_TIME)))
    }

    /// The shape turned (and mirrored, if the player's outline runs the other way)
    /// to best match the solved circle, then centered and scaled to fit the board.
    fn unfold_target(&self) -> Vec<Point> {
        let n = self.shape.len();
        let mean = scale(self.shape.iter().fold((0.0, 0.0), |s, &p| add(s, p)), 1.0 / n as f64);
        // treating points as complex numbers, the best rotation is the angle of sum(circle * conj(shape))
        let fit = |flip: f64| {
            let pts: Vec<Point> = self.shape.iter().map(|&p| sub(p, mean)).map(|p| (p.0 * flip, p.1)).collect();
            let (re, im) = pts.iter().enumerate().fold((0.0, 0.0), |(re, im), (v, &s)| {
                let c = slot_pos(self.slot[v], n);
                (re + dot(c, s), im + cross(s, c))
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
        let center = lerp(lo, hi, 0.5);
        let radius = pts.iter().map(|&p| dist(p, center)).fold(0.0, f64::max);
        pts.iter().map(|&p| scale(sub(p, center), 1.0 / radius)).collect()
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
}

/// 0..1 progress of an animation lasting `length` that starts `delay` after `start`.
fn progress(start: Instant, delay: Duration, length: Duration) -> f64 {
    (start.elapsed().saturating_sub(delay).as_secs_f64() / length.as_secs_f64()).min(1.0)
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

// ---------------------------------------------------------------------------------------------
// Level generation

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
                let (p, q) = (pts[a], pts[b]);
                let d = sub(q, p);
                let mid = lerp(p, q, 0.5);
                let mut normal = scale((-d.1, d.0), 1.0 / dist(p, q));
                if dot(normal, sub(pts[c], mid)) > 0.0 {
                    normal = scale(normal, -1.0);
                }
                let height = dist(p, q) * 3f64.sqrt() / 2.0 * rng.random_range(0.7..1.3);
                let slide = rng.random_range(-0.25..0.25);
                let apex = add(add(mid, scale(normal, height)), scale(d, slide));
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
    // the least room kept between a corner and an edge it isn't on
    const GAP: f64 = 0.25;
    if ![p, q].iter().all(|&e| (0.6..1.5).contains(&dist(apex, e))) {
        return false;
    }
    if pts.iter().any(|&v| dist(v, apex) < 0.5 || inside(&[apex, p, q], v)) {
        return false;
    }
    tris.iter().all(|t| {
        let corners = t.map(|i| pts[i]);
        !inside(&corners, apex)
            && (0..3).all(|k| {
                let (u, w) = (corners[k], corners[(k + 1) % 3]);
                let cuts = [p, q].iter().any(|&e| e != u && e != w && segments_cross((apex, e), (u, w)));
                let crowds_new = u != p && u != q && [p, q].iter().any(|&e| segment_dist(u, (apex, e)) <= GAP);
                !cuts && !crowds_new && segment_dist(apex, (u, w)) > GAP
            })
    })
}

// ---------------------------------------------------------------------------------------------
// Geometry

type Point = (f64, f64);

fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}

fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

fn scale(a: Point, k: f64) -> Point {
    (a.0 * k, a.1 * k)
}

fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn cross(a: Point, b: Point) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn dist(a: Point, b: Point) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    add(a, scale(sub(b, a), t))
}

/// Eases in and out.
fn ease(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Which side of the line through u and v the point p is on: positive left, negative right.
fn side(u: Point, v: Point, p: Point) -> f64 {
    cross(sub(v, u), sub(p, u))
}

/// Canvas coordinates of a slot on the unit circle.
fn slot_pos(slot: usize, n: usize) -> Point {
    let angle = TAU * slot as f64 / n as f64;
    (angle.cos(), angle.sin())
}

/// Whether p is strictly inside the triangle, so points on its border don't count.
fn inside(&[a, b, c]: &[Point; 3], p: Point) -> bool {
    let (d1, d2, d3) = (side(a, b, p), side(b, c, p), side(c, a, p));
    let eps = 1e-9;
    (d1 > eps && d2 > eps && d3 > eps) || (d1 < -eps && d2 < -eps && d3 < -eps)
}

/// Whether two segments properly cross each other.
fn segments_cross((a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

/// Where two segments cross, if they do.
fn intersection((a, b): (Point, Point), (c, d): (Point, Point)) -> Option<Point> {
    let (r, s) = (sub(b, a), sub(d, c));
    let denom = cross(r, s);
    if denom == 0.0 {
        return None;
    }
    let t = cross(sub(c, a), s) / denom;
    let u = cross(sub(c, a), r) / denom;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| lerp(a, b, t))
}

/// Distance from p to the nearest point of segment ab.
fn segment_dist(p: Point, (a, b): (Point, Point)) -> f64 {
    let d = sub(b, a);
    let t = (dot(sub(p, a), d) / dot(d, d)).clamp(0.0, 1.0);
    dist(lerp(a, b, t), p)
}

// ---------------------------------------------------------------------------------------------
// Drawing

/// Edges drawn as if each triangle were an opaque sheet of paper:
/// the parts of an edge that lie under a higher flap are skipped.
struct Flaps {
    /// triangle corners, bottom to top
    tris: Vec<[Point; 3]>,
    creases: Vec<Crease>,
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

impl Flaps {
    /// The paper as it currently lies: gray while tangled, green once solved.
    fn paper(game: &Game) -> Self {
        let color = if game.solved() { Color::Green } else { Color::Gray };
        let seg = |e: &Edge| (game.vertex_pos(e.a), game.vertex_pos(e.b));
        Flaps {
            tris: game.flaps.iter().map(|f| f.map(|v| game.vertex_pos(v))).collect(),
            creases: game
                .edges
                .iter()
                .map(|e| {
                    let (p, q) = seg(e);
                    Crease { p, q, z: e.z, internal: e.internal, color }
                })
                .collect(),
            fade: game.unfold(),
            shadow: false,
            crossings: game
                .crossing_pairs()
                .into_iter()
                .filter_map(|(i, j)| intersection(seg(&game.edges[i]), seg(&game.edges[j])))
                .collect(),
        }
    }

    /// The shadow this paper casts, shifted by `offset`, onto the table and any flaps beneath.
    /// Only free edges cast one: a crease shared by two flaps joins them into one sheet rather
    /// than lying on top.
    fn shadow(&self, offset: Point) -> Self {
        Flaps {
            tris: self.tris.clone(),
            creases: self
                .creases
                .iter()
                .filter(|c| !c.internal)
                .map(|&c| Crease { p: add(c.p, offset), q: add(c.q, offset), color: Color::DarkGray, ..c })
                .collect(),
            fade: 0.0,
            shadow: true,
            crossings: Vec::new(),
        }
    }
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
        for (e, &Crease { p, q, z, internal, color }) in self.creases.iter().enumerate() {
            let above = if self.shadow { z } else { z + 1 };
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
                let pt = lerp(p, q, i as f64 / steps as f64);
                let covered = self.tris[above..].iter().any(|tri| inside(tri, pt));
                if !covered && let Some((x, y)) = painter.get_point(pt.0, pt.1) {
                    painter.paint(x, y, if near_crossing((x, y)) { CROSSED_COLOR } else { color });
                }
            }
        }
    }
}

fn render(frame: &mut Frame, game: &Game) {
    let [board, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // terminal cells are ~twice as tall as wide, so widen the x range to keep the circle round
    let aspect = board.width as f64 / (board.height as f64 * 2.0);
    let pad = 1.2;
    let paper = Flaps::paper(game);
    // the shadow sits one whole cell down and right: a cell holds one color, so a smaller
    // offset would mostly land in cells the creases already own and get painted over
    let shadow = paper.shadow((2.0 * pad * aspect / board.width as f64, -2.0 * pad / board.height as f64));
    let canvas = Canvas::default()
        .x_bounds([-pad * aspect, pad * aspect])
        .y_bounds([-pad, pad])
        .paint(|ctx| {
            ctx.draw(&shadow);
            ctx.layer();
            ctx.draw(&paper);
            // plain vertices are just where edges meet; only the cursor and pick get a marker,
            // and both go away once the puzzle is solved
            if game.solved() {
                return;
            }
            let mut mark = |s: usize, glyph: &'static str, color: Color| {
                let (x, y) = game.vertex_pos(game.vertex_at(s));
                ctx.print(x, y, glyph.fg(color).bold());
            };
            if let Some(s) = game.picked {
                mark(s, "●", Color::Yellow);
            }
            let glyph = if game.cursor_filled() { "●" } else { "○" };
            let color = if game.picked == Some(game.cursor) { Color::Yellow } else { Color::Cyan };
            mark(game.cursor, glyph, color);
        });
    frame.render_widget(canvas, board);

    let level = game.level();
    let text: Span = if game.solved() {
        format!(" level {level} solved in {} moves! enter: next level · q quit", game.moves).green().bold()
    } else {
        format!(
            " level {level}   crossings: {}   moves: {}   ←/→ move · space swap · q quit",
            game.crossings(),
            game.moves
        )
        .into()
    };
    frame.render_widget(text, status);
}

// ---------------------------------------------------------------------------------------------
// Main loop

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
    let mut game = Game::new(level);
    loop {
        game.tick();
        terminal.draw(|frame| render(frame, &game))?;
        if let Some(wait) = game.wake_after()
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
                KeyCode::Enter | KeyCode::Char(' ') if game.solved() => game = Game::new(game.level() + 1),
                KeyCode::Char(' ') | KeyCode::Enter => game.select(),
                _ => {}
            }
        }
    }
}
