//! The puzzle itself: seeds, level generation, the rules and the geometry under them. Nothing
//! here draws, reads input or looks at a clock, so it builds the same for the terminal and for
//! wasm, and a seed gives the same puzzles in both.

use rand::rngs::Xoshiro256PlusPlus;
use rand::seq::SliceRandom;
use rand::{Rng, RngExt, SeedableRng};
use std::f64::consts::TAU;
use std::fmt;
use std::ops::{Add, Mul, Sub};

pub const LEVEL_1_DOTS: usize = 4;

/// Chance that a new flap grows off the newest one: higher makes snakes, lower makes blobs.
const STRIP_BIAS: f64 = 0.6;

// ---------------------------------------------------------------------------------------------
// Seeds and codes

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Seed(pub u32);

impl Seed {
    pub const MASK: u32 = (1 << 24) - 1;

    /// Every step of the mix can be undone, so no two days share a seed for 45,000 years.
    pub fn daily(day: u64) -> Self {
        let mut x = (day as u32).wrapping_add(0x5EED) & Self::MASK;
        x = x.wrapping_mul(0x9E3779) & Self::MASK;
        x ^= x >> 12;
        x = x.wrapping_mul(0x85EBCB) & Self::MASK;
        x ^= x >> 13;
        Seed(x)
    }

