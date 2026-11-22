//! Changes to the recipe, from the controls, the presets and the history,
//! and saving them in the photo's sidecar.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use argento_core::preset::Preset;
use argento_core::recipe::Recipe;
use argento_io::sidecar;
use gtk::{gio, glib};

use crate::adjustments::Adjustments;
use crate::history::History;
use crate::preset_browser::PresetBrowser;
use crate::worker::Worker;

/// How long the recipe must stay still before a change becomes a step of
/// the history and is saved: a slider drag is one step, not hundreds.
const SETTLE: Duration = Duration::from_millis(600);

#[derive(Clone)]
pub struct Editor {
    recipe: Rc<RefCell<Recipe>>,
    history: Rc<RefCell<History>>,
    /// Pending commit of the latest change.
    settling: Rc<RefCell<Option<glib::SourceId>>>,
    /// The photo being edited.
    photo: Rc<RefCell<Option<PathBuf>>>,
    /// Whether changes are saved in the photo's sidecar: not when its
    /// sidecar could not be read, so as not to overwrite it.
    saving: Rc<Cell<bool>>,
    /// The recipe in the sidecar, as last read or written.
    saved: Rc<Cell<Recipe>>,
    adjustments: Rc<Adjustments>,
    presets: PresetBrowser,
    worker: Rc<Worker>,
    toasts: adw::ToastOverlay,
    undo: gio::SimpleAction,
    redo: gio::SimpleAction,
}

impl Editor {
    /// Wires the editor to the controls and the presets, and adds the undo
    /// and redo actions to `window`.
    pub fn new(
        window: &adw::ApplicationWindow,
        recipe: Rc<RefCell<Recipe>>,
        adjustments: Rc<Adjustments>,
        presets: PresetBrowser,
        worker: Rc<Worker>,
        toasts: adw::ToastOverlay,
    ) -> Self {
        let undo = gio::SimpleAction::new("undo", None);
        let redo = gio::SimpleAction::new("redo", None);
        window.add_action(&undo);
        window.add_action(&redo);
        let start = *recipe.borrow();
        let editor = Editor {
            history: Rc::new(RefCell::new(History::starting_at(start))),
            recipe,
            settling: Rc::default(),
            photo: Rc::default(),
            saving: Rc::default(),
            saved: Rc::default(),
            adjustments: adjustments.clone(),
            presets: presets.clone(),
            worker,
            toasts,
            undo: undo.clone(),
            redo: redo.clone(),
        };
        editor.update_actions();

        let this = editor.clone();
        adjustments.connect_changed(move |_| this.edited());
        let this = editor.clone();
        presets.connect_chosen(move |preset| this.choose(preset));
        let this = editor.clone();
        undo.connect_activate(move |_, _| this.undo());
        let this = editor.clone();
        redo.connect_activate(move |_, _| this.redo());
        editor
    }

    /// The photo being edited.
    pub fn photo(&self) -> Option<PathBuf> {
        self.photo.borrow().clone()
    }

    pub fn recipe(&self) -> Recipe {
        *self.recipe.borrow()
    }

    /// Switches to `photo`, with the recipe read from its sidecar. The
    /// changes to the previous photo are saved first. `saving` is false when
    /// the sidecar exists but could not be read.
    pub fn open(&self, photo: &Path, recipe: Option<Recipe>, saving: bool) {
        self.commit();
        let recipe = recipe.unwrap_or_default();
        *self.photo.borrow_mut() = Some(photo.to_owned());
        self.saving.set(saving);
        self.saved.set(recipe);
        *self.history.borrow_mut() = History::starting_at(recipe);
        *self.recipe.borrow_mut() = recipe;
        self.adjustments.sync();
        self.presets.unselect();
        self.worker.render(recipe);
        self.update_actions();
    }

    /// Commits and saves any change still settling: before closing.
    pub fn flush(&self) {
        self.commit();
    }

    /// After the user moved a control.
    fn edited(&self) {
        self.presets.unselect();
        self.worker.render(self.recipe());
        self.cancel_commit();
        let this = self.clone();
        let source = glib::timeout_add_local_once(SETTLE, move || {
            this.settling.take();
            this.commit();
        });
        *self.settling.borrow_mut() = Some(source);
        self.update_actions();
    }

    fn choose(&self, preset: &Preset) {
        preset.apply(&mut self.recipe.borrow_mut());
        self.adjustments.sync();
        self.worker.render(self.recipe());
        self.commit();
    }

    fn undo(&self) {
        self.cancel_commit();
        let previous = self.history.borrow_mut().undo(self.recipe());
        if let Some(previous) = previous {
            self.restore(previous);
        }
    }

    fn redo(&self) {
        self.cancel_commit();
        let next = self.history.borrow_mut().redo(self.recipe());
        if let Some(next) = next {
            self.restore(next);
        }
    }

    fn restore(&self, recipe: Recipe) {
        *self.recipe.borrow_mut() = recipe;
        self.adjustments.sync();
        self.presets.unselect();
        self.worker.render(recipe);
        self.update_actions();
        self.save();
    }

    fn commit(&self) {
        self.cancel_commit();
        self.history.borrow_mut().commit(self.recipe());
        self.update_actions();
        self.save();
    }

    /// Writes the recipe to the sidecar, if it changed. A photo that was
    /// never edited gets no sidecar.
    fn save(&self) {
        let recipe = self.recipe();
        if !self.saving.get() || recipe == self.saved.get() {
            return;
        }
        let Some(photo) = self.photo() else {
            return;
        };
        match sidecar::write(&photo, &recipe) {
            Ok(()) => self.saved.set(recipe),
            Err(e) => {
                // Not retried until the next change, so the toast is not
                // repeated at every step.
                self.saved.set(recipe);
                self.toasts
                    .add_toast(adw::Toast::new(&format!("Cannot save the edit: {e}")));
            }
        }
    }

    fn cancel_commit(&self) {
        if let Some(source) = self.settling.take() {
            source.remove();
        }
    }

    fn update_actions(&self) {
        let current = self.recipe();
        let history = self.history.borrow();
        self.undo.set_enabled(history.can_undo(current));
        self.redo.set_enabled(history.can_redo(current));
    }
}
