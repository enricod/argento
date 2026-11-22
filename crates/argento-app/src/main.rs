//! Argento: black-and-white photo editing for Linux.

mod adjustments;
mod canvas;
mod editor;
mod histogram;
mod history;
mod preset_browser;
mod window;
mod worker;

use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "io.github.enricod.Argento";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    // Always dark, whatever the system setting: a light interface around
    // the photo distorts how its tones are judged.
    app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
    });
    app.connect_activate(window::build);
    app.set_accels_for_action("win.open", &["<Ctrl>o"]);
    app.set_accels_for_action("win.export", &["<Ctrl>e"]);
    app.set_accels_for_action("win.undo", &["<Ctrl>z"]);
    app.set_accels_for_action("win.redo", &["<Ctrl><Shift>z", "<Ctrl>y"]);
    app.set_accels_for_action("win.compare", &["backslash"]);
    app.run()
}
