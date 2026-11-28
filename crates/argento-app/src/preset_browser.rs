//! The left panel: the built-in presets, grouped by category, each with a
//! thumbnail of the current photo.

use std::rc::Rc;

use adw::prelude::*;
use argento_core::image::RgbImage;
use argento_core::pipeline::Pipeline;
use argento_core::preset::{BUILTIN, Category, Preset};
use argento_core::recipe::Recipe;
use gtk::gdk;

use crate::worker::{PhotoId, Rendered};

/// Longer side of the proxy the thumbnails are rendered on.
pub const THUMBNAIL_SIDE: u32 = 256;

fn title(category: Category) -> &'static str {
    match category {
        Category::Classic => "Classic",
        Category::Structure => "Structure",
        Category::Filters => "Filters",
        Category::Scenes => "Scenes",
    }
}

#[derive(Clone)]
pub struct PresetBrowser {
    root: gtk::ScrolledWindow,
    /// One per category, with its presets.
    grids: Rc<Vec<(gtk::FlowBox, Vec<&'static Preset>)>>,
    /// One per preset, in the order of `BUILTIN`.
    pictures: Rc<Vec<gtk::Picture>>,
}

impl PresetBrowser {
    pub fn new() -> Self {
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let mut grids = Vec::new();
        let mut pictures = Vec::new();
        for category in Category::ALL {
            let grid = gtk::FlowBox::builder()
                .selection_mode(gtk::SelectionMode::Single)
                .activate_on_single_click(true)
                .homogeneous(true)
                .min_children_per_line(2)
                .max_children_per_line(4)
                .column_spacing(6)
                .row_spacing(6)
                .build();
            let presets: Vec<&'static Preset> =
                BUILTIN.iter().filter(|p| p.category == category).collect();
            for preset in &presets {
                let (cell, picture) = cell(preset.name);
                grid.append(&cell);
                pictures.push(picture);
            }
            // Each category folds away on its own, so that the presets the
            // user cares about stay in view.
            let heading = gtk::Label::builder()
                .label(title(category))
                .xalign(0.0)
                .css_classes(["heading"])
                .build();
            let section = gtk::Expander::builder()
                .label_widget(&heading)
                .expanded(true)
                .child(&grid)
                .build();
            content.append(&section);
            grids.push((grid, presets));
        }

        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&content)
            .build();
        root.set_size_request(200, -1);
        Self {
            root,
            grids: Rc::new(grids),
            pictures: Rc::new(pictures),
        }
    }

    /// Calls `chosen` with each preset the user clicks.
    pub fn connect_chosen(&self, chosen: impl Fn(&'static Preset) + 'static) {
        let chosen: Rc<dyn Fn(&'static Preset)> = Rc::new(chosen);
        for (grid, presets) in self.grids.iter() {
            let (this, chosen, presets) = (self.clone(), chosen.clone(), presets.clone());
            grid.connect_child_activated(move |grid, child| {
                // Only one preset is selected across all categories.
                for (other, _) in this.grids.iter().filter(|(other, _)| other != grid) {
                    other.unselect_all();
                }
                chosen(presets[child.index() as usize]);
            });
        }
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.root
    }

    /// Clears the selection, once the recipe no longer matches the preset.
    pub fn unselect(&self) {
        for (grid, _) in self.grids.iter() {
            grid.unselect_all();
        }
    }

    /// Shows the thumbnails made by [`render`], in the order of `BUILTIN`.
    pub fn show(&self, thumbnails: &[gdk::Texture]) {
        for (picture, texture) in self.pictures.iter().zip(thumbnails) {
            picture.set_paintable(Some(texture));
        }
    }
}

/// Renders a thumbnail of each built-in preset, in the order of `BUILTIN`.
/// `small` is a proxy of the photo, about [`THUMBNAIL_SIDE`] pixels long.
pub fn render(photo: PhotoId, small: RgbImage) -> Vec<Rendered> {
    let mut pipeline = Pipeline::new(small);
    BUILTIN
        .iter()
        .map(|preset| {
            let output = pipeline.run(&preset.applied_to(Recipe::default()));
            Rendered {
                photo,
                width: output.width(),
                height: output.height(),
                pixels: output.to_srgb8(),
            }
        })
        .collect()
}

fn cell(name: &str) -> (gtk::Box, gtk::Picture) {
    let picture = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Contain)
        .can_shrink(true)
        .height_request(72)
        .width_request(72)
        .build();
    let label = gtk::Label::builder()
        .label(name)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["caption"])
        .build();
    let cell = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_top(4)
        .margin_bottom(4)
        .margin_start(4)
        .margin_end(4)
        .build();
    cell.append(&picture);
    cell.append(&label);
    (cell, picture)
}
