//! unfold-core for the browser. Everything crosses as numbers, strings and flat typed arrays,
//! which web/src/lib/unfold.ts turns back into friendlier shapes.

use unfold_core as uc;
use wasm_bindgen::prelude::*;

/// One go at a puzzle, as `uc::Attempt`, with the puzzle's fixed parts alongside.
#[wasm_bindgen]
pub struct Attempt(uc::Attempt);

#[wasm_bindgen]
impl Attempt {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, level: u32) -> Attempt {
        Attempt(uc::Attempt::new(uc::Puzzle::new(uc::Seed(seed & uc::Seed::MASK), level.max(1) as usize)))
    }

    /// A level from a drawn sheet: corners as x, y pairs in order around the outline, and the
    /// triangles cutting it up as three corner indices each.
    #[wasm_bindgen(js_name = fromSheet)]
    pub fn from_sheet(seed: u32, level: u32, corners: &[f64], triangles: &[u32]) -> Result<Attempt, JsError> {
        let n = corners.len() / 2;
        if !corners.len().is_multiple_of(2) || n < 3 {
            return Err(JsError::new("a sheet needs at least three corners, as x, y pairs"));
        }
        if triangles.len() != 3 * (n - 2) || triangles.iter().any(|&v| v as usize >= n) {
            return Err(JsError::new("a sheet of n corners needs n - 2 triangles of corner indices"));
        }
        let shape = corners.chunks(2).map(|xy| uc::Point::new(xy[0], xy[1])).collect();
        let flaps = triangles.chunks(3).map(|t| [t[0] as usize, t[1] as usize, t[2] as usize]).collect();
        let seed = uc::Seed(seed & uc::Seed::MASK);
        Ok(Attempt(uc::Attempt::new(uc::Puzzle::from_sheet(seed, level.max(1) as usize, shape, flaps))))
    }

    #[wasm_bindgen(getter)]
    pub fn seed(&self) -> u32 {
        self.0.puzzle.seed.0
    }

    #[wasm_bindgen(getter)]
    pub fn level(&self) -> u32 {
        self.0.puzzle.level as u32
    }

    /// Like ABC123/9.
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.0.puzzle.code()
    }

    #[wasm_bindgen(getter)]
    pub fn dots(&self) -> u32 {
        self.0.puzzle.dots() as u32
    }

    /// Vertex triples, three numbers per flap, stacked bottom to top.
    pub fn flaps(&self) -> Vec<u32> {
        self.0.puzzle.flaps.iter().flatten().map(|&v| v as u32).collect()
    }

    /// Four numbers per edge: its two vertices, its top flap, and 1 if two flaps share it.
    pub fn edges(&self) -> Vec<u32> {
        self.0.puzzle.edges.iter().flat_map(|e| [e.a as u32, e.b as u32, e.top_flap as u32, e.shared as u32]).collect()
    }

    /// The flat sheet as x, y pairs per vertex, in the puzzle's own units.
    pub fn shape(&self) -> Vec<f64> {
        flatten(&self.0.puzzle.shape)
    }

    /// Each vertex's slot on the circle, slot 0 at 3 o'clock, counterclockwise.
    pub fn slots(&self) -> Vec<u32> {
        self.0.slot_of().iter().map(|&s| s as u32).collect()
    }

    #[wasm_bindgen(js_name = vertexAt)]
    pub fn vertex_at(&self, slot: u32) -> u32 {
        self.0.vertex_at(slot as usize) as u32
    }

    #[wasm_bindgen(getter)]
    pub fn moves(&self) -> u32 {
        self.0.moves() as u32
    }

    #[wasm_bindgen(getter)]
    pub fn solved(&self) -> bool {
        self.0.solved()
    }

    #[wasm_bindgen(getter)]
    pub fn crossings(&self) -> u32 {
        self.0.crossings() as u32
    }

    /// Edge index pairs that cross, two numbers per pair.
    #[wasm_bindgen(js_name = crossingPairs)]
    pub fn crossing_pairs(&self) -> Vec<u32> {
        self.0.puzzle.crossing_pairs(self.0.slot_of()).into_iter().flat_map(|(i, j)| [i as u32, j as u32]).collect()
    }

    #[wasm_bindgen(getter)]
    pub fn carried(&self) -> Option<u32> {
        self.0.carried().map(|v| v as u32)
    }

    #[wasm_bindgen(js_name = pickUp)]
    pub fn pick_up(&mut self, v: u32) {
        if (v as usize) < self.0.puzzle.dots() {
            self.0.pick_up(v as usize);
        }
    }

    /// +1 counterclockwise, -1 clockwise.
    pub fn step(&mut self, dir: i32) {
        self.0.step(dir.signum() as isize);
    }

    /// Whether it counted as a move.
    #[wasm_bindgen(js_name = setDown)]
    pub fn set_down(&mut self) -> bool {
        self.0.set_down()
    }

    /// The dot that was carried.
    #[wasm_bindgen(js_name = putBack)]
    pub fn put_back(&mut self) -> Option<u32> {
        self.0.put_back().map(|v| v as u32)
    }

    /// Where the solved circle unfolds to, x, y pairs per vertex within the unit circle.
    #[wasm_bindgen(js_name = unfoldTarget)]
    pub fn unfold_target(&self) -> Vec<f64> {
        flatten(&self.0.unfold_target())
    }

    /// The sheet the way up it was drawn, x, y pairs per vertex within the unit circle.
    pub fn upright(&self) -> Vec<f64> {
        flatten(&self.0.puzzle.upright())
    }
}

/// The seed for a day since the Unix epoch.
#[wasm_bindgen(js_name = dailySeed)]
pub fn daily_seed(day: u32) -> u32 {
    uc::Seed::daily(day as u64).0
}

/// A code like ABC123/9 as [seed, level], or nothing if it isn't one.
#[wasm_bindgen(js_name = parseCode)]
pub fn parse_code(code: &str) -> Option<Vec<u32>> {
    uc::parse_code(code.trim()).map(|(seed, level)| vec![seed.0, level as u32])
}

#[wasm_bindgen(js_name = firstLevelDots)]
pub fn first_level_dots() -> u32 {
    uc::LEVEL_1_DOTS as u32
}

fn flatten(points: &[uc::Point]) -> Vec<f64> {
    points.iter().flat_map(|p| [p.x, p.y]).collect()
}
