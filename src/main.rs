use rand::rngs::Xoshiro256PlusPlus;
use rand::seq::SliceRandom;
use rand::{Rng, RngExt, SeedableRng};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Painter, Shape};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};
use std::fmt;
use std::ops::{Add, Mul, Sub};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

const LEVEL_1_DOTS: usize = 4;

const SLIDE_TIME: Duration = Duration::from_millis(150);
const UNFOLD_TIME: Duration = Duration::from_millis(1200);
const BLINK_DELAY: Duration = Duration::from_millis(800);
const BLINK_TIME: Duration = Duration::from_millis(900);
const FRAME_TIME: Duration = Duration::from_millis(16);
const LIFT_TIME: Duration = Duration::from_millis(120);
const COPIED_TIME: Duration = Duration::from_millis(1500);
/// How long a first press of r waits for the second that restarts.
const RESTART_TIME: Duration = Duration::from_millis(2000);

/// How far the hovered and carried dots stand out from the circle, as a fraction of its radius.
const LIFT: f64 = 0.06;

/// Chance that a new flap grows off the newest one: higher makes snakes, lower makes blobs.
const STRIP_BIAS: f64 = 0.6;

const CROSSED_COLOR: Color = Color::LightRed;
/// In braille dots, which are about square on screen.
const CROSSED_RADIUS: usize = 4;

const USAGE: &str = "\
usage: unfld [level | seed/level] [--daily [date | ±days] | --rand]

  unfld                      today's seed, the same for everyone (changes at midnight UTC)
  unfld 9                    level 9 of today's seed
  unfld ABC123/9             level 9 of seed ABC123, to replay a puzzle or share it
  unfld --daily 2026-10-21   that day's seed
  unfld --daily -1           yesterday's seed, or +1 for tomorrow's
  unfld --rand               a random seed

Without a level, play picks up at the first level of the seed not yet solved.";

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
    fn daily(day: u64) -> Self {
        let mut x = (day as u32).wrapping_add(0x5EED) & Self::MASK;
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

/// Days since the Unix epoch, in UTC.
fn today() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() / 86_400
}

/// Year, month and day of a day since the epoch, by Howard Hinnant's civil_from_days.
fn civil_from_days(day: u64) -> (u64, u64, u64) {
    let z = day + 719_468;
    let (era, doe) = (z / 146_097, z % 146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (era * 400 + yoe + (m <= 2) as u64, m, d)
}

/// The inverse, days_from_civil. Out of range months and days run over into the next ones.
fn days_from_civil(y: u64, m: u64, d: u64) -> u64 {
    let y = y - (m <= 2) as u64;
    let (era, yoe) = (y / 400, y % 400);
    let doy = (153 * if m > 2 { m - 3 } else { m + 9 } + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe).saturating_sub(719_468)
}

/// Like "2026 Oct 27".
fn format_day(day: u64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d) = civil_from_days(day);
    format!("{y} {} {d}", MONTHS[m as usize - 1])
}

