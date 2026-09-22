use crate::slicer::{ ImageSlicer, detect::find_cut_center };

impl ImageSlicer {
    pub fn cut_positions(&self, spread: usize, tolerance: usize) -> Vec<usize> {
        let n = self.processed_height;
        if n == 0 {
            return vec![];
        }
        if n <= spread {
            return vec![0, n];
        }

        let smooth_radius = 4usize;

        let smoothed: Vec<f32> = (0..n)
            .map(|i| {
                let lo = i.saturating_sub(smooth_radius);
                let hi = (i + smooth_radius).min(n - 1);
                let slice = &self.edge_map[lo..=hi];
                slice.iter().sum::<f32>() / (slice.len() as f32)
            })
            .collect();

        let max_val = smoothed.iter().cloned().fold(0.0_f32, f32::max);
        let zero_threshold = (max_val * 0.01).max(1e-6);

        let mut cuts = vec![0usize];
        let mut pos = 0;

        while pos + spread.saturating_sub(tolerance) < n {
            let target = pos + spread;
            let lo = (pos + 1).max(target.saturating_sub(tolerance));
            let hi = (target + tolerance).min(n - 1);

            if lo > hi {
                break;
            }

            let cut = find_cut_center(&smoothed, lo, hi, target, zero_threshold).unwrap_or_else(|| {
                (lo..=hi).min_by(|&a, &b| smoothed[a].total_cmp(&smoothed[b])).unwrap_or(target)
            });

            cuts.push(cut);
            pos = cut;
        }

        cuts.push(n);
        cuts
    }
}
