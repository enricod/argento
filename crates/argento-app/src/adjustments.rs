//! The right panel: the controls of the recipe, one group per stage.

use std::cell::RefCell;
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

/// Builds the panel. Each change is written into `recipe`, then `changed`
/// is called with the new recipe.
pub fn build(recipe: Rc<RefCell<Recipe>>, changed: impl Fn(Recipe) + 'static) -> adw::PreferencesPage {
    let changed: Changed = Rc::new(changed);
    let current = *recipe.borrow();

    // Stage 1
    let labels: Vec<&str> = Filter::ALL.iter().map(|&f| label(f)).collect();
    let filter_row = adw::ComboRow::builder()
        .title("Filter")
        .model(&gtk::StringList::new(&labels))
        .build();
    let selected = Filter::ALL.iter().position(|&f| f == current.filter.filter);
    filter_row.set_selected(selected.unwrap_or(0) as u32);

    let strength_row = slider(
        "Strength",
        (0.0, 100.0),
        current.filter.strength,
        &recipe,
        &changed,
        |r, v| r.filter.strength = v,
    );
    strength_row.set_sensitive(current.filter.filter != Filter::Neutral);
    {
        let (recipe, changed, strength_row) = (recipe.clone(), changed.clone(), strength_row.clone());
        filter_row.connect_selected_notify(move |row| {
            let filter = Filter::ALL[row.selected() as usize];
            strength_row.set_sensitive(filter != Filter::Neutral);
            recipe.borrow_mut().filter.filter = filter;
            changed(*recipe.borrow());
        });
    }
    let filter_group = group("Color Filter", &[filter_row.upcast_ref(), strength_row.upcast_ref()]);

    // Stage 2
    let tone = current.global.tone;
    let brightness = slider("Brightness", (-100.0, 100.0), tone.brightness, &recipe, &changed, |r, v| {
        r.global.tone.brightness = v
    });
    let contrast = slider("Contrast", (-100.0, 100.0), tone.contrast, &recipe, &changed, |r, v| {
        r.global.tone.contrast = v
    });
    let tone_group = group("Tone", &[brightness.upcast_ref(), contrast.upcast_ref()]);

    let structure = slider(
        "Structure",
        (-100.0, 100.0),
        current.global.structure.amount,
        &recipe,
        &changed,
        |r, v| r.global.structure.amount = v,
    );
    let structure_group = group("Structure", &[structure.upcast_ref()]);

    let page = adw::PreferencesPage::new();
    page.add(&filter_group);
    page.add(&tone_group);
    page.add(&structure_group);
    page
}

/// A row with a slider shown in percent. The recipe holds `value` as a
/// fraction (percent / 100); `set` writes the new fraction into the recipe.
fn slider(
    title: &str,
    (min, max): (f64, f64),
    value: f32,
    recipe: &Rc<RefCell<Recipe>>,
    changed: &Changed,
    set: impl Fn(&mut Recipe, f32) + 'static,
) -> adw::ActionRow {
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, 1.0);
    scale.set_value(f64::from(value) * 100.0);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk::PositionType::Right);
    scale.set_hexpand(true);
    scale.set_valign(gtk::Align::Center);
    scale.set_size_request(140, -1);
    if min < 0.0 {
        scale.add_mark(0.0, gtk::PositionType::Bottom, None);
    }

    let (recipe, changed) = (recipe.clone(), changed.clone());
    scale.connect_value_changed(move |scale| {
        set(&mut recipe.borrow_mut(), (scale.value() / 100.0) as f32);
        changed(*recipe.borrow());
    });

    let row = adw::ActionRow::builder().title(title).build();
    row.add_suffix(&scale);
    row
}

fn group(title: &str, rows: &[&gtk::Widget]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(title).build();
    for row in rows {
        group.add(*row);
    }
    group
}