/// A date like 2026-10-21, or days from today like -1 or +1.
fn parse_day(s: &str, today: u64) -> Option<u64> {
    let number = |n: &str| n.bytes().all(|b| b.is_ascii_digit()).then(|| n.parse::<u64>().ok()).flatten();
    if let Some(n) = s.strip_prefix('+') {
        return today.checked_add(number(n)?);
    }
    if let Some(n) = s.strip_prefix('-') {
        return today.checked_sub(number(n)?);
    }
    let parts: Vec<u64> = s.split('-').map(number).collect::<Option<_>>()?;
    let &[y, m, d] = &parts[..] else { return None };
    if !(1970..=9999).contains(&y) || !(1..=12).contains(&m) || d == 0 {
        return None;
    }
    // a day past the end of its month comes back as a different date
    let day = days_from_civil(y, m, d);
    (civil_from_days(day) == (y, m, d)).then_some(day)
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
        // not "best", which held scores from when a move was a swap
        let path = data.map(|dir| dir.join("unfld/scores"));
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

    fn first_unsolved(&self, seed: Seed) -> usize {
        (1..).find(|&level| self.get(seed, level).is_none()).unwrap()
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

/// With NO_COLOR set (no-color.org) everything uses the terminal's default color, the carried dot
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

/// Every dot gliding from where it was drawn when the slide began to where it now belongs.
struct Slide {
    from: Vec<Point>,
    started: Instant,
}

/// A dot picked up and walked around the circle, pushing the others along one slot at a time.
struct Carry {
    v: usize,
    /// Where every dot was at pick up, to put them back.
    before: Vec<usize>,
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
    carry: Option<Carry>,
    moves: usize,
    slide: Option<Slide>,
    solved_at: Option<Instant>,
    touched_at: Instant,
    /// Per vertex, from 0 on the circle to 1 fully lifted out.
    lift: Vec<f64>,
    ticked_at: Instant,
    copied_at: Option<Instant>,
    restart_asked_at: Option<Instant>,
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
            carry: None,
            moves: 0,
            slide: None,
            solved_at: None,
            touched_at: Instant::now(),
            lift: vec![0.0; n],
            ticked_at: Instant::now(),
            copied_at: None,
            restart_asked_at: None,
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

    fn code(&self) -> String {
        format!("{}/{}", self.seed, self.level())
    }

    fn copy_code(&mut self) {
        copy_to_clipboard(&self.code());
        self.copied_at = Some(Instant::now());
    }

    fn until_copied_fades(&self) -> Option<Duration> {
        time_left(self.copied_at, COPIED_TIME)
    }

    fn until_restart_lapses(&self) -> Option<Duration> {
        time_left(self.restart_asked_at, RESTART_TIME)
    }

    /// Whether restarting would lose anything.
    fn in_progress(&self) -> bool {
        !self.solved() && (self.moves > 0 || self.carry.is_some())
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

    /// +1 is counterclockwise, -1 clockwise. A carried dot comes along, trading places with the
    /// dot it steps onto.
    fn step_cursor(&mut self, dir: isize) {
        let n = self.slot_of.len() as isize;
        let next = (self.cursor as isize + dir).rem_euclid(n) as usize;
        if let Some(v) = self.carry.as_ref().map(|c| c.v) {
            self.begin_slide();
            let w = self.vertex_at(next);
            self.slot_of.swap(v, w);
        }
        self.cursor = next;
        self.touched_at = Instant::now();
    }

    /// Picks up the dot under the cursor, or sets the carried one down. Setting it down between
    /// the same two dots it left changes nothing, so it isn't a move.
    fn select(&mut self) {
        self.touched_at = Instant::now();
        let Some(carry) = self.carry.take() else {
            self.carry = Some(Carry { v: self.vertex_at(self.cursor), before: self.slot_of.clone() });
            return;
        };
        if neighbors(&self.slot_of, carry.v) == neighbors(&carry.before, carry.v) {
            return;
        }
        self.moves += 1;
        if self.crossings() == 0 {
            self.solved_at = Some(Instant::now());
            self.unfolded = self.unfold_target();
        }
    }

    fn put_back(&mut self) {
        let Some(carry) = self.carry.take() else { return };
        self.begin_slide();
        self.slot_of = carry.before;
        self.cursor = self.slot_of[carry.v];
        self.touched_at = Instant::now();
    }

    fn carrying(&self) -> bool {
        self.carry.is_some()
    }

    /// Call before moving dots, so they glide from where they're drawn, even mid-slide.
    fn begin_slide(&mut self) {
        let from = (0..self.slot_of.len()).map(|v| self.circle_pos(v)).collect();
        self.slide = Some(Slide { from, started: Instant::now() });
    }

    fn tick(&mut self) {
        if self.slide.as_ref().is_some_and(|s| s.started.elapsed() >= SLIDE_TIME) {
            self.slide = None;
        }
        // counted from the last key too, since no frames are drawn while everything is still
        let step = self.ticked_at.max(self.touched_at).elapsed().as_secs_f64() / LIFT_TIME.as_secs_f64();
        self.ticked_at = Instant::now();
        for v in 0..self.lift.len() {
            let target = self.lift_target(v);
            self.lift[v] += (target - self.lift[v]).clamp(-step, step);
        }
    }

    /// Only the dot under the cursor, which is the carried one while carrying.
    fn lift_target(&self, v: usize) -> f64 {
        if self.slot_of[v] == self.cursor && !self.solved() { 1.0 } else { 0.0 }
    }

    fn lifting(&self) -> bool {
        (0..self.lift.len()).any(|v| self.lift[v] != self.lift_target(v))
    }

    fn unfold(&self) -> f64 {
        self.solved_at.map_or(0.0, |at| progress(at, SLIDE_TIME, UNFOLD_TIME))
    }

    /// None means nothing will change until a key is pressed.
    fn wake_after(&self) -> Option<Duration> {
        let next = if self.slide.is_some() || self.lifting() || (self.solved() && self.unfold() < 1.0) {
            Some(FRAME_TIME)
        } else if !self.solved() {
            Some(self.until_blink())
        } else {
            None
        };
        next.into_iter().chain(self.until_copied_fades()).chain(self.until_restart_lapses()).min()
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
        let to = self.rest_pos(v);
        let Some(slide) = &self.slide else { return to };
        polar_lerp(slide.from[v], to, ease(progress(slide.started, Duration::ZERO, SLIDE_TIME)))
    }

    /// In its slot, lifted out as far as it is.
    fn rest_pos(&self, v: usize) -> Point {
        slot_pos(self.slot_of[v], self.slot_of.len()) * (1.0 + LIFT * ease(self.lift[v]))
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

/// The dots on either side of v, counterclockwise first.
fn neighbors(slot_of: &[usize], v: usize) -> (usize, usize) {
    let n = slot_of.len();
    let at = |slot: usize| slot_of.iter().position(|&s| s == slot % n).unwrap();
    (at(slot_of[v] + 1), at(slot_of[v] + n - 1))
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

/// None once a timed notice started at `at` has run its length.
fn time_left(at: Option<Instant>, length: Duration) -> Option<Duration> {
    length.checked_sub(at?.elapsed()).filter(|d| !d.is_zero())
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

/// Around the center rather than straight across, so dots glide along the circle.
fn polar_lerp(a: Point, b: Point, t: f64) -> Point {
    let (ra, rb) = (a.dist(Point::ZERO), b.dist(Point::ZERO));
    let (ta, tb) = (a.y.atan2(a.x), b.y.atan2(b.x));
    let turn = (tb - ta + PI).rem_euclid(TAU) - PI;
    Point::new(1.0, 0.0).rotate(ta + turn * t) * (ra + (rb - ra) * t)
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
            color: palette.pick(if game.solved() { Color::Green } else { Color::Reset }),
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

    /// Whether a block of cells in the top-left corner clears the circle by at least a cell.
    fn corner_is_free(&self, width: u16, height: u16) -> bool {
        let cell = self.cell();
        if width as f64 > self.cols || height as f64 > self.rows {
            return false;
        }
        let inner = Point::new(-self.half_w + width as f64 * cell.x, self.half_h - height as f64 * cell.y);
        let nearest = Point::new(inner.x.min(0.0), inner.y.max(0.0));
        nearest.dist(Point::ZERO) > 1.0 + LIFT + cell.y
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

/// What's on screen besides the game itself.
struct View {
    title: Title,
    palette: Palette,
    /// Set when playing the daily puzzle.
    date: Option<String>,
    help: bool,
}

const HELP: [(&str, &str); 8] = [
    ("←/→  h/l", "move around the circle"),
    ("space/enter", "pick up a dot, carry it, set it down"),
    ("esc", "put a carried dot back where it was"),
    ("[ / ]", "previous or next level, each kept as you left it"),
    ("r r", "restart, or replay once solved"),
    ("c", "copy the puzzle code"),
    ("?", "show or hide this help"),
    ("q/esc", "quit"),
];

fn render_help(frame: &mut Frame, area: Rect, palette: Palette) {
    let goal = "Move dots until no creases cross.";
    let lines: Vec<Line> = [Line::from(goal), Line::default()]
        .into_iter()
        .chain(HELP.iter().map(|(keys, what)| Line::from(vec![format!("{keys:<13}").fg(palette.pick(Color::Cyan)).bold(), (*what).into()])))
        .collect();
    let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 4;
    let box_area = area.centered(Constraint::Length(width), Constraint::Length(lines.len() as u16 + 2));
    let block = Block::bordered().title(" help ").padding(Padding::horizontal(1));
    frame.render_widget(Clear, box_area);
    frame.render_widget(Paragraph::new(lines).block(block), box_area);
}

fn render(frame: &mut Frame, game: &Game, view: &View) {
    let (title, palette) = (&view.title, view.palette);
    let [area, status] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
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
            match (game.carrying(), game.cursor_filled()) {
                (true, _) => mark(game.cursor, palette.picked_glyph(), Color::Yellow),
                (false, true) => mark(game.cursor, "●", Color::Cyan),
                (false, false) => mark(game.cursor, "○", Color::Cyan),
            }
        });
    frame.render_widget(canvas, area);

    // the title and seed sit over the board's top-left corner, which the centered circle leaves
    // empty unless the terminal is narrow; then only the seed shows
    let mut info_row = area.y;
    if board.corner_is_free(Title::WIDTH, Title::HEIGHT + 1) {
        title.render(frame, Rect { width: Title::WIDTH, height: Title::HEIGHT, ..area }, palette);
        info_row += Title::HEIGHT;
    }
    let date = view.date.as_ref().map_or(String::new(), |date| format!(" · {date}"));
    let best = game.best().map_or(String::new(), |best| format!(" · best {best}"));
    let mut info = vec![Span::from(format!(" {}{date}{best}", game.code())).fg(palette.pick(Color::DarkGray))];
    if game.until_copied_fades().is_some() {
        info.push(" · copied".fg(palette.pick(Color::Cyan)));
    }
    frame.render_widget(Line::from(info), Rect { y: info_row, height: 1, ..area });

    let text: Span = if game.until_restart_lapses().is_some() {
        " press r again to restart, any other key to keep playing".fg(palette.pick(Color::Yellow)).bold()
    } else if game.solved() {
        let new_best = if game.new_best() { " new best!" } else { "" };
        format!(" solved in {} moves!{new_best} enter: next level · r replay · ? help", game.moves)
            .fg(palette.pick(Color::Green))
            .bold()
    } else {
        let keys = if game.carrying() { "←/→ carry · space set down · esc put back" } else { "←/→ move · space pick up · ? help" };
        format!(" crossings: {}   moves: {}   {keys}", game.crossings(), game.moves).into()
    };
    frame.render_widget(text, status);

    if view.help {
        render_help(frame, area, palette);
    }
}

// ---------------------------------------------------------------------------------------------
// Title

/// Theme colors only, so the shadow tracks light and dark backgrounds alike.
const TITLE_SHADOW: Color = Color::DarkGray;

/// Lowercase "unfold" in a 3×5 pixel font.
const TITLE_LETTERS: [[&str; 5]; 6] = [
    ["...", "...", "X.X", "X.X", "XXX"],
    ["...", "...", "XX.", "X.X", "X.X"],
    [".XX", "X..", "XX.", "X..", "X.."],
    ["...", "...", "XXX", "X.X", "XXX"],
    ["XX.", ".X.", ".X.", ".X.", "XXX"],
    ["..X", "..X", "XXX", "X.X", "XXX"],
];

fn title_pixel(row: usize, col: usize) -> bool {
    let (letter, x) = (col / 4, col % 4);
    row < 5 && letter < TITLE_LETTERS.len() && x < 3 && TITLE_LETTERS[letter][row].as_bytes()[x] == b'X'
}

/// The title on a slip of paper that starts folded up, unfolds one triangle at a time to reveal
/// the letters, then dissolves away and leaves them.
struct Title {
    started: Instant,
}

impl Title {
    const WIDTH: u16 = 26;
    const HEIGHT: u16 = 5;
    const SEGMENTS: usize = TITLE_LETTERS.len();
    const FIRST_FLIP: Duration = Duration::from_millis(300);
    const FLIP_STEP: Duration = Duration::from_millis(90);
    const FLIP_TIME: Duration = Duration::from_millis(300);
    const DISSOLVE_TIME: Duration = Duration::from_millis(600);

    fn flip_delay(k: usize) -> Duration {
        Self::FIRST_FLIP + Self::FLIP_STEP * (k as u32 - 1)
    }

    fn dissolve_delay() -> Duration {
        Self::flip_delay(2 * Self::SEGMENTS - 1) + Self::FLIP_TIME + Duration::from_millis(200)
    }

    fn animating(&self) -> bool {
        self.started.elapsed() < Self::dissolve_delay() + Self::DISSOLVE_TIME
    }

    fn unfolded(&self, k: usize) -> f64 {
        if k == 0 { 1.0 } else { progress(self.started, Self::flip_delay(k), Self::FLIP_TIME) }
    }

    /// The flat slip as a zigzag strip of triangles, each sharing an edge with the one before.
    fn strip() -> Vec<[Point; 3]> {
        let (w, h) = (Self::WIDTH as f64, Self::HEIGHT as f64);
        let x = |i: usize| 0.5 + i as f64 * (w - 1.0) / Self::SEGMENTS as f64;
        let (top, bottom) = (|i| Point::new(x(i), h - 0.5), |i| Point::new(x(i), 0.5));
        (0..Self::SEGMENTS)
            .flat_map(|i| {
                let (a, b, c, d) = (top(i), bottom(i), top(i + 1), bottom(i + 1));
                if i % 2 == 0 { [[a, b, d], [a, d, c]] } else { [[a, b, c], [b, d, c]] }
            })
            .collect()
    }

    /// Where triangle k is now: each hinge back to the first swings its part of the strip over,
    /// alternating directions like an accordion.
    fn pose(&self, strip: &[[Point; 3]], k: usize) -> [Point; 3] {
        strip[k].map(|p| {
            let mut v = [p.x, p.y, 0.0];
            for j in (1..=k).rev() {
                let shared: Vec<Point> = strip[j].iter().copied().filter(|q| strip[j - 1].contains(q)).collect();
                let direction = if j % 2 == 0 { 1.0 } else { -1.0 };
                v = rotate_about(v, (shared[0], shared[1]), direction * PI * (1.0 - ease(self.unfolded(j))));
            }
            Point::new(v[0], v[1])
        })
    }

    /// Whether p shows a letter yet: its triangle has landed flat and no folded flap covers it.
    fn revealed(&self, strip: &[[Point; 3]], now: &[[Point; 3]], p: Point) -> bool {
        let Some(k) = strip.iter().position(|t| touches(t, p)) else { return false };
        self.unfolded(k) >= 1.0 && !(k + 1..strip.len()).any(|j| self.unfolded(j) < 1.0 && inside(&now[j], p))
    }

    fn render(&self, frame: &mut Frame, area: Rect, palette: Palette) {
        if area.width < Self::WIDTH || area.height < Self::HEIGHT {
            return;
        }
        let strip = Self::strip();
        let now: Vec<[Point; 3]> = (0..strip.len()).map(|k| self.pose(&strip, k)).collect();
        let dissolve = progress(self.started, Self::dissolve_delay(), Self::DISSOLVE_TIME);
        if dissolve < 1.0 {
            let paper = Flaps {
                creases: now
                    .iter()
                    .enumerate()
                    .flat_map(|(k, t)| (0..3).map(move |i| Crease { p: t[i], q: t[(i + 1) % 3], top_flap: k, shared: true }))
                    .collect(),
                tris: now.clone(),
                color: Color::Reset,
                dissolve,
                is_shadow: false,
                crossings: Vec::new(),
                crossed_color: Color::Reset,
            };
            let canvas = Canvas::default()
                .x_bounds([0.0, Self::WIDTH as f64])
                .y_bounds([0.0, Self::HEIGHT as f64])
                .paint(|ctx| ctx.draw(&paper));
            frame.render_widget(canvas, area);
        }

        // letter pixels are drawn two to a cell with half blocks; the shadow sits one pixel down and right
        let text = |r: usize, c: usize| title_pixel(r, c);
        let shadow = |r: usize, c: usize| palette.color && !text(r, c) && r > 0 && c > 0 && text(r - 1, c - 1);
        let buf = frame.buffer_mut();
        for row in 1..Self::HEIGHT - 1 {
            for col in 1..Self::WIDTH - 1 {
                let center = Point::new(col as f64 + 0.5, (Self::HEIGHT - row) as f64 - 0.5);
                if !self.revealed(&strip, &now, center) {
                    continue;
                }
                let (r, c) = (2 * (row as usize - 1), col as usize - 1);
                let (ink, shade) = (Color::Reset, TITLE_SHADOW);
                let (glyph, fg, bg) = match ((text(r, c), shadow(r, c)), (text(r + 1, c), shadow(r + 1, c))) {
                    ((true, _), (true, _)) => ('█', ink, Color::Reset),
                    ((true, _), (_, true)) => ('▀', ink, shade),
                    ((_, true), (true, _)) => ('▄', ink, shade),
                    ((true, _), _) => ('▀', ink, Color::Reset),
                    (_, (true, _)) => ('▄', ink, Color::Reset),
                    ((_, true), (_, true)) => ('█', shade, Color::Reset),
                    ((_, true), _) => ('▀', shade, Color::Reset),
                    (_, (_, true)) => ('▄', shade, Color::Reset),
                    _ => continue,
                };
                buf[(area.x + col, area.y + row)].set_char(glyph).set_fg(fg).set_bg(bg);
            }
        }
    }
}

/// Rotates a point about the axis through a hinge lying flat in the plane.
fn rotate_about(v: [f64; 3], (a, b): (Point, Point), angle: f64) -> [f64; 3] {
    let axis = (b - a) * (1.0 / a.dist(b));
    let rel = [v[0] - a.x, v[1] - a.y, v[2]];
    let (sin, cos) = angle.sin_cos();
    let along = (axis.x * rel[0] + axis.y * rel[1]) * (1.0 - cos);
    let turned = [axis.y * rel[2], -axis.x * rel[2], axis.x * rel[1] - axis.y * rel[0]];
    [
        a.x + rel[0] * cos + turned[0] * sin + axis.x * along,
        a.y + rel[1] * cos + turned[1] * sin + axis.y * along,
        rel[2] * cos + turned[2] * sin,
    ]
}

/// Inside or on the border.
fn touches(&[a, b, c]: &[Point; 3], p: Point) -> bool {
    let s = [side(a, b, p), side(b, c, p), side(c, a, p)];
    let eps = 1e-9;
    s.iter().all(|&d| d >= -eps) || s.iter().all(|&d| d <= eps)
}

// ---------------------------------------------------------------------------------------------
// Clipboard

/// Sends an OSC 52 escape, which most terminals take as a clipboard write and which also works
/// over ssh, then tries the system's clipboard tool for terminals that ignore it (like macOS
/// Terminal). Both fail silently: there's no way to tell whether the escape worked anyway.
fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut out = std::io::stdout();
    let _ = write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes())).and_then(|()| out.flush());
    let tools: &[(&str, &[&str])] = &[("pbcopy", &[]), ("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])];
    for (tool, args) in tools {
        let child = Command::new(tool).args(*args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
        if let Ok(mut child) = child {
            let _ = child.stdin.take().unwrap().write_all(text.as_bytes());
            let _ = child.wait();
            return;
        }
    }
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            let c = if i <= chunk.len() { ALPHABET[(n >> (18 - 6 * i) & 63) as usize] } else { b'=' };
            out.push(c as char);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Main loop

fn main() -> std::io::Result<()> {
    let Some((seed, level, day)) = parse_args(std::env::args().skip(1)) else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    ratatui::run(|terminal| app(terminal, seed, level, day, Palette::from_env()))
}

/// The seed, the level if one was given, and the day when the seed is a daily one.
fn parse_args(args: impl Iterator<Item = String>) -> Option<(Seed, Option<usize>, Option<u64>)> {
    let (mut seed, mut level) = (None, None);
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let (s, l) = match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--daily" => {
                let day = args.next_if_map(|a| parse_day(&a, today()).ok_or(a)).unwrap_or_else(today);
                (Some((Seed::daily(day), Some(day))), None)
            }
            "--rand" => (Some((Seed::random(), None)), None),
            code if code.contains('/') => parse_code(code).map(|(s, l)| (Some((s, None)), Some(l)))?,
            n => (None, Some(parse_level(n)?)),
        };
        if let Some(s) = s
            && seed.replace(s).is_some()
        {
            return None;
        }
        level = l.or(level);
    }
    let (seed, day) = seed.unwrap_or_else(|| (Seed::daily(today()), Some(today())));
    // a pasted code from today's daily gets its date too
    let day = day.or_else(|| (seed == Seed::daily(today())).then(today));
    Some((seed, level, day))
}

/// Shelves the current level and brings back the one `step` away, or starts it fresh if it's
/// never been played.
fn jump(shelf: &mut BTreeMap<usize, Game>, game: &mut Game, step: isize, scores: &Scores) {
    let level = game.level().strict_add_signed(step);
    let mut next = shelf.remove(&level).unwrap_or_else(|| Game::new(game.seed, level, scores));
    next.touched_at = Instant::now();
    shelf.insert(game.level(), std::mem::replace(game, next));
}

fn app(terminal: &mut DefaultTerminal, seed: Seed, level: Option<usize>, day: Option<u64>, palette: Palette) -> std::io::Result<()> {
    let mut scores = Scores::load();
    let level = level.unwrap_or_else(|| scores.first_unsolved(seed));
    let mut game = Game::new(seed, level, &scores);
    let date = day.map(format_day);
    let mut view = View { title: Title { started: Instant::now() }, palette, date, help: false };
    // levels left with [ and ], to come back to as they were
    let mut shelf = BTreeMap::new();
    loop {
        game.tick();
        terminal.draw(|frame| render(frame, &game, &view))?;
        let wait = if view.title.animating() { Some(FRAME_TIME) } else { game.wake_after() };
        if let Some(wait) = wait
            && !event::poll(wait)?
        {
            continue;
        }
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            // any key closes the help, and does only that, except quitting
            if view.help && !matches!(key.code, KeyCode::Char('q')) {
                view.help = false;
                continue;
            }
            // any key but a second r drops a restart that's waiting
            let restart_asked = game.until_restart_lapses().is_some();
            game.restart_asked_at = None;
            match key.code {
                KeyCode::Esc if game.carrying() => game.put_back(),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('?') => view.help = true,
                KeyCode::Char('c') => game.copy_code(),
                KeyCode::Left | KeyCode::Char('h') => game.step_cursor(1),
                KeyCode::Right | KeyCode::Char('l') => game.step_cursor(-1),
                KeyCode::Char('r') if game.in_progress() && !restart_asked => game.restart_asked_at = Some(Instant::now()),
                KeyCode::Char('r') => game = Game::new(seed, game.level(), &scores),
                KeyCode::Char('[') if game.level() > 1 => jump(&mut shelf, &mut game, -1, &scores),
                KeyCode::Char(']') => jump(&mut shelf, &mut game, 1, &scores),
                KeyCode::Enter | KeyCode::Char(' ') if game.solved() => jump(&mut shelf, &mut game, 1, &scores),
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
