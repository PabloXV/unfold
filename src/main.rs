use rand::rngs::Xoshiro256PlusPlus;
use rand::seq::SliceRandom;
use rand::{Rng, RngExt, SeedableRng};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Painter, Shape};
use ratatui::{DefaultTerminal, Frame};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use std::fmt;
use std::ops::{Add, Mul, Sub};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

const LEVEL_1_DOTS: usize = 4;

const SWAP_TIME: Duration = Duration::from_millis(250);
const UNFOLD_TIME: Duration = Duration::from_millis(1200);
const BLINK_DELAY: Duration = Duration::from_millis(800);
const BLINK_TIME: Duration = Duration::from_millis(900);
const FRAME_TIME: Duration = Duration::from_millis(16);

/// Chance that a new flap grows off the newest one: higher makes snakes, lower makes blobs.
const STRIP_BIAS: f64 = 0.6;

const CROSSED_COLOR: Color = Color::Indexed(174);
/// In braille dots, which are about square on screen.
const CROSSED_RADIUS: usize = 4;

const USAGE: &str = "\
usage: unfld [level | seed/level] [--daily | --rand]

  unfld 9          level 9 with a random seed
  unfld ABC123/9   level 9 of seed ABC123, to replay a puzzle or share it
  unfld --daily    today's seed, the same for everyone (changes at midnight UTC)
  unfld --rand     a random seed, which is also the default";

// ---------------------------------------------------------------------------------------------
// Seeds, scores, palette

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Seed(u32);

impl Seed {
    const MASK: u32 = (1 << 24) - 1;

    fn random() -> Self {
        Seed(rand::rng().random_range(0..=Self::MASK))
    }

    /// Every step of the mix can be undone, so no two days share a seed for 45,000 years.
    fn daily() -> Self {
        let secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let mut x = ((secs / 86_400) as u32).wrapping_add(0x5EED) & Self::MASK;
        x = x.wrapping_mul(0x9E3779) & Self::MASK;
        x ^= x >> 12;
        x = x.wrapping_mul(0x85EBCB) & Self::MASK;
        x ^= x >> 13;
        Seed(x)
    }

    fn parse(s: &str) -> Option<Self> {
        let hex = s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit());
        hex.then(|| Seed(u32::from_str_radix(s, 16).unwrap()))
    }

    fn rng(self, level: usize) -> Xoshiro256PlusPlus {
        Xoshiro256PlusPlus::seed_from_u64((self.0 as u64) << 32 | level as u64)
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:06X}", self.0)
    }
}

fn parse_level(s: &str) -> Option<usize> {
    s.parse().ok().filter(|&level| level >= 1)
}

/// A puzzle code like ABC123/9.
fn parse_code(s: &str) -> Option<(Seed, usize)> {
    let (seed, level) = s.split_once('/')?;
    Some((Seed::parse(seed)?, parse_level(level)?))
}

/// Fewest moves per puzzle, as `SEED/LEVEL MOVES` lines.
struct Scores {
    path: Option<PathBuf>,
    best: BTreeMap<(Seed, usize), usize>,
}

impl Scores {
    fn load() -> Self {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
        let path = data.map(|dir| dir.join("unfld/best"));
        let text = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        let best = text
            .lines()
            .filter_map(|line| {
                let (code, moves) = line.split_once(' ')?;
                Some((parse_code(code)?, moves.trim().parse().ok()?))
            })
            .collect();
        Scores { path, best }
    }

    fn get(&self, seed: Seed, level: usize) -> Option<usize> {
        self.best.get(&(seed, level)).copied()
    }

    /// Write errors are ignored: losing a score isn't worth interrupting the game.
    fn record(&mut self, seed: Seed, level: usize, moves: usize) {
        if self.get(seed, level).is_some_and(|best| best <= moves) {
            return;
        }
        self.best.insert((seed, level), moves);
        let Some(path) = &self.path else { return };
        let text: String = self.best.iter().map(|((seed, level), moves)| format!("{seed}/{level} {moves}\n")).collect();
        let _ = std::fs::create_dir_all(path.parent().unwrap()).and_then(|()| std::fs::write(path, text));
    }
}

