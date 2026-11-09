use rand::RngExt;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Painter, Shape};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use unfold_core::{Edge, Attempt, Point, Puzzle, Seed, days_from_civil, format_day, inside, intersection, parse_code, parse_day, parse_level, slot_pos, touches};

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

const CROSSED_COLOR: Color = Color::LightRed;
/// In braille dots, which are about square on screen.
const CROSSED_RADIUS: usize = 4;

const USAGE: &str = "\
usage: unfld [level | seed/level] [--daily [date | ±days] | --rand]

  unfld                      today's seed, the same for everyone on the same date
  unfld 9                    level 9 of today's seed
  unfld ABC123/9             level 9 of seed ABC123, to replay a puzzle or share it
  unfld --daily 2026-10-21   that day's seed
  unfld --daily -1           yesterday's seed, or +1 for tomorrow's
  unfld --rand               a random seed

Without a level, play picks up at the first level of the seed not yet solved.";

// ---------------------------------------------------------------------------------------------
// Seeds, scores, palette

fn random_seed() -> Seed {
    Seed(rand::rng().random_range(0..=Seed::MASK))
}

/// Days since the Unix epoch by the local date, so a new daily puzzle comes at local midnight,
/// like the web version's. Falls back to UTC when the time zone can't be found.
fn today() -> u64 {
    let date = jiff::Zoned::now().date();
    days_from_civil(date.year() as u64, date.month() as u64, date.day() as u64)
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

/// An attempt, with everything about showing it in the terminal.
struct Game {
    attempt: Attempt,
    prior_best: Option<usize>,
    unfolded: Vec<Point>,
    cursor: usize,
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
        let attempt = Attempt::new(Puzzle::new(seed, level));
        let n = attempt.puzzle.dots();
        Game {
            attempt,
            prior_best: scores.get(seed, level),
            unfolded: Vec::new(),
            cursor: 0,
            slide: None,
            solved_at: None,
            touched_at: Instant::now(),
            lift: vec![0.0; n],
            ticked_at: Instant::now(),
            copied_at: None,
            restart_asked_at: None,
        }
    }

    fn level(&self) -> usize {
        self.attempt.puzzle.level
    }

    fn code(&self) -> String {
        self.attempt.puzzle.code()
    }

    fn moves(&self) -> usize {
        self.attempt.moves()
    }

    fn slot_of(&self) -> &[usize] {
        self.attempt.slot_of()
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
        !self.solved() && (self.moves() > 0 || self.carrying())
    }

    fn solved(&self) -> bool {
        self.attempt.solved()
    }

    fn new_best(&self) -> bool {
        self.solved() && self.prior_best.is_some_and(|best| self.moves() < best)
    }

    fn best(&self) -> Option<usize> {
        self.solved().then_some(self.moves()).into_iter().chain(self.prior_best).min()
    }

    fn crossings(&self) -> usize {
        self.attempt.crossings()
    }

    /// Only the crossings not hidden under a higher flap.
    fn visible_crossings(&self) -> Vec<Point> {
        let flaps = self.flap_pos();
        self.attempt
            .puzzle
            .crossing_pairs(self.slot_of())
            .into_iter()
            .filter_map(|(e, f)| {
                let pt = intersection(self.edge_pos(e), self.edge_pos(f))?;
                let hidden = flaps[e.top_flap.min(f.top_flap) + 1..].iter().any(|t| inside(t, pt));
                (!hidden).then_some(pt)
            })
            .collect()
    }

    fn vertex_at(&self, slot: usize) -> usize {
        self.attempt.vertex_at(slot)
    }

    /// +1 is counterclockwise, -1 clockwise. A carried dot comes along, trading places with the
    /// dot it steps onto.
    fn step_cursor(&mut self, dir: isize) {
        let n = self.slot_of().len() as isize;
        if self.carrying() {
            self.begin_slide();
            self.attempt.step(dir);
        }
        self.cursor = (self.cursor as isize + dir).rem_euclid(n) as usize;
        self.touched_at = Instant::now();
    }

    /// Picks up the dot under the cursor, or sets the carried one down.
    fn select(&mut self) {
        self.touched_at = Instant::now();
        if !self.carrying() {
            self.attempt.pick_up(self.vertex_at(self.cursor));
        } else if self.attempt.set_down() && self.attempt.solved() {
            self.solved_at = Some(Instant::now());
            self.unfolded = self.attempt.unfold_target();
        }
    }

    fn put_back(&mut self) {
        if !self.carrying() {
            return;
        }
        self.begin_slide();
        let v = self.attempt.put_back().unwrap();
        self.cursor = self.slot_of()[v];
        self.touched_at = Instant::now();
    }

    fn carrying(&self) -> bool {
        self.attempt.carried().is_some()
    }

    /// Call before moving dots, so they glide from where they're drawn, even mid-slide.
    fn begin_slide(&mut self) {
        let from = (0..self.slot_of().len()).map(|v| self.circle_pos(v)).collect();
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
        if self.slot_of()[v] == self.cursor && !self.solved() { 1.0 } else { 0.0 }
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
        self.attempt.puzzle.flaps.iter().map(|f| f.map(|v| self.vertex_pos(v))).collect()
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
        slot_pos(self.slot_of()[v], self.slot_of().len()) * (1.0 + LIFT * ease(self.lift[v]))
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

/// None once a timed notice started at `at` has run its length.
fn time_left(at: Option<Instant>, length: Duration) -> Option<Duration> {
    length.checked_sub(at?.elapsed()).filter(|d| !d.is_zero())
}

fn progress(start: Instant, delay: Duration, length: Duration) -> f64 {
    (start.elapsed().saturating_sub(delay).as_secs_f64() / length.as_secs_f64()).min(1.0)
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
            creases: game.attempt.puzzle
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
        format!(" solved in {} moves!{new_best} enter: next level · r replay · ? help", game.moves())
            .fg(palette.pick(Color::Green))
            .bold()
    } else {
        let keys = if game.carrying() { "←/→ carry · space set down · esc put back" } else { "←/→ move · space pick up · ? help" };
        format!(" crossings: {}   moves: {}   {keys}", game.crossings(), game.moves()).into()
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
            "--rand" => (Some((random_seed(), None)), None),
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
    let mut next = shelf.remove(&level).unwrap_or_else(|| Game::new(game.attempt.puzzle.seed, level, scores));
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
                        scores.record(seed, game.level(), game.moves());
                    }
                }
                _ => {}
            }
        }
    }
}
