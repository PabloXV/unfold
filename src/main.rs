use rand::rngs::Xoshiro256PlusPlus;
use rand::seq::SliceRandom;
use rand::{Rng, RngExt, SeedableRng};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Stylize};
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Painter, Shape};
use ratatui::{DefaultTerminal, Frame};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

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

/// a muted red (256-color palette) for crossings and the bits of edges right around them
const CROSSED_COLOR: Color = Color::Indexed(174);
/// how far the red reaches from a crossing, in braille dots (2 per cell across, 4 down)
const CROSSED_RADIUS: usize = 4;

/// Colors for everything on screen. With NO_COLOR set (see no-color.org) they all become the
/// terminal's default and glyphs carry what color would have: the picked dot gets its own symbol,
/// and shadows are left out since they'd just look like doubled lines.
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

const USAGE: &str = "\
usage: unfld [level | seed/level] [--daily | --rand]

  unfld 9          level 9 with a random seed
  unfld ABC123/9   level 9 of seed ABC123, to replay a puzzle or share it
  unfld --daily    today's seed, the same for everyone (changes at midnight UTC)
  unfld --rand     a random seed, which is also the default";

// ---------------------------------------------------------------------------------------------
// Seeds and scores

/// A play session's seed: 6 hex digits, short enough to read out to a friend.
/// Together with a level it pins down one exact puzzle, written like ABC123/9.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Seed(u32);

impl Seed {
    const MASK: u32 = (1 << 24) - 1;

    fn random() -> Self {
        Seed(rand::rng().random_range(0..=Self::MASK))
    }

    /// Today's seed, by UTC date. The day number goes through a mix of multiplies by odd
    /// numbers and xor-shifts, each of which can be undone, so no two days share a seed
    /// for the next 45,000 years.
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
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:06X}", self.0)
    }
}

fn parse_level(s: &str) -> Option<usize> {
    s.parse().ok().filter(|&level| level >= 1)
}

/// Parses a puzzle code like ABC123/9.
fn parse_code(s: &str) -> Option<(Seed, usize)> {
    let (seed, level) = s.split_once('/')?;
    Some((Seed::parse(seed)?, parse_level(level)?))
}