/// With NO_COLOR set (no-color.org) everything uses the terminal's default color, the picked dot
/// gets its own glyph, and shadows are left out since they'd look like doubled lines.
#[derive(Clone, Copy)]
struct Palette {
    color: bool,
}

impl Palette {
    fn from_env() -> Self {
        Palette { color: std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()) }
    }

    fn pick(self, color: Color) -> Color {
        if self.color { color } else { Color::Reset }
    }

    fn picked_glyph(self) -> &'static str {
        if self.color { "●" } else { "◉" }
    }
}

// ---------------------------------------------------------------------------------------------
// Game state

struct Swap {
    a: usize,
    b: usize,
    started: Instant,
}

struct Edge {
    a: usize,
    b: usize,
    top_flap: usize,
    /// Shared by two flaps, as opposed to part of the outline.
    shared: bool,
}

struct Game {
    seed: Seed,
    prior_best: Option<usize>,
    slot_of: Vec<usize>,
    edges: Vec<Edge>,
    /// Vertex triples, stacked bottom to top like sheets of paper.
    flaps: Vec<[usize; 3]>,
    shape: Vec<Point>,
    unfolded: Vec<Point>,
    cursor: usize,
    picked: Option<usize>,
    moves: usize,
    swap: Option<Swap>,
    solved_at: Option<Instant>,
    touched_at: Instant,
}

impl Game {
    fn new(seed: Seed, level: usize, scores: &Scores) -> Self {
        let mut rng = seed.rng(level);
        let n = level + LEVEL_1_DOTS - 1;
        let (shape, mut flaps) = grow(n, &mut rng);
        flaps.shuffle(&mut rng);
        let mut game = Game {
            seed,
            prior_best: scores.get(seed, level),
            slot_of: (0..n).collect(),
            edges: edges_of(&flaps),
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
        // always shuffle at least once: vertices are numbered in growth order, which gives the puzzle away
        loop {
            game.slot_of.shuffle(&mut rng);
            if game.crossings() > 0 {
                return game;
            }
        }
    }

    fn level(&self) -> usize {
        self.slot_of.len() + 1 - LEVEL_1_DOTS
    }

    fn solved(&self) -> bool {
        self.solved_at.is_some()
    }

    fn new_best(&self) -> bool {
        self.solved() && self.prior_best.is_some_and(|best| self.moves < best)
    }

    fn best(&self) -> Option<usize> {
        self.solved().then_some(self.moves).into_iter().chain(self.prior_best).min()
    }

    fn crossing_pairs(&self) -> Vec<(&Edge, &Edge)> {
        let slots = |e: &Edge| (self.slot_of[e.a], self.slot_of[e.b]);
        let mut pairs = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            for f in &self.edges[i + 1..] {
                if chords_cross(slots(e), slots(f)) {
                    pairs.push((e, f));
                }
            }
        }
        pairs
    }

    fn crossings(&self) -> usize {
        self.crossing_pairs().len()
    }

    /// Only the crossings not hidden under a higher flap.
    fn visible_crossings(&self) -> Vec<Point> {
        let flaps = self.flap_pos();
        self.crossing_pairs()
            .into_iter()
            .filter_map(|(e, f)| {
                let pt = intersection(self.edge_pos(e), self.edge_pos(f))?;
                let hidden = flaps[e.top_flap.min(f.top_flap) + 1..].iter().any(|t| inside(t, pt));
                (!hidden).then_some(pt)
            })
            .collect()
    }

    fn vertex_at(&self, slot: usize) -> usize {
        self.slot_of.iter().position(|&s| s == slot).unwrap()
    }

    /// +1 is counterclockwise, -1 clockwise.
    fn step_cursor(&mut self, dir: isize) {
        let n = self.slot_of.len() as isize;
        self.cursor = (self.cursor as isize + dir).rem_euclid(n) as usize;
        self.touched_at = Instant::now();
    }

