//! The recipe: every parameter of an edit.

use crate::ops::filter::FilterParams;
use crate::ops::structure::StructureParams;
use crate::ops::tone::ToneParams;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Recipe {
    /// Stage 1: from colour to luminance.
    pub filter: FilterParams,
    /// Stage 2: global adjustments.
    pub global: GlobalParams,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlobalParams {
    pub tone: ToneParams,
    pub structure: StructureParams,
}
