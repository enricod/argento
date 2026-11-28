//! Runs a recipe on an image, stage by stage, keeping each stage's output so
//! that a change recomputes only from the stage it touches onwards.

use crate::image::{GrayImage, RgbImage};
use crate::ops::film::{self, FilmParams, Stock};
use crate::ops::filter::{self, FilterParams};
use crate::ops::{structure, tone};
use crate::recipe::{GlobalParams, Recipe};

/// A stage's output with the parameters that produced it: those of the
/// stage and of every stage before it.
struct Cached<P, T> {
    params: P,
    output: T,
}

/// The output kept in `slot` if it was produced with `params`, otherwise a
/// new one from `compute`.
fn cached<P: PartialEq, T>(
    slot: &mut Option<Cached<P, T>>,
    params: P,
    compute: impl FnOnce() -> T,
) -> &T {
    if slot.as_ref().is_none_or(|c| c.params != params) {
        *slot = Some(Cached {
            output: compute(),
            params,
        });
    }
    &slot.as_ref().expect("filled above").output
}

/// How stage 1 turns colour into luminance: with the colour filter, or with
/// the spectral response of a film stock, which replaces the filter. Only
/// the parameters that stage 1 uses are compared, so that a film's grain
/// does not rerun it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Spectrum {
    Filter(FilterParams),
    Film(Stock),
}

impl Spectrum {
    fn of(recipe: &Recipe) -> Self {
        match recipe.film {
            Some(film) => Spectrum::Film(film.stock),
            None => Spectrum::Filter(recipe.filter),
        }
    }
}

/// Everything stage 4 depends on.
type RenderKey = ((Spectrum, GlobalParams), Option<FilmParams>);

pub struct Pipeline {
    source: RgbImage,
    /// Stage 1.
    filtered: Option<Cached<Spectrum, GrayImage>>,
    /// Stage 2.
    adjusted: Option<Cached<(Spectrum, GlobalParams), GrayImage>>,
    /// Stage 4: film curve and grain.
    rendered: Option<Cached<RenderKey, GrayImage>>,
    /// How many times each stage has run, to test the cache.
    #[cfg(test)]
    runs: [usize; 3],
}

impl Pipeline {
    pub fn new(source: RgbImage) -> Self {
        Self {
            source,
            filtered: None,
            adjusted: None,
            rendered: None,
            #[cfg(test)]
            runs: [0; 3],
        }
    }

