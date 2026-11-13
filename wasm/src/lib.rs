//! unfold-core for the browser. Everything crosses as numbers, strings and flat typed arrays,
//! which web/src/lib/unfold.ts turns back into friendlier shapes.

use unfold_core as uc;
use wasm_bindgen::prelude::*;

/// A level's sheet, as `uc::Puzzle`: its flaps and flat shape, which the web folds up itself.
#[wasm_bindgen]
pub struct Puzzle(uc::Puzzle);

#[wasm_bindgen]
impl Puzzle {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, level: u32) -> Puzzle {
        Puzzle(uc::Puzzle::new(uc::Seed(seed & uc::Seed::MASK), level.max(1) as usize))
    }

    /// Like ABC123/9.
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.0.code()
    }

    #[wasm_bindgen(getter)]
    pub fn dots(&self) -> u32 {
        self.0.dots() as u32
    }

    /// Vertex triples, three numbers per flap.
    pub fn flaps(&self) -> Vec<u32> {
        self.0.flaps.iter().flatten().map(|&v| v as u32).collect()
    }

    /// The flat sheet as x, y pairs per vertex, in the puzzle's own units.
    pub fn shape(&self) -> Vec<f64> {
        flatten(&self.0.shape)
    }

    /// The sheet the way up it was drawn, x, y pairs per vertex within the unit circle.
    pub fn upright(&self) -> Vec<f64> {
        flatten(&self.0.upright())
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
