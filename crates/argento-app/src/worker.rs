//! The preview thread: runs the pipeline on the proxy away from the UI.
//!
//! Only the latest request counts: requests that pile up while a render is
//! running are merged, and only the newest photo and recipe are rendered.

use std::cell::Cell;
use std::sync::mpsc;
use std::thread;

use argento_core::image::RgbImage;
use argento_core::pipeline::Pipeline;
use argento_core::recipe::Recipe;
use gtk::prelude::*;
use gtk::{gdk, glib};

/// Longer side of the proxy the preview is rendered on.
pub const PROXY_SIDE: u32 = 2048;

/// Identifies a photo given to the worker. Ids start at 1.
pub type PhotoId = u64;

enum Job {
    Photo(PhotoId, RgbImage),
    Recipe(Recipe),
}

/// A rendered preview, as 8-bit sRGB, three bytes per pixel.
pub struct Rendered {
    /// The photo this preview belongs to.
    pub photo: PhotoId,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rendered {
    pub fn texture(self) -> gdk::Texture {
        let stride = self.width as usize * 3;
        gdk::MemoryTexture::new(
            self.width as i32,
            self.height as i32,
            gdk::MemoryFormat::R8g8b8,
            &glib::Bytes::from_owned(self.pixels),
            stride,
        )
        .upcast()
    }
}

pub struct Worker {
    jobs: mpsc::Sender<Job>,
    last_photo: Cell<PhotoId>,
}

impl Worker {
    /// Starts the thread. `on_render` is called on the UI thread with each
    /// finished preview.
    pub fn spawn(on_render: impl Fn(Rendered) + 'static) -> Self {
        let (jobs, pending) = mpsc::channel();
        let (finished, rendered) = async_channel::unbounded();
        thread::Builder::new()
            .name("preview".into())
            .spawn(move || render_loop(&pending, &finished))
            .expect("cannot start the preview thread");
        glib::spawn_future_local(async move {
            while let Ok(preview) = rendered.recv().await {
                on_render(preview);
            }
        });
        Self {
            jobs,
            last_photo: Cell::new(0),
        }
    }

    /// Replaces the photo, given as its proxy, and renders it with the
    /// current recipe. Returns the id its previews will carry.
    pub fn set_photo(&self, proxy: RgbImage) -> PhotoId {
        let id = self.last_photo.get() + 1;
        self.last_photo.set(id);
        self.send(Job::Photo(id, proxy));
        id
    }

    /// Renders the current photo with a new recipe.
    pub fn render(&self, recipe: Recipe) {
        self.send(Job::Recipe(recipe));
    }

    fn send(&self, job: Job) {
        // The thread only stops if it panicked; the preview then just stops
        // updating.
        let _ = self.jobs.send(job);
    }
}

fn render_loop(pending: &mpsc::Receiver<Job>, finished: &async_channel::Sender<Rendered>) {
    let mut pipeline = None;
    let mut photo = 0;
    let mut recipe = Recipe::default();
    while let Ok(job) = pending.recv() {
        for job in std::iter::once(job).chain(pending.try_iter()) {
            match job {
                Job::Photo(id, proxy) => {
                    photo = id;
                    pipeline = Some(Pipeline::new(proxy));
                }
                Job::Recipe(newer) => recipe = newer,
            }
        }
        let Some(pipeline) = pipeline.as_mut() else {
            continue;
        };
        let output = pipeline.run(&recipe);
        let preview = Rendered {
            photo,
            width: output.width(),
            height: output.height(),
            pixels: output.to_srgb8(),
        };
        if finished.send_blocking(preview).is_err() {
            return;
        }
    }
}
