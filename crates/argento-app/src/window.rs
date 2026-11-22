//! The main window: presets on the left, the photo in the centre,
//! adjustments on the right.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use argento_core::image::RgbImage;
use argento_core::pipeline::Pipeline;
use argento_core::recipe::Recipe;
use argento_io::sidecar::{self, SidecarError};

use crate::adjustments::Adjustments;
use crate::canvas::Canvas;
use crate::editor::Editor;
use crate::histogram::Histogram;
use crate::preset_browser::{self, PresetBrowser, THUMBNAIL_SIDE};
use crate::worker::{self, PROXY_SIDE, PhotoId, Worker};

pub fn build(app: &adw::Application) {
    let canvas = Canvas::new();
    let histogram = Histogram::new();
    let presets = PresetBrowser::new();

    // Previews are shown only for this photo: none while a photo is loading.
    let accepted: Rc<Cell<Option<PhotoId>>> = Rc::new(Cell::new(None));
    let worker = {
        let (canvas, histogram, accepted) = (canvas.clone(), histogram.clone(), accepted.clone());
        Rc::new(Worker::spawn(move |preview| {
            if accepted.get() == Some(preview.image.photo) {
                canvas.show(preview.image.texture());
                histogram.show(preview.histogram);
            }
        }))
    };
    let recipe = Rc::new(RefCell::new(Recipe::default()));
    let adjustments = Rc::new(Adjustments::new(recipe.clone()));

    let compare = gio::SimpleAction::new_stateful("compare", None, &false.to_variant());
    {
        let canvas = canvas.clone();
        compare.connect_change_state(move |action, state| {
            if let Some(state) = state {
                canvas.set_comparing(state.get().unwrap_or(false));
                action.set_state(state);
            }
        });
    }

    let side = gtk::Box::new(gtk::Orientation::Vertical, 0);
    side.append(histogram.widget());
    adjustments.widget().set_vexpand(true);
    side.append(adjustments.widget());

    let right = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(canvas.widget())
        .end_child(&side)
        .shrink_start_child(false)
        .resize_end_child(false)
        .shrink_end_child(false)
        .position(840)
        .build();
    let body = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(presets.widget())
        .end_child(&right)
        .shrink_end_child(false)
        .resize_start_child(false)
        .shrink_start_child(false)
        .position(240)
        .build();

    let title = adw::WindowTitle::new("Argento", "");
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));
    header.pack_start(&button("edit-undo-symbolic", "Undo", "win.undo"));
    header.pack_start(&button("edit-redo-symbolic", "Redo", "win.redo"));
    let before = gtk::ToggleButton::builder()
        .icon_name("view-dual-symbolic")
        .tooltip_text("Show Original")
        .action_name("win.compare")
        .build();
    header.pack_end(&before);
    let export = gtk::Button::builder()
        .label("Export")
        .tooltip_text("Save as TIFF next to the original")
        .action_name("win.export")
        .build();
    header.pack_end(&export);

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
    window.add_action(&compare);

    let editor = Editor::new(
        &window,
        recipe,
        adjustments,
        presets.clone(),
        worker.clone(),
        toasts.clone(),
    );
    {
        let editor = editor.clone();
        window.connect_close_request(move |_| {
            editor.flush();
            glib::Propagation::Proceed
        });
    }

    let export = gio::SimpleAction::new("export", None);
    export.set_enabled(false);
    window.add_action(&export);

    let photos = Photos {
        window: window.clone(),
        canvas: canvas.clone(),
        presets,
        title,
        toasts,
        worker,
        editor,
        export: export.clone(),
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
    {
        let photos = photos.clone();
        export.connect_activate(move |_, _| photos.export_beside());
    }

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

fn button(icon: &str, tooltip: &str, action: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .action_name(action)
        .build()
}

/// Opens photos, hands them to the preview worker and the editor, and
/// exports them.
#[derive(Clone)]
struct Photos {
    window: adw::ApplicationWindow,
    canvas: Canvas,
    presets: PresetBrowser,
    title: adw::WindowTitle,
    toasts: adw::ToastOverlay,
    worker: Rc<Worker>,
    editor: Editor,
    export: gio::SimpleAction,
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
                argento_io::load::load(&decoding).map(|photo| {
                    let proxy = photo.downscale_to_fit(PROXY_SIDE);
                    let small = proxy.downscale_to_fit(THUMBNAIL_SIDE);
                    let before = (proxy.width(), proxy.height(), proxy.to_srgb8());
                    (proxy, small, before, sidecar::read(&decoding))
                })
            })
            .await;

            if this.latest.get() != request {
                return;
            }
            let name = file_name(&path);
            match result {
                Ok(Ok((proxy, small, (width, height, before), saved))) => {
                    this.open_edit(&path, saved);
                    // The spinner stays until the first preview arrives.
                    this.canvas
                        .set_before(worker::texture(width, height, before));
                    let id = this.worker.set_photo(proxy);
                    this.displayed.set(Some(id));
                    this.accepted.set(Some(id));
                    this.title.set_subtitle(&name);
                    this.thumbnails(id, small);
                    this.export.set_enabled(true);
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
            self.worker.render(self.editor.recipe());
        }
        self.toast(message);
    }

    /// Hands the photo to the editor with the recipe from its sidecar.
    fn open_edit(&self, path: &Path, saved: Result<Option<Recipe>, SidecarError>) {
        match saved {
            Ok(recipe) => self.editor.open(path, recipe, true),
            Err(e) => {
                self.editor.open(path, None, false);
                self.toast(&format!(
                    "Edits to {} will not be saved: {e}",
                    file_name(path)
                ));
            }
        }
    }

    /// Exports the photo being edited as a 16-bit TIFF in its own folder,
    /// replacing the previous export.
    fn export_beside(&self) {
        if let Some(photo) = self.editor.photo() {
            let target = argento_io::export::beside(&photo);
            self.export(photo, target);
        }
    }

    /// Renders `photo` at full resolution with the current recipe and writes
    /// it to `target`, in a background thread.
    fn export(&self, photo: PathBuf, target: PathBuf) {
        let name = file_name(&target);
        if argento_io::export::Format::from_path(&target).is_none() {
            self.toast(&format!("Cannot export {name}: use .jpg, .tif or .png"));
            return;
        }
        let progress = adw::Toast::builder()
            .title(format!("Exporting {name}…"))
            .timeout(0)
            .build();
        self.toasts.add_toast(progress.clone());

        let recipe = self.editor.recipe();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || -> Result<(), String> {
                let source = argento_io::load::load(&photo).map_err(|e| e.to_string())?;
                let output = Pipeline::new(source).run(&recipe);
                argento_io::export::export(&output, &target).map_err(|e| e.to_string())
            })
            .await;
            progress.dismiss();
            match result {
                Ok(Ok(())) => this.toast(&format!("Exported {name}")),
                Ok(Err(e)) => this.toast(&format!("Cannot export {name}: {e}")),
                Err(_) => this.toast(&format!("Cannot export {name}: the export crashed")),
            }
        });
    }

    /// Renders the preset thumbnails of the photo in a background thread.
    fn thumbnails(&self, photo: PhotoId, small: RgbImage) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(thumbnails) =
                gio::spawn_blocking(move || preset_browser::render(photo, small)).await
            else {
                return;
            };
            if this.displayed.get() == Some(photo) {
                let textures: Vec<_> = thumbnails.into_iter().map(|t| t.texture()).collect();
                this.presets.show(&textures);
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