    pub fn parse(s: &str) -> Option<Self> {
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

/// Year, month and day of a day since the epoch, by Howard Hinnant's civil_from_days.
pub fn civil_from_days(day: u64) -> (u64, u64, u64) {
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
pub fn days_from_civil(y: u64, m: u64, d: u64) -> u64 {
    let y = y - (m <= 2) as u64;
    let (era, yoe) = (y / 400, y % 400);
    let doy = (153 * if m > 2 { m - 3 } else { m + 9 } + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe).saturating_sub(719_468)
}

/// Like "2026 Oct 27".
pub fn format_day(day: u64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d) = civil_from_days(day);
    format!("{y} {} {d}", MONTHS[m as usize - 1])
}

/// A date like 2026-10-21, or days from today like -1 or +1.
pub fn parse_day(s: &str, today: u64) -> Option<u64> {
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

pub fn parse_level(s: &str) -> Option<usize> {
    s.parse().ok().filter(|&level| level >= 1)
}

/// A puzzle code like ABC123/9.
pub fn parse_code(s: &str) -> Option<(Seed, usize)> {
    let (seed, level) = s.split_once('/')?;
    Some((Seed::parse(seed)?, parse_level(level)?))
}

// ---------------------------------------------------------------------------------------------
// The puzzle

pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub top_flap: usize,
    /// Shared by two flaps, as opposed to part of the outline.
    pub shared: bool,
}

/// One level of a seed: a flat sheet folded down onto a circle, its dots shuffled around it.
/// Vertices are numbered 0..dots, and a `slot_of` maps each vertex to its slot on the circle.
pub struct Puzzle {
    pub seed: Seed,
    pub level: usize,
    /// Vertex triples, stacked bottom to top like sheets of paper.
    pub flaps: Vec<[usize; 3]>,
    pub edges: Vec<Edge>,
    /// The flat sheet, which the solved circle unfolds into.
    pub shape: Vec<Point>,
    /// The scrambled slots the level starts from.
    pub start: Vec<usize>,
}

impl Puzzle {
    pub fn new(seed: Seed, level: usize) -> Self {
        let mut rng = seed.rng(level);
        let n = level + LEVEL_1_DOTS - 1;
        let (shape, mut flaps) = grow(n, &mut rng);
        flaps.shuffle(&mut rng);
        let mut puzzle = Puzzle { seed, level, edges: edges_of(&flaps), flaps, shape, start: (0..n).collect() };
        // always shuffle at least once: vertices are numbered in growth order, which gives the puzzle away
        loop {
            puzzle.start.shuffle(&mut rng);
            if puzzle.crossings(&puzzle.start) > 0 {
                return puzzle;
            }
        }
    }

    pub fn dots(&self) -> usize {
        self.shape.len()
    }

    pub fn code(&self) -> String {
        format!("{}/{}", self.seed, self.level)
    }

    pub fn crossing_pairs(&self, slot_of: &[usize]) -> Vec<(&Edge, &Edge)> {
        let slots = |e: &Edge| (slot_of[e.a], slot_of[e.b]);
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

    pub fn crossings(&self, slot_of: &[usize]) -> usize {
        self.crossing_pairs(slot_of).len()
    }

    /// The shape turned, and mirrored if the player's outline runs the other way, to best match
    /// the solved circle, then centered and scaled to fit the unit circle.
    pub fn unfold_target(&self, slot_of: &[usize]) -> Vec<Point> {
        let n = self.shape.len();
        let mean = self.shape.iter().fold(Point::ZERO, |s, &p| s + p) * (1.0 / n as f64);
        // as complex numbers, the best rotation is the angle of sum(circle * conj(shape))
        let fit = |flip: f64| {
            let pts: Vec<Point> = self.shape.iter().map(|&p| p - mean).map(|p| Point::new(p.x * flip, p.y)).collect();
            let (re, im) = pts.iter().enumerate().fold((0.0, 0.0), |(re, im), (v, &s)| {
                let c = slot_pos(slot_of[v], n);
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
}

// ---------------------------------------------------------------------------------------------
// Playing

/// A dot picked up and walked around the circle, pushing the others along one slot at a time.
struct Carry {
    v: usize,
    /// Where every dot was at pick up, to put them back.
    before: Vec<usize>,
}

/// A level being played. A move is picking a dot up, carrying it along the circle, and setting
/// it down somewhere new; the puzzle is solved when a move leaves no creases crossing.
pub struct Play {
    pub puzzle: Puzzle,
    slot_of: Vec<usize>,
    carry: Option<Carry>,
    moves: usize,
    solved: bool,
}

impl Play {
    pub fn new(puzzle: Puzzle) -> Self {
        Play { slot_of: puzzle.start.clone(), puzzle, carry: None, moves: 0, solved: false }
    }

    /// Each vertex's slot on the circle.
    pub fn slot_of(&self) -> &[usize] {
        &self.slot_of
    }

    pub fn vertex_at(&self, slot: usize) -> usize {
        self.slot_of.iter().position(|&s| s == slot).unwrap()
    }

    pub fn moves(&self) -> usize {
        self.moves
    }

    pub fn solved(&self) -> bool {
        self.solved
    }

    pub fn crossings(&self) -> usize {
        self.puzzle.crossings(&self.slot_of)
    }

    pub fn carried(&self) -> Option<usize> {
        self.carry.as_ref().map(|c| c.v)
    }

    /// Does nothing once solved or while already carrying.
    pub fn pick_up(&mut self, v: usize) {
        if !self.solved && self.carry.is_none() {
            self.carry = Some(Carry { v, before: self.slot_of.clone() });
        }
    }

    /// +1 is counterclockwise, -1 clockwise. The carried dot trades places with the dot it steps
    /// onto. Does nothing when not carrying.
    pub fn step(&mut self, dir: isize) {
        let Some(v) = self.carried() else { return };
        let n = self.slot_of.len() as isize;
        let w = self.vertex_at((self.slot_of[v] as isize + dir).rem_euclid(n) as usize);
        self.slot_of.swap(v, w);
    }

    /// Whether it counted as a move. Setting a dot down between the same two dots it left changes
    /// nothing, so it doesn't.
    pub fn set_down(&mut self) -> bool {
        let Some(carry) = self.carry.take() else { return false };
        if neighbors(&self.slot_of, carry.v) == neighbors(&carry.before, carry.v) {
            return false;
        }
        self.moves += 1;
        self.solved = self.crossings() == 0;
        true
    }

    /// Returns every dot to where it was at pick up, and the dot that was carried.
    pub fn put_back(&mut self) -> Option<usize> {
        let carry = self.carry.take()?;
        self.slot_of = carry.before;
        Some(carry.v)
    }

    /// Where the solved circle unfolds to, per vertex.
    pub fn unfold_target(&self) -> Vec<Point> {
        self.puzzle.unfold_target(&self.slot_of)
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

/// Chords between circle slots cross when exactly one end of the second lies strictly
/// between the ends of the first.
pub fn chords_cross((a, b): (usize, usize), (c, d): (usize, usize)) -> bool {
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
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point::new(0.0, 0.0);

    pub const fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    pub fn dot(self, o: Point) -> f64 {
        self.x * o.x + self.y * o.y
    }

    pub fn cross(self, o: Point) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Plain sqrt rather than hypot: sqrt is exact everywhere while hypot comes from the system's
    /// math library, and a seed has to generate the same level on every platform.
    pub fn dist(self, o: Point) -> f64 {
        let d = self - o;
        d.dot(d).sqrt()
    }

    pub fn lerp(self, o: Point, t: f64) -> Point {
        self + (o - self) * t
    }

    pub fn rotate(self, angle: f64) -> Point {
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

/// Slot 0 is at 3 o'clock, and slots run counterclockwise.
pub fn slot_pos(slot: usize, n: usize) -> Point {
    let angle = TAU * slot as f64 / n as f64;
    Point::new(angle.cos(), angle.sin())
}

/// Positive when p is left of the line from u to v.
pub fn side(u: Point, v: Point, p: Point) -> f64 {
    (v - u).cross(p - u)
}

/// Strictly inside, so points on the border don't count.
pub fn inside(&[a, b, c]: &[Point; 3], p: Point) -> bool {
    let (d1, d2, d3) = (side(a, b, p), side(b, c, p), side(c, a, p));
    let eps = 1e-9;
    (d1 > eps && d2 > eps && d3 > eps) || (d1 < -eps && d2 < -eps && d3 < -eps)
}

/// Inside or on the border.
pub fn touches(&[a, b, c]: &[Point; 3], p: Point) -> bool {
    let s = [side(a, b, p), side(b, c, p), side(c, a, p)];
    let eps = 1e-9;
    s.iter().all(|&d| d >= -eps) || s.iter().all(|&d| d <= eps)
}

fn segments_cross((a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

pub fn intersection((a, b): (Point, Point), (c, d): (Point, Point)) -> Option<Point> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a over everything a seed decides: the flaps, the starting slots and the exact bits of
    /// the shape.
    fn fingerprint(p: &Puzzle) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mut eat = |x: u64| {
            for b in x.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        };
        p.flaps.iter().flatten().for_each(|&v| eat(v as u64));
        p.start.iter().for_each(|&s| eat(s as u64));
        p.shape.iter().for_each(|p| {
            eat(p.x.to_bits());
            eat(p.y.to_bits());
        });
        h
    }

    /// Recorded from the terminal version before the split. If this fails, seeds and daily
    /// puzzles no longer match what players already saw or shared, on any platform.
    #[test]
    fn seeds_make_the_same_puzzles() {
        let known = [
            (0x000000, 1, 0xb2697debe2fa8861),
            (0x000000, 5, 0x015fda193863ecb4),
            (0xABC123, 9, 0x98b8e47df6bd0032),
            (0xFFFFFF, 20, 0xb1cf9c856490be4c),
            (0xDC86B0, 3, 0xc25d7a263bbf4e32),
            (0x7811DB, 12, 0xdc8f8103018988dc),
        ];
        for (seed, level, hash) in known {
            let p = Puzzle::new(Seed(seed), level);
            assert_eq!(fingerprint(&p), hash, "{}", p.code());
        }
    }

    #[test]
    fn days_make_the_same_seeds() {
        for (day, seed) in [(0, 0x50DC75), (1, 0xB62B7B), (20_000, 0xDC86B0), (20_400, 0x7811DB)] {
            assert_eq!(Seed::daily(day), Seed(seed), "day {day}");
        }
    }

    /// A triangulated polygon has 2n - 3 edges, and every level starts tangled.
    #[test]
    fn levels_are_folded_polygons() {
        for level in 1..=15 {
            let p = Puzzle::new(Seed(0x123456), level);
            assert_eq!(p.dots(), level + LEVEL_1_DOTS - 1);
            assert_eq!(p.edges.len(), 2 * p.dots() - 3);
            assert!(p.crossings(&p.start) > 0);
        }
    }

    /// A dot carried all the way around lands between the same two dots, so it's no move.
    #[test]
    fn going_full_circle_isnt_a_move() {
        let mut play = Play::new(Puzzle::new(Seed(0xABC123), 4));
        let start = play.slot_of().to_vec();
        play.pick_up(2);
        for _ in 0..play.puzzle.dots() - 1 {
            play.step(1);
        }
        assert_ne!(play.slot_of(), start, "the others shifted round by one");
        assert!(!play.set_down());
        assert_eq!(play.moves(), 0);
    }

    #[test]
    fn putting_back_restores_everything() {
        let mut play = Play::new(Puzzle::new(Seed(0xABC123), 4));
        let start = play.slot_of().to_vec();
        play.pick_up(1);
        play.step(-1);
        play.step(-1);
        assert_eq!(play.put_back(), Some(1));
        assert_eq!(play.slot_of(), start);
        assert_eq!((play.moves(), play.carried()), (0, None));
    }

    /// Random play until solved: every move keeps the dots a permutation of the slots, counts
    /// exactly when a dot's neighbors changed, and solving is exactly reaching no crossings.
    #[test]
    fn random_play_reaches_a_solve() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
        for level in 1..=3 {
            let mut play = Play::new(Puzzle::new(Seed(0x5EED00), level));
            let n = play.puzzle.dots();
            for _ in 0..100_000 {
                let v = rng.random_range(0..n);
                let before = neighbors(play.slot_of(), v);
                play.pick_up(v);
                let dir = if rng.random::<bool>() { 1 } else { -1 };
                for _ in 0..rng.random_range(1..n) {
                    play.step(dir);
                }
                let moves = play.moves();
                let counted = play.set_down();
                assert_eq!(counted, neighbors(play.slot_of(), v) != before);
                assert_eq!(play.moves(), moves + counted as usize);
                let mut slots = play.slot_of().to_vec();
                slots.sort();
                assert_eq!(slots, (0..n).collect::<Vec<_>>());
                // unsolved going in, since the loop stops at a solve
                assert_eq!(play.solved(), counted && play.crossings() == 0);
                if play.solved() {
                    break;
                }
            }
            assert!(play.solved(), "level {level} never solved");
            assert_eq!(play.crossings(), 0);
            play.pick_up(0);
            assert_eq!(play.carried(), None, "no picking up once solved");
        }
    }

    #[test]
    fn codes_and_days_parse() {
        assert_eq!(parse_code("abc123/9"), Some((Seed(0xABC123), 9)));
        assert_eq!(parse_code("ABC123/0"), None);
        assert_eq!(parse_code("ABC12/9"), None);
        assert_eq!(Puzzle::new(Seed(0xABC), 2).code(), "000ABC/2");
        let day = parse_day("2026-10-21", 0).unwrap();
        assert_eq!(format_day(day), "2026 Oct 21");
        assert_eq!(parse_day("2026-02-29", 0), None);
        assert_eq!(parse_day("+1", day), Some(day + 1));
        assert_eq!(parse_day("-1", 0), None);
    }
}
