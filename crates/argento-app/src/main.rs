//! Argento: black-and-white photo editing for Linux.

mod adjustments;
mod canvas;
mod editor;
mod histogram;
mod history;
mod menu;
mod preset_browser;
mod window;
mod worker;

use std::cell::OnceCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use window::MainWindow;

const APP_ID: &str = "io.github.enricod.Argento";

fn main() -> glib::ExitCode {
    gtk::gio::resources_register_include!("argento.gresource")
        .expect("the resources are built into the binary");
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    // Always dark, whatever the system setting: a light interface around
    // the photo distorts how its tones are judged.
    app.connect_startup(|app| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        menu::install(app);
        gtk::Window::set_default_icon_name(APP_ID);
    });

    // Built on first use, by a plain launch or by opening a file.
    let main: Rc<OnceCell<MainWindow>> = Rc::default();
    let window = {
        let main = main.clone();
        move |app: &adw::Application| main.get_or_init(|| window::build(app)).clone()
    };
    {
        let window = window.clone();
        app.connect_activate(move |app| window(app).present());
    }
    // `argento photo.jpg`, or "Open With" in Files. Argento edits one photo
    // at a time: of several files, the first is opened.
    app.connect_open(move |app, files, _| {
        let main = window(app);
        main.present();
        if let Some(path) = files.iter().find_map(|file| file.path()) {
            main.open(path);
        }
    });
    app.run()
}
