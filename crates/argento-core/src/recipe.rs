//! The recipe: every parameter of an edit.

use crate::ops::filter::FilterParams;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Recipe {
    /// Stage 1: from colour to luminance.
    pub filter: FilterParams,
}
