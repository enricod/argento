//! Runs a recipe on an image, stage by stage, keeping each stage's output so
//! that a change recomputes only from the stage it touches onwards.

use crate::image::{GrayImage, RgbImage};
use crate::ops::filter::{self, FilterParams};
use crate::recipe::Recipe;

/// A stage's output with the parameters that produced it.
struct Cached<P, T> {
    params: P,
    output: T,
}

pub struct Pipeline {
    source: RgbImage,
    filtered: Option<Cached<FilterParams, GrayImage>>,
    #[cfg(test)]
    filter_runs: usize,
}

impl Pipeline {
    pub fn new(source: RgbImage) -> Self {
        Self {
            source,
            filtered: None,
            #[cfg(test)]
            filter_runs: 0,
        }
    }

    pub fn run(&mut self, recipe: &Recipe) -> RgbImage {
        let stale = self
            .filtered
            .as_ref()
            .is_none_or(|cached| cached.params != recipe.filter);
        if stale {
            #[cfg(test)]
            {
                self.filter_runs += 1;
            }
            self.filtered = Some(Cached {
                params: recipe.filter,
                output: filter::apply(&self.source, &recipe.filter),
            });
        }
        let filtered = &self.filtered.as_ref().expect("stage 1 has just run").output;

        // Toning (stage 5) is not built yet: the grey goes out unchanged.
        filtered.to_rgb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::filter::Filter;

    fn colours() -> RgbImage {
        RgbImage::from_linear(2, 1, vec![0.8, 0.1, 0.1, 0.1, 0.2, 0.8])
    }

    fn red() -> Recipe {
        Recipe {
            filter: FilterParams {
                filter: Filter::Red,
                strength: 1.0,
            },
        }
    }

    #[test]
    fn produces_the_filtered_grey_in_all_three_channels() {
        let mut pipeline = Pipeline::new(colours());
        let out = pipeline.run(&red());
        let grey = filter::apply(&colours(), &red().filter);

        assert_eq!((out.width(), out.height()), (2, 1));
        for (pixel, v) in out.data().chunks(3).zip(grey.data()) {
            assert_eq!(pixel, [*v, *v, *v]);
        }
    }

    #[test]
    fn reuses_the_filter_stage_when_its_parameters_have_not_changed() {
        let mut pipeline = Pipeline::new(colours());
        pipeline.run(&red());
        pipeline.run(&red());
        assert_eq!(pipeline.filter_runs, 1);
    }

    #[test]
    fn recomputes_the_filter_stage_when_its_parameters_change() {
        let mut pipeline = Pipeline::new(colours());
        let first = pipeline.run(&red());
        let second = pipeline.run(&Recipe::default());
        assert_eq!(pipeline.filter_runs, 2);
        assert_ne!(first, second);
    }
}