    fn select(&mut self) {
        self.touched_at = Instant::now();
        match self.picked {
            None => self.picked = Some(self.cursor),
            Some(s) if s == self.cursor => self.picked = None,
            Some(s) => {
                let (a, b) = (self.vertex_at(s), self.vertex_at(self.cursor));
                self.slot_of.swap(a, b);
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

    fn tick(&mut self) {
        if self.swap.as_ref().is_some_and(|s| s.started.elapsed() >= SWAP_TIME) {
            self.swap = None;
        }
    }

    fn unfold(&self) -> f64 {
        self.solved_at.map_or(0.0, |at| progress(at, SWAP_TIME, UNFOLD_TIME))
    }

    /// None means nothing will change until a key is pressed.
    fn wake_after(&self) -> Option<Duration> {
        if self.swap.is_some() || (self.solved() && self.unfold() < 1.0) {
            Some(FRAME_TIME)
        } else if !self.solved() {
            Some(self.until_blink())
        } else {
            None
        }
    }

    fn edge_pos(&self, e: &Edge) -> (Point, Point) {
        (self.vertex_pos(e.a), self.vertex_pos(e.b))
    }

    fn flap_pos(&self) -> Vec<[Point; 3]> {
        self.flaps.iter().map(|f| f.map(|v| self.vertex_pos(v))).collect()
    }

    fn vertex_pos(&self, v: usize) -> Point {
        let on_circle = self.circle_pos(v);
        match self.unfold() {
            0.0 => on_circle,
            t => on_circle.lerp(self.unfolded[v], ease(t)),
        }
    }

    fn circle_pos(&self, v: usize) -> Point {
        let n = self.slot_of.len();
        let to = slot_pos(self.slot_of[v], n);
        let Some(swap) = &self.swap else { return to };
        let partner = match v {
            _ if v == swap.a => swap.b,
            _ if v == swap.b => swap.a,
            _ => return to,
        };
        let from = slot_pos(self.slot_of[partner], n);
        from.lerp(to, ease(progress(swap.started, Duration::ZERO, SWAP_TIME)))
    }

    /// The shape turned, and mirrored if the player's outline runs the other way, to best match
    /// the solved circle, then centered and scaled to fit the board.
    fn unfold_target(&self) -> Vec<Point> {
        let n = self.shape.len();
        let mean = self.shape.iter().fold(Point::ZERO, |s, &p| s + p) * (1.0 / n as f64);
        // as complex numbers, the best rotation is the angle of sum(circle * conj(shape))
        let fit = |flip: f64| {
            let pts: Vec<Point> = self.shape.iter().map(|&p| p - mean).map(|p| Point::new(p.x * flip, p.y)).collect();
            let (re, im) = pts.iter().enumerate().fold((0.0, 0.0), |(re, im), (v, &s)| {
                let c = slot_pos(self.slot_of[v], n);
                (re + c.dot(s), im + s.cross(c))
            });
            (re.hypot(im), pts, im.atan2(re))
        };
        let (a, b) = (fit(1.0), fit(-1.0));
        let (_, pts, angle) = if a.0 >= b.0 { a } else { b };
        let pts: Vec<Point> = pts.iter().map(|p| p.rotate(angle)).collect();
        let lo = pts.iter().fold(Point::new(f64::MAX, f64::MAX), |m, p| Point::new(m.x.min(p.x), m.y.min(p.y)));
        let hi = pts.iter().fold(Point::new(f64::MIN, f64::MIN), |m, p| Point::new(m.x.max(p.x), m.y.max(p.y)));
        let center = lo.lerp(hi, 0.5);
        let radius = pts.iter().map(|&p| p.dist(center)).fold(0.0, f64::max);
        pts.iter().map(|&p| (p - center) * (1.0 / radius)).collect()
    }

    fn cursor_filled(&self) -> bool {
        let Some(idle) = self.touched_at.elapsed().checked_sub(BLINK_DELAY) else { return true };
        (idle.as_millis() / BLINK_TIME.as_millis()) % 2 == 1
    }

    fn until_blink(&self) -> Duration {
        let elapsed = self.touched_at.elapsed();
        match elapsed.checked_sub(BLINK_DELAY) {
            None => BLINK_DELAY - elapsed,
            Some(idle) => BLINK_TIME - Duration::from_nanos((idle.as_nanos() % BLINK_TIME.as_nanos()) as u64),
        }
    }
}

fn edges_of(flaps: &[[usize; 3]]) -> Vec<Edge> {
    let mut edges: Vec<Edge> = Vec::new();
    for &[a, b, c] in flaps {
        for (u, v) in [(a, b), (b, c), (a, c)] {
            let (a, b) = (u.min(v), u.max(v));
            if edges.iter().any(|e| (e.a, e.b) == (a, b)) {
                continue;
            }
            let owns = |f: &[usize; 3]| f.contains(&a) && f.contains(&b);
            edges.push(Edge {
                a,
                b,
                top_flap: flaps.iter().rposition(owns).unwrap(),
                shared: flaps.iter().filter(|f| owns(f)).count() == 2,
            });
        }
    }
    edges
}

fn progress(start: Instant, delay: Duration, length: Duration) -> f64 {
    (start.elapsed().saturating_sub(delay).as_secs_f64() / length.as_secs_f64()).min(1.0)
}

/// Chords between circle slots cross when exactly one end of the second lies strictly
/// between the ends of the first.
fn chords_cross((a, b): (usize, usize), (c, d): (usize, usize)) -> bool {
    if a == c || a == d || b == c || b == d {
        return false;
    }
    let between = |v: usize| a.min(b) < v && v < a.max(b);
    between(c) != between(d)
}

// ---------------------------------------------------------------------------------------------
// Level generation

/// Grows a flat sheet of triangles by folding each new one out from an outline edge,
/// giving a triangulated polygon of any outline, not just a convex one.
fn grow(n: usize, rng: &mut impl Rng) -> (Vec<Point>, Vec<[usize; 3]>) {
    'restart: loop {
        let mut pts = vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.5, 3f64.sqrt() / 2.0)];
        let mut tris = vec![[0, 1, 2]];
        while pts.len() < n {
            let (outline, fresh) = outline(&tris);
            let placed = (0..200).find_map(|_| {
                let pool = if rng.random::<f64>() < STRIP_BIAS { &fresh } else { &outline };
                let (a, b, opposite) = pool[rng.random_range(0..pool.len())];
                let (p, q) = (pts[a], pts[b]);
                let d = q - p;
                let mid = p.lerp(q, 0.5);
                let mut normal = Point::new(-d.y, d.x) * (1.0 / p.dist(q));
                if normal.dot(pts[opposite] - mid) > 0.0 {
                    normal = normal * -1.0;
                }
                let height = p.dist(q) * 3f64.sqrt() / 2.0 * rng.random_range(0.7..1.3);
                let apex = mid + normal * height + d * rng.random_range(-0.25..0.25);
                fits(&pts, &tris, apex, p, q).then_some((apex, a, b))
            });
            let Some((apex, a, b)) = placed else { continue 'restart };
            pts.push(apex);
            tris.push([a, b, pts.len() - 1]);
        }
        return (pts, tris);
    }
}

/// An outline edge's ends, and the third corner of its triangle.
type OutlineEdge = (usize, usize, usize);

/// The whole outline, and the part of it on the newest triangle.
fn outline(tris: &[[usize; 3]]) -> (Vec<OutlineEdge>, Vec<OutlineEdge>) {
    let (mut all, mut fresh) = (Vec::new(), Vec::new());
    for (i, &[a, b, c]) in tris.iter().enumerate() {
        for edge @ (u, v, _) in [(a, b, c), (b, c, a), (c, a, b)] {
            if !tris.iter().any(|t| t != &tris[i] && t.contains(&u) && t.contains(&v)) {
                all.push(edge);
                if i == tris.len() - 1 {
                    fresh.push(edge);
                }
            }
        }
    }
    (all, fresh)
}

/// Whether a new flap on edge pq lies flat on the sheet without overlapping it or making slivers.
fn fits(pts: &[Point], tris: &[[usize; 3]], apex: Point, p: Point, q: Point) -> bool {
    const GAP: f64 = 0.25;
    if ![p, q].iter().all(|&e| (0.6..1.5).contains(&apex.dist(e))) {
        return false;
    }
    if pts.iter().any(|&v| v.dist(apex) < 0.5 || inside(&[apex, p, q], v)) {
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

#[derive(Clone, Copy, PartialEq, Debug)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    const ZERO: Point = Point::new(0.0, 0.0);

    const fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn dot(self, o: Point) -> f64 {
        self.x * o.x + self.y * o.y
    }

    fn cross(self, o: Point) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Plain sqrt rather than hypot: sqrt is exact everywhere while hypot comes from the system's
    /// math library, and a seed has to generate the same level on every platform.
    fn dist(self, o: Point) -> f64 {
        let d = self - o;
        d.dot(d).sqrt()
    }

    fn lerp(self, o: Point, t: f64) -> Point {
        self + (o - self) * t
    }

    fn rotate(self, angle: f64) -> Point {
        let (sin, cos) = angle.sin_cos();
        Point::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}

fn ease(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn slot_pos(slot: usize, n: usize) -> Point {
    let angle = TAU * slot as f64 / n as f64;
    Point::new(angle.cos(), angle.sin())
}

/// Positive when p is left of the line from u to v.
fn side(u: Point, v: Point, p: Point) -> f64 {
    (v - u).cross(p - u)
}

/// Strictly inside, so points on the border don't count.
fn inside(&[a, b, c]: &[Point; 3], p: Point) -> bool {
    let (d1, d2, d3) = (side(a, b, p), side(b, c, p), side(c, a, p));
    let eps = 1e-9;
    (d1 > eps && d2 > eps && d3 > eps) || (d1 < -eps && d2 < -eps && d3 < -eps)
}

fn segments_cross((a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

fn intersection((a, b): (Point, Point), (c, d): (Point, Point)) -> Option<Point> {
    let (r, s) = (b - a, d - c);
    let denom = r.cross(s);
    if denom == 0.0 {
        return None;
    }
    let t = (c - a).cross(s) / denom;
    let u = (c - a).cross(r) / denom;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| a.lerp(b, t))
}

fn segment_dist(p: Point, (a, b): (Point, Point)) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.dot(d)).clamp(0.0, 1.0);
    a.lerp(b, t).dist(p)
}

// ---------------------------------------------------------------------------------------------
// Drawing

/// Creases drawn as if each flap were an opaque sheet of paper.
struct Flaps {
    tris: Vec<[Point; 3]>,
    creases: Vec<Crease>,
    color: Color,
    dissolve: f64,
    /// A shadow lies under its own flap too, not just the ones above.
    is_shadow: bool,
    crossings: Vec<Point>,
    crossed_color: Color,
}

#[derive(Clone, Copy)]
struct Crease {
    p: Point,
    q: Point,
    top_flap: usize,
    shared: bool,
}

impl Flaps {
    fn paper(game: &Game, palette: Palette, crossings: Vec<Point>) -> Self {
        Flaps {
            tris: game.flap_pos(),
            creases: game
                .edges
                .iter()
                .map(|e| {
                    let (p, q) = game.edge_pos(e);
                    Crease { p, q, top_flap: e.top_flap, shared: e.shared }
                })
                .collect(),
            color: palette.pick(if game.solved() { Color::Green } else { Color::Gray }),
            dissolve: game.unfold(),
            is_shadow: false,
            crossings,
            crossed_color: palette.pick(CROSSED_COLOR),
        }
    }

    /// Two flaps sharing a crease are one sheet, not stacked, so only outline creases cast a shadow.
    fn shadow(&self, offset: Point) -> Self {
        Flaps {
            tris: self.tris.clone(),
            creases: self
                .creases
                .iter()
                .filter(|c| !c.shared)
                .map(|&c| Crease { p: c.p + offset, q: c.q + offset, ..c })
                .collect(),
            color: Color::DarkGray,
            dissolve: 0.0,
            is_shadow: true,
            crossings: Vec::new(),
            crossed_color: Color::DarkGray,
        }
    }
}

impl Shape for Flaps {
    fn draw(&self, painter: &mut Painter) {
        let dot = |painter: &Painter, p: Point| painter.get_point(p.x, p.y);
        let crossings: Vec<_> = self.crossings.iter().filter_map(|&c| dot(painter, c)).collect();
        let near_crossing = |(x, y): (usize, usize)| {
            crossings.iter().any(|&(cx, cy)| {
                let (dx, dy) = (cx.abs_diff(x), cy.abs_diff(y));
                dx * dx + dy * dy <= CROSSED_RADIUS * CROSSED_RADIUS
            })
        };
        for (e, &Crease { p, q, top_flap, shared }) in self.creases.iter().enumerate() {
            let covering = &self.tris[if self.is_shadow { top_flap } else { top_flap + 1 }..];
            let steps = match (dot(painter, p), dot(painter, q)) {
                (Some(a), Some(b)) => a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)).max(1),
                _ => 100,
            };
            for i in 0..=steps {
                // each dot has a fixed pseudo-random threshold, so a crease dissolves in a scatter
                let threshold = (i.wrapping_mul(2654435761) ^ e.wrapping_mul(40503)) % 1000;
                if shared && (threshold as f64) < self.dissolve * 1000.0 {
                    continue;
                }
                let pt = p.lerp(q, i as f64 / steps as f64);
                if !covering.iter().any(|t| inside(t, pt))
                    && let Some(xy) = dot(painter, pt)
                {
                    painter.paint(xy.0, xy.1, if near_crossing(xy) { self.crossed_color } else { self.color });
                }
            }
        }
    }
}

/// Canvas coordinates for the board: a unit circle with some padding, widened to stay round
/// in cells about twice as tall as wide.
struct Board {
    half_w: f64,
    half_h: f64,
    cols: f64,
    rows: f64,
}

impl Board {
    fn new(area: Rect) -> Self {
        const PAD: f64 = 1.2;
        let (cols, rows) = (area.width as f64, area.height as f64);
        Board { half_w: PAD * cols / (rows * 2.0), half_h: PAD, cols, rows }
    }

