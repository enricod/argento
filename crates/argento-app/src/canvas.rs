//! The central panel: the photo, or a hint on how to open one.

use adw::prelude::*;
use gtk::gdk;

#[derive(Clone)]
pub struct Canvas {
    root: gtk::Stack,
    picture: gtk::Picture,
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

        let root = gtk::Stack::builder().hexpand(true).vexpand(true).build();
        root.set_size_request(200, -1);
        root.add_named(&empty, Some("empty"));
        root.add_named(&picture, Some("photo"));
        root.set_visible_child_name("empty");

        Self { root, picture }
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.root
    }

    pub fn show(&self, texture: &gdk::Texture) {
        self.picture.set_paintable(Some(texture));
        self.root.set_visible_child_name("photo");
    }
}
