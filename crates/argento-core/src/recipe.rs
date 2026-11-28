//! The recipe: every parameter of an edit.

use serde::{Deserialize, Serialize};

use crate::ops::film::FilmParams;
use crate::ops::filter::FilterParams;
use crate::ops::structure::StructureParams;
use crate::ops::tone::ToneParams;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recipe {
    /// Stage 1: from colour to luminance. A film stock replaces the filter.
    pub filter: FilterParams,
    /// Stages 1 and 4: the film stock, with its tone curve and grain; `None`
    /// is a digital conversion.
    pub film: Option<FilmParams>,
    /// Stage 2: global adjustments.
    pub global: GlobalParams,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalParams {
    pub tone: ToneParams,
    pub structure: StructureParams,
}