    pub fn run(&mut self, recipe: &Recipe) -> RgbImage {
        let Self {
            source,
            filtered,
            adjusted,
            rendered,
            ..
        } = self;
        #[cfg(test)]
        let runs = &mut self.runs;

        let spectrum = Spectrum::of(recipe);
        let filtered = cached(filtered, spectrum, || {
            #[cfg(test)]
            {
                runs[0] += 1;
            }
            match spectrum {
                Spectrum::Filter(params) => filter::apply(source, &params),
                Spectrum::Film(stock) => filter::mix(source, stock.spectrum()),
            }
        });
        let adjusted = cached(adjusted, (spectrum, recipe.global), || {
            #[cfg(test)]
            {
                runs[1] += 1;
            }
            let toned = tone::apply(filtered, &recipe.global.tone);
            structure::apply(&toned, &recipe.global.structure)
        });
        let rendered = cached(rendered, ((spectrum, recipe.global), recipe.film), || {
            #[cfg(test)]
            {
                runs[2] += 1;
            }
            match recipe.film {
                None => adjusted.clone(),
                Some(film) => {
                    let curved = tone::apply(adjusted, &film.stock.curve());
                    film::grain(&curved, &film)
                }
            }
        });

        // Toning (stage 5) is not built yet: the grey goes out unchanged.
        rendered.to_rgb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::filter::Filter;
    use crate::ops::structure::StructureParams;
    use crate::ops::tone::ToneParams;

    fn colours() -> RgbImage {
        RgbImage::from_linear(2, 1, vec![0.8, 0.1, 0.1, 0.1, 0.2, 0.8])
    }

    fn red() -> Recipe {
        Recipe {
            filter: FilterParams {
                filter: Filter::Red,
                strength: 1.0,
            },
            ..Recipe::default()
        }
    }

    fn brighter(mut recipe: Recipe) -> Recipe {
        recipe.global.tone.brightness = 0.5;
        recipe
    }

    fn with_film(mut recipe: Recipe, film: FilmParams) -> Recipe {
        recipe.film = Some(film);
        recipe
    }

    #[test]
    fn produces_the_adjusted_grey_in_all_three_channels() {
        let recipe = Recipe {
            global: GlobalParams {
                tone: ToneParams {
                    brightness: 0.3,
                    contrast: 0.2,
                },
                structure: StructureParams { amount: 0.5 },
            },
            ..red()
        };
        let out = Pipeline::new(colours()).run(&recipe);

        let grey = filter::apply(&colours(), &recipe.filter);
        let grey = tone::apply(&grey, &recipe.global.tone);
        let grey = structure::apply(&grey, &recipe.global.structure);
        assert_eq!((out.width(), out.height()), (2, 1));
        for (pixel, v) in out.data().chunks(3).zip(grey.data()) {
            assert_eq!(pixel, [*v, *v, *v]);
        }
    }

    #[test]
    fn reruns_nothing_when_the_recipe_has_not_changed() {
        let mut pipeline = Pipeline::new(colours());
        pipeline.run(&red());
        pipeline.run(&red());
        assert_eq!(pipeline.runs, [1, 1, 1]);
    }

    #[test]
    fn a_global_change_does_not_rerun_the_filter() {
        let mut pipeline = Pipeline::new(colours());
        let before = pipeline.run(&red());
        let after = pipeline.run(&brighter(red()));
        assert_eq!(pipeline.runs, [1, 2, 2]);
        assert_ne!(before, after);
    }

    #[test]
    fn a_filter_change_reruns_every_stage_after_it() {
        let mut pipeline = Pipeline::new(colours());
        let before = pipeline.run(&red());
        let after = pipeline.run(&Recipe::default());
        assert_eq!(pipeline.runs, [2, 2, 2]);
        assert_ne!(before, after);
    }

    #[test]
    fn a_film_replaces_the_filter_in_stage_one() {
        let mut pipeline = Pipeline::new(colours());
        let film = FilmParams {
            grain: 0.0,
            ..FilmParams::default()
        };
        let with = pipeline.run(&with_film(red(), film));
        let without = Pipeline::new(colours()).run(&Recipe {
            filter: FilterParams::default(),
            ..red()
        });
        // The film's spectrum is what counts: the filter in the recipe is
        // ignored while a film is chosen.
        let filter_free = Pipeline::new(colours()).run(&with_film(Recipe::default(), film));
        assert_eq!(with, filter_free);
        assert_ne!(with, without);
    }

    #[test]
    fn a_grain_change_reruns_only_the_last_stage() {
        let mut pipeline = Pipeline::new(colours());
        let film = FilmParams {
            grain: 0.2,
            ..FilmParams::default()
        };
        pipeline.run(&with_film(red(), film));
        let grainier = FilmParams { grain: 0.8, ..film };
        pipeline.run(&with_film(red(), grainier));
        assert_eq!(pipeline.runs, [1, 1, 2]);
    }

    #[test]
    fn a_film_stock_change_reruns_every_stage() {
        let mut pipeline = Pipeline::new(colours());
        let film = FilmParams {
            grain: 0.0,
            ..FilmParams::default()
        };
        pipeline.run(&with_film(red(), film));
        pipeline.run(&with_film(
            red(),
            FilmParams {
                stock: Stock::Orthochromatic,
                ..film
            },
        ));
        assert_eq!(pipeline.runs, [2, 2, 2]);
    }
}
