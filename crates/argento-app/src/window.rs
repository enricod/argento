//! The main window: presets on the left, the photo in the centre,
//! adjustments on the right.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use argento_core::recipe::Recipe;

use crate::adjustments;
use crate::canvas::Canvas;
use crate::worker::{PROXY_SIDE, PhotoId, Rendered, Worker};

pub fn build(app: &adw::Application) {
    let canvas = Canvas::new();
    let presets = placeholder("Presets");

    // Previews are shown only for this photo: none while a photo is loading.
    let accepted: Rc<Cell<Option<PhotoId>>> = Rc::new(Cell::new(None));
    let worker = {
        let (canvas, accepted) = (canvas.clone(), accepted.clone());
        Rc::new(Worker::spawn(move |preview| {
            if accepted.get() == Some(preview.photo) {
                canvas.show(&texture(preview));
            }
        }))
    };
    let recipe = Rc::new(RefCell::new(Recipe::default()));
    let adjustments = {
        let worker = worker.clone();
        adjustments::build(recipe.clone(), move |recipe| worker.render(recipe))
    };

    let right = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(canvas.widget())
        .end_child(&adjustments)
        .shrink_start_child(false)
        .resize_end_child(false)
        .shrink_end_child(false)
        .position(840)
        .build();
    let body = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&presets)
        .end_child(&right)
        .shrink_end_child(false)
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
        worker,
        recipe,
        accepted,
        displayed: Rc::new(Cell::new(None)),
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

/// Opens photos and hands them to the preview worker.
#[derive(Clone)]
struct Photos {
    window: adw::ApplicationWindow,
    canvas: Canvas,
    title: adw::WindowTitle,
    toasts: adw::ToastOverlay,
    worker: Rc<Worker>,
    recipe: Rc<RefCell<Recipe>>,
    /// The photo whose previews reach the canvas.
    accepted: Rc<Cell<Option<PhotoId>>>,
    /// The last photo opened successfully.
    displayed: Rc<Cell<Option<PhotoId>>>,
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

    /// Decodes the photo and makes its proxy in a background thread, then
    /// sends the proxy to the preview worker.
    fn load(&self, path: PathBuf) {
        let request = self.latest.get() + 1;
        self.latest.set(request);
        self.accepted.set(None);
        self.canvas.show_loading(&file_name(&path));

        let this = self.clone();
        glib::spawn_future_local(async move {
            let decoding = path.clone();
            let result = gio::spawn_blocking(move || {
                argento_io::load::load(&decoding).map(|photo| photo.downscale_to_fit(PROXY_SIDE))
            })
            .await;

            if this.latest.get() != request {
                return;
            }
            let name = file_name(&path);
            match result {
                Ok(Ok(proxy)) => {
                    // The spinner stays until the first preview arrives.
                    let id = this.worker.set_photo(proxy);
                    this.displayed.set(Some(id));
                    this.accepted.set(Some(id));
                    this.title.set_subtitle(&name);
                }
                Ok(Err(e)) => this.failed(&format!("Cannot open {name}: {e}")),
                Err(_) => this.failed(&format!("Cannot open {name}: the loader crashed")),
            }
        });
    }

    /// Goes back to the photo shown before, brought up to date with any
    /// adjustment made while loading.
    fn failed(&self, message: &str) {
        self.accepted.set(self.displayed.get());
        self.canvas.stop_loading();
        if self.displayed.get().is_some() {
            self.worker.render(*self.recipe.borrow());
        }
        self.toast(message);
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }
}

fn texture(preview: Rendered) -> gdk::Texture {
    gdk::MemoryTexture::new(
        preview.width as i32,
        preview.height as i32,
        gdk::MemoryFormat::R8g8b8,
        &glib::Bytes::from_owned(preview.pixels),
        preview.width as usize * 3,
    )
    .upcast()
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