    fn cell(&self) -> Point {
        Point::new(2.0 * self.half_w / self.cols, 2.0 * self.half_h / self.rows)
    }

    /// Canvas text and braille dots round to cells differently, so text printed at a point can
    /// land a cell away from the dot drawn there. This finds the dot's cell (as Painter::get_point
    /// does) and returns the point that text placement maps to that cell.
    fn text_pos(&self, p: Point) -> Point {
        let dot_x = ((p.x + self.half_w) * (self.cols * 2.0 - 1.0) / (2.0 * self.half_w)).round();
        let dot_y = ((self.half_h - p.y) * (self.rows * 4.0 - 1.0) / (2.0 * self.half_h)).round();
        let (col, row) = ((dot_x / 2.0).floor(), (dot_y / 4.0).floor());
        Point::new(
            -self.half_w + (col + 0.5).min(self.cols - 1.0) * 2.0 * self.half_w / (self.cols - 1.0),
            self.half_h - (row + 0.5).min(self.rows - 1.0) * 2.0 * self.half_h / (self.rows - 1.0),
        )
    }
}

fn render(frame: &mut Frame, game: &Game, palette: Palette) {
    let [header, area, status] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let best = game.best().map_or(String::new(), |best| format!(" · best {best}"));
    frame.render_widget(Span::from(format!(" {}/{}{best}", game.seed, game.level())).fg(palette.pick(Color::DarkGray)), header);

    let board = Board::new(area);
    let crossings = game.visible_crossings();
    let paper = Flaps::paper(game, palette, crossings.clone());
    // a whole cell, because a cell holds one color: a smaller shift would mostly land in cells
    // the creases already own and get painted over
    let shadow = paper.shadow(Point::new(board.cell().x, -board.cell().y));
    let canvas = Canvas::default()
        .x_bounds([-board.half_w, board.half_w])
        .y_bounds([-board.half_h, board.half_h])
        .paint(|ctx| {
            if palette.color {
                ctx.draw(&shadow);
                ctx.layer();
            }
            ctx.draw(&paper);
            let mut print = |at: Point, glyph: Span<'static>| {
                let p = board.text_pos(at);
                ctx.print(p.x, p.y, glyph);
            };
            for &pt in &crossings {
                print(pt, "×".fg(palette.pick(CROSSED_COLOR)));
            }
            if game.solved() {
                return;
            }
            let mut mark = |slot: usize, glyph: &'static str, color: Color| {
                print(game.vertex_pos(game.vertex_at(slot)), glyph.fg(palette.pick(color)).bold());
            };
            if let Some(slot) = game.picked {
                mark(slot, palette.picked_glyph(), Color::Yellow);
            }
            let on_picked = game.picked == Some(game.cursor);
            let glyph = match (game.cursor_filled(), on_picked) {
                (false, _) => "○",
                (true, false) => "●",
                (true, true) => palette.picked_glyph(),
            };
            mark(game.cursor, glyph, if on_picked { Color::Yellow } else { Color::Cyan });
        });
    frame.render_widget(canvas, area);

    let text: Span = if game.solved() {
        let new_best = if game.new_best() { " new best!" } else { "" };
        format!(" solved in {} moves!{new_best} enter: next level · r replay · q quit", game.moves)
            .fg(palette.pick(Color::Green))
            .bold()
    } else {
        format!(" crossings: {}   moves: {}   ←/→ move · space swap · r restart · q quit", game.crossings(), game.moves).into()
    };
    frame.render_widget(text, status);
}

