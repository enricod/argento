//! The right panel: the controls of the recipe, one group per stage.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use argento_core::ops::filter::Filter;
use argento_core::recipe::Recipe;

type Changed = Rc<dyn Fn(Recipe)>;

fn label(filter: Filter) -> &'static str {
    match filter {
        Filter::Neutral => "Neutral",
        Filter::Red => "Red",
        Filter::Orange => "Orange",
        Filter::Yellow => "Yellow",
        Filter::Green => "Green",
        Filter::Blue => "Blue",
    }
}

/// Context shared by the controls' handlers.
#[derive(Clone)]
struct Shared {
    recipe: Rc<RefCell<Recipe>>,
    changed: Changed,
    /// Set while the controls are being brought in line with the recipe:
    /// their handlers then do nothing.
    syncing: Rc<Cell<bool>>,
}

impl Shared {
    fn edit(&self, edit: impl FnOnce(&mut Recipe)) {
        if self.syncing.get() {
            return;
        }
        edit(&mut self.recipe.borrow_mut());
        (self.changed)(*self.recipe.borrow());
    }
}

pub struct Adjustments {
    page: adw::PreferencesPage,
    shared: Shared,
    filter: adw::ComboRow,
    strength: Slider,
    brightness: Slider,
    contrast: Slider,
    structure: Slider,
}

impl Adjustments {
    /// Builds the panel. Each change made by the user is written into
    /// `recipe`, then `changed` is called with the new recipe.
    pub fn new(recipe: Rc<RefCell<Recipe>>, changed: impl Fn(Recipe) + 'static) -> Self {
        let shared = Shared {
            recipe,
            changed: Rc::new(changed),
            syncing: Rc::new(Cell::new(false)),
        };

        // Stage 1
        let labels: Vec<&str> = Filter::ALL.iter().map(|&f| label(f)).collect();
        let filter = adw::ComboRow::builder()
            .title("Filter")
            .model(&gtk::StringList::new(&labels))
            .build();
        let strength = Slider::new("Strength", (0.0, 100.0), &shared, |r, v| {
            r.filter.strength = v
        });
        {
            let (shared, strength) = (shared.clone(), strength.row.clone());
            filter.connect_selected_notify(move |row| {
                let filter = Filter::ALL[row.selected() as usize];
                strength.set_sensitive(filter != Filter::Neutral);
                shared.edit(|r| r.filter.filter = filter);
            });
        }

        // Stage 2
        let brightness = Slider::new("Brightness", (-100.0, 100.0), &shared, |r, v| {
            r.global.tone.brightness = v
        });
        let contrast = Slider::new("Contrast", (-100.0, 100.0), &shared, |r, v| {
            r.global.tone.contrast = v
        });
        let structure = Slider::new("Structure", (-100.0, 100.0), &shared, |r, v| {
            r.global.structure.amount = v
        });

        let page = adw::PreferencesPage::new();
        page.add(&group(
            "Color Filter",
            &[filter.upcast_ref(), strength.row.upcast_ref()],
        ));
        page.add(&group(
            "Tone",
            &[brightness.row.upcast_ref(), contrast.row.upcast_ref()],
        ));
        page.add(&group("Structure", &[structure.row.upcast_ref()]));

        let adjustments = Self {
            page,
            shared,
            filter,
            strength,
            brightness,
            contrast,
            structure,
        };
        adjustments.sync();
        adjustments
    }

    pub fn widget(&self) -> &adw::PreferencesPage {
        &self.page
    }

    /// Brings the controls in line with the recipe after it was changed
    /// elsewhere. `changed` is not called.
    pub fn sync(&self) {
        let r = *self.shared.recipe.borrow();
        self.shared.syncing.set(true);
        let selected = Filter::ALL.iter().position(|&f| f == r.filter.filter);
        self.filter.set_selected(selected.unwrap_or(0) as u32);
        self.strength
            .row
            .set_sensitive(r.filter.filter != Filter::Neutral);
        self.strength.set(r.filter.strength);
        self.brightness.set(r.global.tone.brightness);
        self.contrast.set(r.global.tone.contrast);
        self.structure.set(r.global.structure.amount);
        self.shared.syncing.set(false);
    }
}

/// A row with a slider shown in percent. The recipe holds the value as a
/// fraction (percent / 100).
struct Slider {
    row: adw::ActionRow,
    scale: gtk::Scale,
}

impl Slider {
    /// `set` writes the new fraction into the recipe.
    fn new(
        title: &str,
        (min, max): (f64, f64),
        shared: &Shared,
        set: impl Fn(&mut Recipe, f32) + 'static,
    ) -> Self {
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, 1.0);
        scale.set_draw_value(true);
        scale.set_value_pos(gtk::PositionType::Right);
        scale.set_hexpand(true);
        scale.set_valign(gtk::Align::Center);
        scale.set_size_request(140, -1);
        if min < 0.0 {
            scale.add_mark(0.0, gtk::PositionType::Bottom, None);
        }

        let shared = shared.clone();
        scale.connect_value_changed(move |scale| {
            let value = (scale.value() / 100.0) as f32;
            shared.edit(|r| set(r, value));
        });

        let row = adw::ActionRow::builder().title(title).build();
        row.add_suffix(&scale);
        Self { row, scale }
    }

    fn set(&self, value: f32) {
        self.scale.set_value(f64::from(value) * 100.0);
    }
}

fn group(title: &str, rows: &[&gtk::Widget]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(title).build();
    for row in rows {
        group.add(*row);
    }
    group
}
