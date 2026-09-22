mod cuts;
mod detect;
mod edge_map;
mod image_io;
mod slice;
mod slice_chapter;

use std::path::PathBuf;

use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub enum SlicerProgress {
    EdgeMap {
        done: usize,
        total: usize,
    },
    Slices {
        done: usize,
        total: usize,
    },
}

pub type ProgressSink<'a> = &'a mut (dyn FnMut(SlicerProgress) + Send);

#[derive(Debug)]
pub struct ImageSlicer {
    edge_map: Vec<f32>,
    processed_height: usize,
    image_heights: Vec<u32>,
}

impl ImageSlicer {
    pub async fn new(
        images: &[PathBuf],
        height: usize,
        step: usize,
        progress: ProgressSink<'_>
    ) -> Result<Self> {
        let processed_height = height - 2;
        let mut slicer = Self {
            edge_map: vec![0.0; processed_height],
            processed_height,
            image_heights: Vec::with_capacity(images.len()),
        };
        slicer.build_edge_map(images, step, progress).await?;
        Ok(slicer)
    }
}
