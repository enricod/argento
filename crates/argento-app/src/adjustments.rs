//! The right panel: the controls of the recipe, one group per stage.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use argento_core::ops::film::{FilmParams, Stock};
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

fn stock_label(stock: Stock) -> &'static str {
    match stock {
        Stock::Panchromatic => "Panchromatic",
        Stock::Orthochromatic => "Orthochromatic",
        Stock::HighContrast => "High contrast",
        Stock::Soft => "Soft",
    }
}

/// Context shared by the controls' handlers.
#[derive(Clone)]
struct Shared {
    recipe: Rc<RefCell<Recipe>>,
    changed: Rc<RefCell<Option<Changed>>>,
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
        let recipe = *self.recipe.borrow();
        if let Some(changed) = self.changed.borrow().as_ref() {
            changed(recipe);
        }
    }
}

/// Which controls take effect for the recipe: a film replaces the colour
/// filter, and grain belongs to a film.
#[derive(Clone)]
struct Sensitive {
    filter: adw::ComboRow,
    strength: adw::ActionRow,
    grain: adw::ActionRow,
    grain_size: adw::ActionRow,
}

impl Sensitive {
    fn update(&self, recipe: &Recipe) {
        let film = recipe.film.is_some();
        self.filter.set_sensitive(!film);
        self.strength
            .set_sensitive(!film && recipe.filter.filter != Filter::Neutral);
        self.grain.set_sensitive(film);
        self.grain_size.set_sensitive(film);
    }
}

pub struct Adjustments {
    page: adw::PreferencesPage,
    shared: Shared,
    sensitive: Sensitive,
    film: adw::ComboRow,
    grain: Slider,
    grain_size: Slider,
    filter: adw::ComboRow,
    strength: Slider,
    brightness: Slider,
    contrast: Slider,
    structure: Slider,
}

impl Adjustments {
    /// Builds the panel. Each change made by the user is written into
    /// `recipe`.
    pub fn new(recipe: Rc<RefCell<Recipe>>) -> Self {
        let shared = Shared {
            recipe,
            changed: Rc::default(),
            syncing: Rc::new(Cell::new(false)),
        };

        // Stage 1 and 4: the film. Its rows come first on the page.
        let stock_labels: Vec<&str> = std::iter::once("None")
            .chain(Stock::ALL.iter().map(|&s| stock_label(s)))
            .collect();
        let film = adw::ComboRow::builder()
            .title("Film")
            .model(&gtk::StringList::new(&stock_labels))
            .build();
        let grain = Slider::new("Grain", (0.0, 100.0), &shared, |r, v| {
            if let Some(film) = r.film.as_mut() {
                film.grain = v;
            }
        });
        let grain_size = Slider::new(
            "Grain size (% of the longer side)",
            (0.0, 0.5),
            &shared,
            |r, v| {
                if let Some(film) = r.film.as_mut() {
                    film.grain_size = v;
                }
            },
        );
        grain_size.scale.set_digits(2);

        // Stage 1: the colour filter, when no film is chosen.
        let labels: Vec<&str> = Filter::ALL.iter().map(|&f| label(f)).collect();
        let filter = adw::ComboRow::builder()
            .title("Filter")
            .model(&gtk::StringList::new(&labels))
            .build();
        let strength = Slider::new("Strength", (0.0, 100.0), &shared, |r, v| {
            r.filter.strength = v
        });

        let sensitive = Sensitive {
            filter: filter.clone(),
            strength: strength.row.clone(),
            grain: grain.row.clone(),
            grain_size: grain_size.row.clone(),
        };

        {
            let (shared, sensitive) = (shared.clone(), sensitive.clone());
            film.connect_selected_notify(move |row| {
                let stock = (row.selected() as usize)
                    .checked_sub(1)
                    .map(|i| Stock::ALL[i]);
                shared.edit(|r| {
                    r.film = stock.map(|stock| FilmParams {
                        stock,
                        ..r.film.unwrap_or_default()
                    })
                });
                sensitive.update(&shared.recipe.borrow());
            });
        }
        {
            let (shared, sensitive) = (shared.clone(), sensitive.clone());
            filter.connect_selected_notify(move |row| {
                let filter = Filter::ALL[row.selected() as usize];
                shared.edit(|r| r.filter.filter = filter);
                sensitive.update(&shared.recipe.borrow());
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
            "Film",
            &[
                film.upcast_ref(),
                grain.row.upcast_ref(),
                grain_size.row.upcast_ref(),
            ],
        ));
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
            sensitive,
            film,
            grain,
            grain_size,
            filter,
            strength,
            brightness,
            contrast,
            structure,
        };
        adjustments.sync();
        adjustments
    }

    /// Calls `changed` with the new recipe after each change made by the
    /// user.
    pub fn connect_changed(&self, changed: impl Fn(Recipe) + 'static) {
        *self.shared.changed.borrow_mut() = Some(Rc::new(changed));
    }

    pub fn widget(&self) -> &adw::PreferencesPage {
        &self.page
    }

    /// Brings the controls in line with the recipe after it was changed
    /// elsewhere. `changed` is not called.
    pub fn sync(&self) {
        let r = *self.shared.recipe.borrow();
        self.shared.syncing.set(true);
        let film_index = r
            .film
            .and_then(|f| Stock::ALL.iter().position(|&s| s == f.stock))
            .map_or(0, |i| i + 1);
        self.film.set_selected(film_index as u32);
        let f = r.film.unwrap_or_default();
        self.grain.set(f.grain);
        self.grain_size.set(f.grain_size);
        let selected = Filter::ALL.iter().position(|&f| f == r.filter.filter);
        self.filter.set_selected(selected.unwrap_or(0) as u32);
        self.strength.set(r.filter.strength);
        self.brightness.set(r.global.tone.brightness);
        self.contrast.set(r.global.tone.contrast);
        self.structure.set(r.global.structure.amount);
        self.sensitive.update(&r);
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
