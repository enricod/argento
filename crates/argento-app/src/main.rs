//! Argento: black-and-white photo editing for Linux.

mod canvas;
mod window;

use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "io.github.enricod.Argento";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(window::build);
    app.set_accels_for_action("win.open", &["<Ctrl>o"]);
    app.run()
}
