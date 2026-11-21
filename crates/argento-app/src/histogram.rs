//! The luminance histogram of the preview, above the adjustments.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;

pub const BINS: usize = 256;

/// Counts the pixels of each 8-bit level. `pixels` is grey RGB, three
/// equal bytes per pixel, so only the first of each three is read.
pub fn count(pixels: &[u8]) -> [u32; BINS] {
    let mut bins = [0; BINS];
    for pixel in pixels.as_chunks::<3>().0 {
        bins[usize::from(pixel[0])] += 1;
    }
    bins
}

#[derive(Clone)]
pub struct Histogram {
    area: gtk::DrawingArea,
    bins: Rc<RefCell<Option<[u32; BINS]>>>,
}

impl Histogram {
    pub fn new() -> Self {
        let bins: Rc<RefCell<Option<[u32; BINS]>>> = Rc::default();
        let area = gtk::DrawingArea::builder()
            .content_height(96)
            .margin_top(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        let drawn = bins.clone();
        area.set_draw_func(move |area, cr, width, height| {
            draw(
                area,
                cr,
                f64::from(width),
                f64::from(height),
                drawn.borrow().as_ref(),
            );
        });
        Self { area, bins }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.area
    }

    pub fn show(&self, bins: [u32; BINS]) {
        *self.bins.borrow_mut() = Some(bins);
        self.area.queue_draw();
    }
}

fn draw(
    area: &gtk::DrawingArea,
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    bins: Option<&[u32; BINS]>,
) {
    let color = area.color();
    let (r, g, b) = (
        f64::from(color.red()),
        f64::from(color.green()),
        f64::from(color.blue()),
    );

    // Background
    cr.set_source_rgba(r, g, b, 0.06);
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();

    let Some(bins) = bins else {
        return;
    };
    // Scaled to the tallest level between the extremes: clipped blacks or
    // whites would otherwise flatten everything else. Their spikes are cut.
    let tallest = bins[1..BINS - 1].iter().copied().max().unwrap_or(0).max(1);
    let step = width / BINS as f64;
    cr.move_to(0.0, height);
    for (i, &count) in bins.iter().enumerate() {
        let y = height * (1.0 - (f64::from(count) / f64::from(tallest)).min(1.0));
        cr.line_to(i as f64 * step, y);
        cr.line_to((i + 1) as f64 * step, y);
    }
    cr.line_to(width, height);
    cr.close_path();
    cr.set_source_rgba(r, g, b, 0.55);
    let _ = cr.fill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_level() {
        let bins = count(&[0, 0, 0, 255, 255, 255, 255, 255, 255, 7, 7, 7]);
        assert_eq!(bins[0], 1);
        assert_eq!(bins[7], 1);
        assert_eq!(bins[255], 2);
        assert_eq!(bins.iter().sum::<u32>(), 4);
    }
}
