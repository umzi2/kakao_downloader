pub fn find_cut_center(
    signal: &[f32],
    lo: usize,
    hi: usize,
    target: usize,
    threshold: f32
) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    let mut run_start: Option<usize> = None;

    let commit = |best: &mut Option<(usize, usize)>, start: usize, end: usize| {
        let center = (start + end) / 2;
        let is_better = best.is_none_or(
            |(s, e)| center.abs_diff(target) < ((s + e) / 2).abs_diff(target)
        );
        if is_better {
            *best = Some((start, end));
        }
    };

    (lo..=hi).for_each(|index| {
        if signal[index] <= threshold {
            if run_start.is_none() {
                run_start = Some(index);
            }
        } else if let Some(start) = run_start.take() {
            commit(&mut best, start, index - 1);
        }
    });
    if let Some(start) = run_start {
        commit(&mut best, start, hi);
    }

    best.map(|(s, e)| (s + e) / 2)
}
