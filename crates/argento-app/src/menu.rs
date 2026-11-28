//! The main menu, in the header bar: application-wide actions and their
//! keyboard shortcuts.

use adw::prelude::*;
use gtk::gio;

/// Registers the application's actions and every keyboard shortcut.
pub fn install(app: &adw::Application) {
    let shortcuts = gio::ActionEntry::builder("shortcuts")
        .activate(|app: &adw::Application, _, _| show_shortcuts(app))
        .build();
    let about = gio::ActionEntry::builder("about")
        .activate(|app: &adw::Application, _, _| show_about(app))
        .build();
    let quit = gio::ActionEntry::builder("quit")
        .activate(|app: &adw::Application, _, _| app.quit())
        .build();
    app.add_action_entries([shortcuts, about, quit]);

    app.set_accels_for_action("app.shortcuts", &["<Ctrl>question"]);
    app.set_accels_for_action("app.quit", &["<Ctrl>q"]);
    app.set_accels_for_action("win.open", &["<Ctrl>o"]);
    app.set_accels_for_action("win.export", &["<Ctrl>e"]);
    app.set_accels_for_action("win.undo", &["<Ctrl>z"]);
    app.set_accels_for_action("win.redo", &["<Ctrl><Shift>z", "<Ctrl>y"]);
    app.set_accels_for_action("win.compare", &["backslash"]);
}

/// The menu button of the header bar.
pub fn button() -> gtk::MenuButton {
    let menu = gio::Menu::new();
    let photo = gio::Menu::new();
    photo.append(Some("_Open…"), Some("win.open"));
    photo.append(Some("_Export as TIFF"), Some("win.export"));
    menu.append_section(None, &photo);
    let app = gio::Menu::new();
    app.append(Some("_Keyboard Shortcuts"), Some("app.shortcuts"));
    app.append(Some("_About Argento"), Some("app.about"));
    menu.append_section(None, &app);

    gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main Menu")
        .menu_model(&menu)
        .primary(true)
        .build()
}

fn show_shortcuts(app: &adw::Application) {
    let general = adw::ShortcutsSection::new(Some("General"));
    for (title, action) in [
        ("Open a photo", "win.open"),
        ("Export as TIFF", "win.export"),
        ("Keyboard shortcuts", "app.shortcuts"),
        ("Quit", "app.quit"),
    ] {
        general.add(adw::ShortcutsItem::from_action(title, action));
    }
    let editing = adw::ShortcutsSection::new(Some("Editing"));
    for (title, action) in [
        ("Undo", "win.undo"),
        ("Redo", "win.redo"),
        ("Show the original", "win.compare"),
    ] {
        editing.add(adw::ShortcutsItem::from_action(title, action));
    }

    let dialog = adw::ShortcutsDialog::new();
    dialog.add(general);
    dialog.add(editing);
    dialog.present(app.active_window().as_ref());
}

fn show_about(app: &adw::Application) {
    let dialog = adw::AboutDialog::builder()
        .application_name("Argento")
        .application_icon(crate::APP_ID)
        .comments("Black-and-white photo editing")
        .developer_name("Enrico Donelli")
        .version(env!("CARGO_PKG_VERSION"))
        .website("https://github.com/enricod/argento")
        .issue_url("https://github.com/enricod/argento/issues")
        .license_type(gtk::License::Gpl30)
        .build();
    dialog.present(app.active_window().as_ref());
}
