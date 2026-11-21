//! Presets: ready-made looks, applied on top of the current recipe.
//!
//! A preset is a partial recipe: it sets only the fields it cares about and
//! leaves the others as they are, so that choosing a preset never throws
//! away work it has no opinion on (such as the user's control points, once
//! they exist).

use crate::ops::filter::{Filter, FilterParams};
use crate::ops::structure::StructureParams;
use crate::ops::tone::ToneParams;
use crate::recipe::{GlobalParams, Recipe};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Classic,
    Structure,
    Filters,
    Scenes,
}

impl Category {
    pub const ALL: [Category; 4] = [
        Category::Classic,
        Category::Structure,
        Category::Filters,
        Category::Scenes,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    pub category: Category,
    /// Stage 1; `None` keeps the recipe's filter.
    pub filter: Option<FilterParams>,
    /// Stage 2; `None` keeps the recipe's global adjustments.
    pub global: Option<GlobalParams>,
}

impl Preset {
    /// Writes the preset's fields into `recipe`.
    pub fn apply(&self, recipe: &mut Recipe) {
        if let Some(filter) = self.filter {
            recipe.filter = filter;
        }
        if let Some(global) = self.global {
            recipe.global = global;
        }
    }

    /// `recipe` with the preset applied.
    pub fn applied_to(&self, mut recipe: Recipe) -> Recipe {
        self.apply(&mut recipe);
        recipe
    }
}

/// A preset that sets stages 1 and 2 entirely.
const fn look(
    name: &'static str,
    category: Category,
    filter: Filter,
    strength: f32,
    brightness: f32,
    contrast: f32,
    structure: f32,
) -> Preset {
    Preset {
        name,
        category,
        filter: Some(FilterParams { filter, strength }),
        global: Some(GlobalParams {
            tone: ToneParams {
                brightness,
                contrast,
            },
            structure: StructureParams { amount: structure },
        }),
    }
}

use Category::*;
use Filter::*;

/// The built-in presets, in display order.
#[rustfmt::skip]
pub const BUILTIN: [Preset; 15] = [
    //    name               category    filter   strength bright  contrast structure
    look("Neutral",          Classic,    Neutral, 1.0,      0.0,   0.0,     0.0),
    look("High Contrast",    Classic,    Neutral, 1.0,      0.0,   0.5,     0.15),
    look("Soft Contrast",    Classic,    Neutral, 1.0,      0.05, -0.3,     0.0),
    look("High Key",         Classic,    Neutral, 1.0,      0.45, -0.2,    -0.1),
    look("Low Key",          Classic,    Neutral, 1.0,     -0.4,   0.3,     0.1),
    look("Fine Structure",   Structure,  Neutral, 1.0,      0.0,   0.1,     0.4),
    look("Full Dynamic",     Structure,  Neutral, 1.0,      0.0,   0.2,     0.85),
    look("Smooth",           Structure,  Neutral, 1.0,      0.05, -0.1,    -0.5),
    look("Red Filter",       Filters,    Red,     1.0,      0.0,   0.15,    0.0),
    look("Orange Filter",    Filters,    Orange,  1.0,      0.0,   0.1,     0.0),
    look("Yellow Filter",    Filters,    Yellow,  1.0,      0.0,   0.05,    0.0),
    look("Green Filter",     Filters,    Green,   1.0,      0.0,   0.05,    0.0),
    look("Blue Filter",      Filters,    Blue,    1.0,      0.05,  0.0,     0.0),
    // Red darkens blue skies; strong structure brings out the clouds.
    look("Dramatic Sky",     Scenes,     Red,     1.0,     -0.05,  0.4,     0.5),
    // Orange lightens and evens out skin; less structure softens it.
    look("Soft Portrait",    Scenes,     Orange,  0.7,      0.1,  -0.1,    -0.3),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique() {
        for (i, a) in BUILTIN.iter().enumerate() {
            for b in &BUILTIN[i + 1..] {
                assert_ne!(a.name, b.name);
            }
        }
    }

    #[test]
    fn every_category_has_presets() {
        for category in Category::ALL {
            assert!(
                BUILTIN.iter().any(|p| p.category == category),
                "{category:?}"
            );
        }
    }

    #[test]
    fn presets_of_a_category_are_contiguous() {
        let mut seen = Vec::new();
        for preset in &BUILTIN {
            if seen.last() != Some(&preset.category) {
                assert!(!seen.contains(&preset.category), "{}", preset.name);
                seen.push(preset.category);
            }
        }
    }

    #[test]
    fn values_are_within_the_controls_ranges() {
        for preset in &BUILTIN {
            let r = preset.applied_to(Recipe::default());
            assert!((0.0..=1.0).contains(&r.filter.strength), "{}", preset.name);
            let g = r.global;
            for v in [g.tone.brightness, g.tone.contrast, g.structure.amount] {
                assert!((-1.0..=1.0).contains(&v), "{}", preset.name);
            }
        }
    }

    #[test]
    fn neutral_is_the_default_recipe() {
        assert_eq!(BUILTIN[0].applied_to(Recipe::default()), Recipe::default());
    }

    #[test]
    fn unset_fields_are_kept() {
        let preset = Preset {
            name: "Filter only",
            category: Filters,
            filter: Some(FilterParams {
                filter: Red,
                strength: 0.5,
            }),
            global: None,
        };
        let mut recipe = Recipe::default();
        recipe.global.tone.contrast = 0.3;
        let applied = preset.applied_to(recipe);
        assert_eq!(applied.filter, preset.filter.unwrap());
        assert_eq!(applied.global, recipe.global);
    }
}
