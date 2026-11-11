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

    /// The flat sheet as x, y pairs per vertex, in the puzzle's own units.
    pub fn shape(&self) -> Vec<f64> {
        flatten(&self.0.puzzle.shape)
    }

    /// Each vertex's slot on the circle, slot 0 at 3 o'clock, counterclockwise.
    pub fn slots(&self) -> Vec<u32> {
        self.0.slot_of().iter().map(|&s| s as u32).collect()
    }

    #[wasm_bindgen(getter)]
    pub fn moves(&self) -> u32 {
        self.0.moves() as u32
    }

    #[wasm_bindgen(getter)]
    pub fn solved(&self) -> bool {
        self.0.solved()
    }

    /// The fewest moves this level can be solved in, from its start.
    #[wasm_bindgen(getter)]
    pub fn par(&self) -> u32 {
        self.0.par() as u32
    }

    #[wasm_bindgen(getter)]
    pub fn crossings(&self) -> u32 {
        self.0.crossings() as u32
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

/// The paper's front and back colors for a seed, as CSS hex colors.
#[wasm_bindgen(js_name = paperColors)]
pub fn paper_colors(seed: u32) -> Vec<String> {
    let (front, back) = uc::Seed(seed & uc::Seed::MASK).paper();
    vec![front.color.to_string(), back.color.to_string()]
}

/// A code like ABC123/9 as [seed, level], or nothing if it isn't one.
#[wasm_bindgen(js_name = parseCode)]
pub fn parse_code(code: &str) -> Option<Vec<u32>> {
    uc::parse_code(code.trim()).map(|(seed, level)| vec![seed.0, level as u32])
}

fn flatten(points: &[uc::Point]) -> Vec<f64> {
    points.iter().flat_map(|p| [p.x, p.y]).collect()
}
