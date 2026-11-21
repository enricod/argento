//! The preview thread: runs the pipeline on the proxy away from the UI.
//!
//! Only the latest request counts: requests that pile up while a render is
//! running are merged, and only the newest photo and recipe are rendered.

use std::sync::mpsc;
use std::thread;

use argento_core::image::RgbImage;
use argento_core::pipeline::Pipeline;
use argento_core::recipe::Recipe;
use gtk::glib;

/// Longer side of the proxy the preview is rendered on.
pub const PROXY_SIDE: u32 = 2048;

enum Job {
    Photo(RgbImage),
    Recipe(Recipe),
}

/// A rendered preview, as 8-bit sRGB, three bytes per pixel.
pub struct Rendered {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct Worker {
    jobs: mpsc::Sender<Job>,
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
        Self { jobs }
    }

    /// Replaces the photo, given as its proxy, and renders it with the
    /// current recipe.
    pub fn set_photo(&self, proxy: RgbImage) {
        self.send(Job::Photo(proxy));
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
    let mut recipe = Recipe::default();
    while let Ok(job) = pending.recv() {
        for job in std::iter::once(job).chain(pending.try_iter()) {
            match job {
                Job::Photo(proxy) => pipeline = Some(Pipeline::new(proxy)),
                Job::Recipe(newer) => recipe = newer,
            }
        }
        let Some(pipeline) = pipeline.as_mut() else {
            continue;
        };
        let output = pipeline.run(&recipe);
        let preview = Rendered {
            width: output.width(),
            height: output.height(),
            pixels: output.to_srgb8(),
        };
        if finished.send_blocking(preview).is_err() {
            return;
        }
    }
}
