/// Resample a chronological history to one value per terminal column.
/// Linear interpolation keeps the dotted columns contiguous when the history
/// contains fewer samples than the graph has horizontal space.
pub(super) fn resample_history(history: &[u64], width: u16) -> Vec<f64> {
    let width = width as usize;
    if history.is_empty() || width == 0 {
        return Vec::new();
    }
    if history.len() == 1 {
        let mut values = vec![0.0; width];
        values[width - 1] = history[0] as f64;
        return values;
    }
    if width == 1 {
        return vec![*history.last().unwrap() as f64];
    }

    let last = history.len() - 1;
    (0..width).map(|column| {
        let position = column as f64 * last as f64 / (width - 1) as f64;
        let left = position.floor() as usize;
        let right = (left + 1).min(last);
        let fraction = position - left as f64;
        history[left] as f64 * (1.0 - fraction) + history[right] as f64 * fraction
    }).collect()
}
