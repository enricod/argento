//! The right panel: the controls of the recipe, one group per stage.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use argento_core::ops::filter::Filter;
use argento_core::recipe::Recipe;

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
    let changed = Rc::new(changed);
    let current = *recipe.borrow();

    let labels: Vec<&str> = Filter::ALL.iter().map(|&f| label(f)).collect();
    let filter_row = adw::ComboRow::builder()
        .title("Filter")
        .model(&gtk::StringList::new(&labels))
        .build();
    let selected = Filter::ALL.iter().position(|&f| f == current.filter.filter);
    filter_row.set_selected(selected.unwrap_or(0) as u32);

    let strength = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    strength.set_value(f64::from(current.filter.strength) * 100.0);
    strength.set_draw_value(true);
    strength.set_value_pos(gtk::PositionType::Right);
    strength.set_hexpand(true);
    strength.set_valign(gtk::Align::Center);
    strength.set_size_request(140, -1);
    let strength_row = adw::ActionRow::builder().title("Strength").build();
    strength_row.add_suffix(&strength);
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
    strength.connect_value_changed(move |scale| {
        recipe.borrow_mut().filter.strength = (scale.value() / 100.0) as f32;
        changed(*recipe.borrow());
    });

    let filter_group = adw::PreferencesGroup::builder().title("Color Filter").build();
    filter_group.add(&filter_row);
    filter_group.add(&strength_row);

    let page = adw::PreferencesPage::new();
    page.add(&filter_group);
    page.set_size_request(280, -1);
    page
}
