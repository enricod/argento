//! The central panel: the photo, or a hint on how to open one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::gdk;

#[derive(Clone)]
pub struct Canvas {
    root: gtk::Stack,
    picture: gtk::Picture,
    loading: adw::StatusPage,
    /// The photo as opened, and the latest preview of the edit.
    before: Rc<RefCell<Option<gdk::Texture>>>,
    after: Rc<RefCell<Option<gdk::Texture>>>,
    /// Whether the photo is shown as opened instead of edited.
    comparing: Rc<Cell<bool>>,
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
            before: Rc::default(),
            after: Rc::default(),
            comparing: Rc::default(),
        }
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.root
    }

    /// Shows a new preview of the edit.
    pub fn show(&self, texture: gdk::Texture) {
        *self.after.borrow_mut() = Some(texture);
        self.refresh();
        self.root.set_visible_child_name("photo");
    }

    /// Sets the photo as opened, shown while comparing. Its first preview
    /// follows with [`Canvas::show`].
    pub fn set_before(&self, texture: gdk::Texture) {
        *self.before.borrow_mut() = Some(texture);
    }

    /// Switches between the photo as opened and the edit.
    pub fn set_comparing(&self, comparing: bool) {
        self.comparing.set(comparing);
        self.refresh();
    }

    fn refresh(&self) {
        let shown = if self.comparing.get() {
            &self.before
        } else {
            &self.after
        };
        self.picture.set_paintable(shown.borrow().as_ref());
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