// ---------------------------------------------------------------------------------------------
// Main loop

fn main() -> std::io::Result<()> {
    let Some((seed, level)) = parse_args(std::env::args().skip(1)) else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    ratatui::run(|terminal| app(terminal, seed, level, Palette::from_env()))
}

fn parse_args(args: impl Iterator<Item = String>) -> Option<(Seed, usize)> {
    let (mut seed, mut level) = (None, 1);
    for arg in args {
        let (s, l) = match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--daily" => (Some(Seed::daily()), None),
            "--rand" => (Some(Seed::random()), None),
            code if code.contains('/') => parse_code(code).map(|(s, l)| (Some(s), Some(l)))?,
            n => (None, Some(parse_level(n)?)),
        };
        if let Some(s) = s
            && seed.replace(s).is_some()
        {
            return None;
        }
        level = l.unwrap_or(level);
    }
    Some((seed.unwrap_or_else(Seed::random), level))
}

fn app(terminal: &mut DefaultTerminal, seed: Seed, level: usize, palette: Palette) -> std::io::Result<()> {
    let mut scores = Scores::load();
    let mut game = Game::new(seed, level, &scores);
    loop {
        game.tick();
        terminal.draw(|frame| render(frame, &game, palette))?;
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
                KeyCode::Char('r') => game = Game::new(seed, game.level(), &scores),
                KeyCode::Enter | KeyCode::Char(' ') if game.solved() => game = Game::new(seed, game.level() + 1, &scores),
                KeyCode::Char(' ') | KeyCode::Enter => {
                    game.select();
                    if game.solved() {
                        scores.record(seed, game.level(), game.moves);
                    }
                }
                _ => {}
            }
        }
    }
}
