use std::path::{ Path, PathBuf };

use crate::{ error::{ Error, Result }, slicer::{ ImageSlicer, ProgressSink } };

impl ImageSlicer {
    pub async fn slice_chapter(
        images: &[PathBuf],
        out_dir: &Path,
        spread: usize,
        tolerance: usize,
        step: usize,
        height: usize,
        progress: ProgressSink<'_>
    ) -> Result<Vec<PathBuf>> {
        if images.is_empty() {
            return Err(Error::unexpected("нет изображений для нарезки"));
        }

        let slicer = Self::new(images, height, step, progress).await?;
        let cuts = slicer.cut_positions(spread, tolerance);
        slicer.slice_pages(images, &cuts, out_dir, progress).await
    }
}
