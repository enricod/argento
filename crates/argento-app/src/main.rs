//! Argento: black-and-white photo editing for Linux.

use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "io.github.enricod.Argento";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_window);
    app.run()
}

/// Presets on the left, the image in the centre, adjustments on the right.
fn build_window(app: &adw::Application) {
    let presets = placeholder("Presets");
    let canvas = placeholder("Open a photo to start");
    let adjustments = placeholder("Adjustments");

    let right = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&canvas)
        .end_child(&adjustments)
        .resize_end_child(false)
        .shrink_end_child(false)
        .position(880)
        .build();
    let body = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&presets)
        .end_child(&right)
        .resize_start_child(false)
        .shrink_start_child(false)
        .position(240)
        .build();

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&body));

    adw::ApplicationWindow::builder()
        .application(app)
        .title("Argento")
        .default_width(1400)
        .default_height(900)
        .content(&toolbar)
        .build()
        .present();
}

fn placeholder(text: &str) -> gtk::Widget {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("dim-label");
    label.set_hexpand(true);
    label.set_vexpand(true);
    label.set_size_request(200, -1);
    label.upcast()
}
