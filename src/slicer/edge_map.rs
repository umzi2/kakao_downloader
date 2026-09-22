use std::{ ops::IndexMut, path::PathBuf };

use crate::{
    error::{ Error, Result },
    slicer::{ ImageSlicer, ProgressSink, SlicerProgress, image_io::read_luma8 },
};

impl ImageSlicer {
    pub async fn build_edge_map(
        &mut self,
        images: &[PathBuf],
        step: usize,
        progress: ProgressSink<'_>
    ) -> Result<()> {
        if images.len() < 2 {
            return Ok(());
        }

        let total = images.len();

        self.image_heights.clear();

        let mut prev = read_luma8(&images[0]).await?;
        self.image_heights.push(prev.height());

        let mut next = read_luma8(&images[1]).await?;
        self.image_heights.push(next.height());

        let width = prev.width() as usize;

        if prev.width() != next.width() {
            return Err(
                Error::unexpected(
                    format!(
                        "ширина изображений {} и {} различается: {} и {}",
                        images[0].display(),
                        images[1].display(),
                        prev.width(),
                        next.width()
                    )
                )
            );
        }

        let mut y_offset = 0;

        progress(SlicerProgress::EdgeMap {
            done: (2).min(total),
            total,
        });

        for i in 1..images.len() {
            let next_next = if i + 1 < images.len() {
                let image = read_luma8(&images[i + 1]).await?;
                self.image_heights.push(image.height());
                Some(image)
            } else {
                None
            };

            let prev_height = prev.height() as usize;

            unsafe {
                let prev_ptr = prev.as_ptr();
                let next_ptr = next.as_ptr();

                for y in 1..prev_height {
                    let p = std::slice::from_raw_parts(prev_ptr.add((y - 1) * width), width);

                    let c = std::slice::from_raw_parts(prev_ptr.add(y * width), width);

                    let n = if y + 1 < prev_height {
                        std::slice::from_raw_parts(prev_ptr.add((y + 1) * width), width)
                    } else {
                        std::slice::from_raw_parts(next_ptr, width)
                    };

                    let r = self.edge_map.index_mut(y_offset + y - 1);

                    for x in (1..width - 1).step_by(step) {
                        let pl = *p.get_unchecked(x - 1) as f32;
                        let pc = *p.get_unchecked(x) as f32;
                        let pr = *p.get_unchecked(x + 1) as f32;

                        let cl = *c.get_unchecked(x - 1) as f32;
                        let cr = *c.get_unchecked(x + 1) as f32;

                        let nl = *n.get_unchecked(x - 1) as f32;
                        let nc = *n.get_unchecked(x) as f32;
                        let nr = *n.get_unchecked(x + 1) as f32;

                        let gx = pr - pl + 2.0 * (cr - cl) + nr - nl;

                        let gy = pl + 2.0 * pc + pr - (nl + 2.0 * nc + nr);

                        *r += (gx * gx + gy * gy).sqrt();
                    }
                }
            }

            y_offset += prev_height;

            drop(prev);

            prev = next;

            progress(SlicerProgress::EdgeMap {
                done: (i + 2).min(total),
                total,
            });

            match next_next {
                Some(image) => {
                    next = image;
                }
                None => {
                    break;
                }
            }
        }

        progress(SlicerProgress::EdgeMap { done: total, total });

        Ok(())
    }
}
