//! The main window: presets on the left, the photo in the centre,
//! adjustments on the right.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::canvas::Canvas;

pub fn build(app: &adw::Application) {
    let canvas = Canvas::new();
    let presets = placeholder("Presets");
    let adjustments = placeholder("Adjustments");

    let right = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(canvas.widget())
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

    let title = adw::WindowTitle::new("Argento", "");
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));

    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&body));

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&toasts));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Argento")
        .default_width(1400)
        .default_height(900)
        .content(&toolbar)
        .build();

    let photos = Photos {
        window: window.clone(),
        canvas: canvas.clone(),
        title,
        toasts,
        latest: Rc::new(Cell::new(0)),
    };

    let open = {
        let photos = photos.clone();
        gio::ActionEntry::builder("open")
            .activate(move |_: &adw::ApplicationWindow, _, _| photos.choose())
            .build()
    };
    window.add_action_entries([open]);

    let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    drop.connect_drop(move |_, value, _, _| {
        let path = value
            .get::<gdk::FileList>()
            .ok()
            .and_then(|files| files.files().into_iter().next())
            .and_then(|file| file.path());
        match path {
            Some(path) => {
                photos.load(path);
                true
            }
            None => false,
        }
    });
    canvas.widget().add_controller(drop);

    window.present();
}

/// Opens photos and shows them on the canvas.
#[derive(Clone)]
struct Photos {
    window: adw::ApplicationWindow,
    canvas: Canvas,
    title: adw::WindowTitle,
    toasts: adw::ToastOverlay,
    /// Number of the last load requested: an older load that finishes later
    /// is discarded.
    latest: Rc<Cell<u64>>,
}

impl Photos {
    /// Asks the user for a photo, then loads it.
    fn choose(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Photos (JPEG, PNG, TIFF)"));
        for mime in ["image/jpeg", "image/png", "image/tiff"] {
            filter.add_mime_type(mime);
        }
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder()
            .title("Open Photo")
            .modal(true)
            .filters(&filters)
            .default_filter(&filter)
            .build();

        let this = self.clone();
        glib::spawn_future_local(async move {
            match dialog.open_future(Some(&this.window)).await {
                Ok(file) => match file.path() {
                    Some(path) => this.load(path),
                    None => this.toast("Only local files can be opened"),
                },
                Err(e) if e.matches(gtk::DialogError::Dismissed) => {}
                Err(e) => this.toast(&format!("Cannot open the file chooser: {e}")),
            }
        });
    }

    /// Decodes the photo in a background thread, then shows it.
    fn load(&self, path: PathBuf) {
        let request = self.latest.get() + 1;
        self.latest.set(request);

        let this = self.clone();
        glib::spawn_future_local(async move {
            let decoding = path.clone();
            let result = gio::spawn_blocking(move || {
                argento_io::load::load(&decoding)
                    .map(|image| (image.width(), image.height(), image.to_srgb8()))
            })
            .await;

            if this.latest.get() != request {
                return;
            }
            let name = file_name(&path);
            match result {
                Ok(Ok((width, height, pixels))) => {
                    let texture = gdk::MemoryTexture::new(
                        width as i32,
                        height as i32,
                        gdk::MemoryFormat::R8g8b8,
                        &glib::Bytes::from_owned(pixels),
                        width as usize * 3,
                    );
                    this.canvas.show(texture.upcast_ref());
                    this.title.set_subtitle(&name);
                }
                Ok(Err(e)) => this.toast(&format!("Cannot open {name}: {e}")),
                Err(_) => this.toast(&format!("Cannot open {name}: the loader crashed")),
            }
        });
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn placeholder(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("dim-label");
    label.set_size_request(200, -1);
    label
}
