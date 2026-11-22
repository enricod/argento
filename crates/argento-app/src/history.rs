//! Undo and redo, as stacks of recipes.
//!
//! The history holds the last recipe committed to it. The current recipe may
//! have moved on since (a slider still being dragged): undoing first commits
//! it, so that redo can bring it back.

use argento_core::recipe::Recipe;

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Recipe>,
    redo: Vec<Recipe>,
    committed: Recipe,
}

impl History {
    /// A history with no steps, starting from `recipe`.
    pub fn starting_at(recipe: Recipe) -> Self {
        Self {
            committed: recipe,
            ..Self::default()
        }
    }

    /// Records `current` as a step, unless nothing changed since the last one.
    pub fn commit(&mut self, current: Recipe) {
        if current != self.committed {
            self.undo
                .push(std::mem::replace(&mut self.committed, current));
            self.redo.clear();
        }
    }

    /// The recipe before `current`, if any.
    pub fn undo(&mut self, current: Recipe) -> Option<Recipe> {
        self.commit(current);
        let previous = self.undo.pop()?;
        self.redo
            .push(std::mem::replace(&mut self.committed, previous));
        Some(previous)
    }

    /// The recipe undone last, if `current` has not changed since.
    pub fn redo(&mut self, current: Recipe) -> Option<Recipe> {
        self.commit(current);
        let next = self.redo.pop()?;
        self.undo.push(std::mem::replace(&mut self.committed, next));
        Some(next)
    }

    pub fn can_undo(&self, current: Recipe) -> bool {
        !self.undo.is_empty() || current != self.committed
    }

    pub fn can_redo(&self, current: Recipe) -> bool {
        !self.redo.is_empty() && current == self.committed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(c: f32) -> Recipe {
        let mut r = Recipe::default();
        r.global.tone.contrast = c;
        r
    }

    #[test]
    fn nothing_to_undo_at_first() {
        let mut h = History::default();
        assert!(!h.can_undo(Recipe::default()));
        assert!(!h.can_redo(Recipe::default()));
        assert_eq!(h.undo(Recipe::default()), None);
    }

    #[test]
    fn undo_and_redo_walk_the_steps() {
        let mut h = History::default();
        h.commit(contrast(0.1));
        h.commit(contrast(0.2));
        assert_eq!(h.undo(contrast(0.2)), Some(contrast(0.1)));
        assert_eq!(h.undo(contrast(0.1)), Some(Recipe::default()));
        assert_eq!(h.undo(Recipe::default()), None);
        assert_eq!(h.redo(Recipe::default()), Some(contrast(0.1)));
        assert_eq!(h.redo(contrast(0.1)), Some(contrast(0.2)));
        assert_eq!(h.redo(contrast(0.2)), None);
    }

    #[test]
    fn unchanged_commits_are_not_steps() {
        let mut h = History::default();
        h.commit(contrast(0.1));
        h.commit(contrast(0.1));
        assert_eq!(h.undo(contrast(0.1)), Some(Recipe::default()));
        assert!(!h.can_undo(Recipe::default()));
    }

    #[test]
    fn undo_keeps_an_uncommitted_change_for_redo() {
        let mut h = History::default();
        assert!(h.can_undo(contrast(0.3)));
        assert_eq!(h.undo(contrast(0.3)), Some(Recipe::default()));
        assert_eq!(h.redo(Recipe::default()), Some(contrast(0.3)));
    }

    #[test]
    fn a_new_change_discards_redo() {
        let mut h = History::default();
        h.commit(contrast(0.1));
        h.undo(contrast(0.1));
        assert!(h.can_redo(Recipe::default()));
        assert!(!h.can_redo(contrast(0.5)));
        h.commit(contrast(0.5));
        assert_eq!(h.redo(contrast(0.5)), None);
    }
}
