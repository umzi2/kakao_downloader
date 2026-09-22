use std::{ collections::VecDeque, path::{ Path, PathBuf } };

use image::RgbImage;
use tokio::fs::create_dir_all;

use crate::{
    error::{ Context, Error, Result },
    slicer::{ ImageSlicer, ProgressSink, SlicerProgress, image_io::{ read_rgb8, save_png } },
};

impl ImageSlicer {
    pub async fn slice_pages(
        &self,
        images: &[PathBuf],
        cuts: &[usize],
        out_dir: &Path,
        progress: ProgressSink<'_>
    ) -> Result<Vec<PathBuf>> {
        if images.len() != self.image_heights.len() {
            return Err(
                Error::unexpected(
                    format!(
                        "число путей ({}) не совпадает с числом высот ({})",
                        images.len(),
                        self.image_heights.len()
                    )
                )
            );
        }
        if cuts.len() < 2 {
            return Err(Error::unexpected("нужно минимум две границы нарезки"));
        }

        let heights: Vec<usize> = self.image_heights
            .iter()
            .map(|h| *h as usize)
            .collect();
        let total: usize = heights.iter().sum();

        let bounds: Vec<usize> = cuts
            .iter()
            .map(|c| c + 1)
            .collect();
        if !bounds.windows(2).all(|w| w[0] < w[1]) || bounds[bounds.len() - 1] > total {
            return Err(
                Error::unexpected(
                    format!("границы должны строго возрастать и лежать в пределах {total}")
                )
            );
        }

        create_dir_all(out_dir).await.context(format!("создание {}", out_dir.display()))?;

        let mut window: VecDeque<(usize, RgbImage)> = VecDeque::new();
        let mut window_height = 0usize;
        let mut base = 0usize;
        let mut next = 0usize;
        let mut saved = Vec::new();
        let total_slices = bounds.len() - 1;

        for (i, edges) in bounds.windows(2).enumerate() {
            let (start, end) = (edges[0], edges[1]);

            while let Some((_, image)) = window.front() {
                let height = image.height() as usize;
                if base + height <= start {
                    window.pop_front();
                    base += height;
                    window_height -= height;
                } else {
                    break;
                }
            }

            while base + window_height < end {
                let image = read_rgb8(&images[next]).await?;
                window_height += image.height() as usize;
                window.push_back((next, image));
                next += 1;
            }

            let width = window.front().context("окно изображений пустое")?.1.width() as usize;

            let out_height = end - start;
            let stride = width * 3;
            let mut buffer = vec![0u8; stride * out_height];
            let mut top = base;

            for (index, image) in window.iter() {
                if (image.width() as usize) != width {
                    return Err(
                        Error::unexpected(
                            format!(
                                "ширина {} ({}) отличается от ширины остальных страниц ({width})",
                                images[*index].display(),
                                image.width()
                            )
                        )
                    );
                }
                if (image.height() as usize) != heights[*index] {
                    return Err(
                        Error::unexpected(
                            format!(
                                "высота {} не совпадает с посчитанной",
                                images[*index].display()
                            )
                        )
                    );
                }

                let height = image.height() as usize;
                let from = start.max(top);
                let to = end.min(top + height);

                if from < to {
                    let source = image.as_raw();
                    let dest_offset = (from - start) * stride;
                    let source_offset = (from - top) * stride;
                    let len = (to - from) * stride;
                    buffer[dest_offset..dest_offset + len].copy_from_slice(
                        &source[source_offset..source_offset + len]
                    );
                }

                top += height;
            }

            let path = out_dir.join(format!("{i:04}.png"));
            let image = RgbImage::from_raw(width as u32, out_height as u32, buffer).context(
                format!("буфер фрагмента {}", path.display())
            )?;
            save_png(&path, image).await?;
            saved.push(path);

            progress(SlicerProgress::Slices {
                done: i + 1,
                total: total_slices,
            });

            while let Some((_, image)) = window.front() {
                let height = image.height() as usize;
                if base + height <= end {
                    window.pop_front();
                    base += height;
                    window_height -= height;
                } else {
                    break;
                }
            }
        }

        Ok(saved)
    }
}
