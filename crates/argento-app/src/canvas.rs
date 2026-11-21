//! The central panel: the photo, or a hint on how to open one.

use adw::prelude::*;
use gtk::gdk;

#[derive(Clone)]
pub struct Canvas {
    root: gtk::Stack,
    picture: gtk::Picture,
    loading: adw::StatusPage,
}

impl Canvas {
    pub fn new() -> Self {
        let empty = adw::StatusPage::builder()
            .icon_name("image-x-generic-symbolic")
            .title("Open a Photo")
            .description("Press Ctrl+O or drop a JPEG, PNG or TIFF file here")
            .build();
        let picture = gtk::Picture::builder()
            .content_fit(gtk::ContentFit::Contain)
            .can_shrink(true)
            .build();

        let loading = adw::StatusPage::new();
        loading.set_paintable(Some(&adw::SpinnerPaintable::new(Some(&loading))));

        let root = gtk::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        root.set_size_request(200, -1);
        root.add_named(&empty, Some("empty"));
        root.add_named(&picture, Some("photo"));
        root.add_named(&loading, Some("loading"));
        root.set_visible_child_name("empty");

        Self {
            root,
            picture,
            loading,
        }
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.root
    }

    pub fn show(&self, texture: &gdk::Texture) {
        self.picture.set_paintable(Some(texture));
        self.root.set_visible_child_name("photo");
    }

    /// A spinner in place of the photo while `name` is being opened.
    pub fn show_loading(&self, name: &str) {
        self.loading.set_title(&format!("Opening {name}"));
        self.root.set_visible_child_name("loading");
    }

    /// Back to the photo shown before loading started, or to the hint if
    /// there was none.
    pub fn stop_loading(&self) {
        let page = if self.picture.paintable().is_some() {
            "photo"
        } else {
            "empty"
        };
        self.root.set_visible_child_name(page);
    }
}