/// Fewest moves per puzzle, kept in a plain text file of `SEED/LEVEL MOVES` lines.
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

    /// Saves a solve if it beats the old best. A score that can't be written is just lost;
    /// it's not worth interrupting the game over.
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
    seed: Seed,
    /// the fewest moves this puzzle has been solved in before, on this machine
    prior_best: Option<usize>,
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
    fn new(seed: Seed, level: usize, scores: &Scores) -> Self {
        // the same seed and level always make the same puzzle, so it can be replayed or shared
        let mut rng = Xoshiro256PlusPlus::seed_from_u64((seed.0 as u64) << 32 | level as u64);
        let n = level + LEVEL_1_DOTS - 1;
        let (shape, mut flaps) = grow(n, &mut rng);
        flaps.shuffle(&mut rng);

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
            seed,
            prior_best: scores.get(seed, level),
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
        // shuffle at least once (vertices are numbered in growth order, which would give the puzzle
        // away), and keep going until there's something to untangle
        game.slot.shuffle(&mut rng);
        while game.crossings() == 0 {
            game.slot.shuffle(&mut rng);
        }
        game
    }

    fn level(&self) -> usize {
        self.slot.len() + 1 - LEVEL_1_DOTS
    }

    fn solved(&self) -> bool {
        self.solved_at.is_some()
    }

    /// Whether this solve beat an earlier one.
    fn new_best(&self) -> bool {
        self.solved() && self.prior_best.is_some_and(|best| self.moves < best)
    }

    /// The fewest moves this puzzle has been solved in, counting this solve.
    fn best(&self) -> Option<usize> {
        let this = self.solved().then_some(self.moves);
        this.into_iter().chain(self.prior_best).min()
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

    /// Where crossing edges meet on screen right now. A crossing under a higher flap than
    /// either edge can't be seen, so it isn't included; the count still includes it.
    fn crossing_points(&self) -> Vec<Point> {
        let flaps = self.flap_pos();
        self.crossing_pairs()
            .into_iter()
            .filter_map(|(i, j)| {
                let (a, b) = (&self.edges[i], &self.edges[j]);
                let pt = intersection(self.edge_pos(a), self.edge_pos(b))?;
                let hidden = flaps[a.z.min(b.z) + 1..].iter().any(|f| inside(f, pt));
                (!hidden).then_some(pt)
            })
            .collect()
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

    fn edge_pos(&self, e: &Edge) -> (Point, Point) {
        (self.vertex_pos(e.a), self.vertex_pos(e.b))
    }

    /// Each flap's corners as drawn, bottom to top.
    fn flap_pos(&self) -> Vec<[Point; 3]> {
        self.flaps.iter().map(|f| f.map(|v| self.vertex_pos(v))).collect()
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
fn grow(n: usize, rng: &mut impl Rng) -> (Vec<Point>, Vec<[usize; 3]>) {
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

/// Plain sqrt rather than hypot: sqrt is exact on every platform while hypot comes from the
/// system's math library, and level generation must come out the same everywhere for a seed.
fn dist(a: Point, b: Point) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
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
    /// where edges cross, in canvas coordinates; creases are tinted around these
    crossings: Vec<Point>,
    crossed_color: Color,
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
    fn paper(game: &Game, palette: Palette, crossings: Vec<Point>) -> Self {
        let color = palette.pick(if game.solved() { Color::Green } else { Color::Gray });
        Flaps {
            tris: game.flap_pos(),
            creases: game
                .edges
                .iter()
                .map(|e| {
                    let (p, q) = game.edge_pos(e);
                    Crease { p, q, z: e.z, internal: e.internal, color }
                })
                .collect(),
            fade: game.unfold(),
            shadow: false,
            crossings,
            crossed_color: palette.pick(CROSSED_COLOR),
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
            crossed_color: Color::DarkGray,
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
                    painter.paint(x, y, if near_crossing((x, y)) { self.crossed_color } else { color });
                }
            }
        }
    }
}

fn render(frame: &mut Frame, game: &Game, palette: Palette) {
    let [header, board, status] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let best = game.best().map_or(String::new(), |best| format!(" · best {best}"));
    frame.render_widget(Span::from(format!(" {}/{}{best}", game.seed, game.level())).fg(palette.pick(Color::DarkGray)), header);

    // terminal cells are ~twice as tall as wide, so widen the x range to keep the circle round
    let aspect = board.width as f64 / (board.height as f64 * 2.0);
    let pad = 1.2;
    let (half_w, half_h) = (pad * aspect, pad);
    let (cols, rows) = (board.width as f64, board.height as f64);
    // Text printed on the canvas and braille dots round positions to cells differently, so text
    // printed at a point can land a cell away from the dots drawn there. This finds the cell
    // holding the point's dot (the same math as Painter::get_point) and returns its center in the
    // terms text placement uses.
    let print_at = |(x, y): Point| -> Point {
        let dot_x = ((x + half_w) * (cols * 2.0 - 1.0) / (2.0 * half_w)).round();
        let dot_y = ((half_h - y) * (rows * 4.0 - 1.0) / (2.0 * half_h)).round();
        let (col, row) = ((dot_x / 2.0).floor(), (dot_y / 4.0).floor());
        (
            -half_w + (col + 0.5).min(cols - 1.0) * 2.0 * half_w / (cols - 1.0),
            half_h - (row + 0.5).min(rows - 1.0) * 2.0 * half_h / (rows - 1.0),
        )
    };
    let crossings = game.crossing_points();
    let paper = Flaps::paper(game, palette, crossings.clone());
    // the shadow sits one whole cell down and right: a cell holds one color, so a smaller
    // offset would mostly land in cells the creases already own and get painted over
    let shadow = paper.shadow((2.0 * half_w / cols, -2.0 * half_h / rows));
    let canvas = Canvas::default()
        .x_bounds([-half_w, half_w])
        .y_bounds([-half_h, half_h])
        .paint(|ctx| {
            if palette.color {
                ctx.draw(&shadow);
                ctx.layer();
            }
            ctx.draw(&paper);
            for &pt in &crossings {
                let (x, y) = print_at(pt);
                ctx.print(x, y, "×".fg(palette.pick(CROSSED_COLOR)));
            }
            // plain vertices are just where edges meet; only the cursor and pick get a marker,
            // and both go away once the puzzle is solved
            if game.solved() {
                return;
            }
            let mut mark = |s: usize, glyph: &'static str, color: Color| {
                let (x, y) = print_at(game.vertex_pos(game.vertex_at(s)));
                ctx.print(x, y, glyph.fg(palette.pick(color)).bold());
            };
            if let Some(s) = game.picked {
                mark(s, palette.picked_glyph(), Color::Yellow);
            }
            let on_picked = game.picked == Some(game.cursor);
            let glyph = match (game.cursor_filled(), on_picked) {
                (false, _) => "○",
                (true, false) => "●",
                (true, true) => palette.picked_glyph(),
            };
            mark(game.cursor, glyph, if on_picked { Color::Yellow } else { Color::Cyan });
        });
    frame.render_widget(canvas, board);

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

/// The seed and starting level from the command line, or None if it doesn't make sense.
/// The seed lasts the whole session; only the level moves on.
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
        // a seed can only be given once
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
