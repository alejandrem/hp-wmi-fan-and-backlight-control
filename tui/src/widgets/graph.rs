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

/// Expand real samples with linear interpolation onto Braille's two horizontal
/// dot positions per terminal cell. The newest sample stays at the right edge.
pub(super) fn vertical_dot_history(history: &[f64], width: u16) -> Vec<(f64, f64)> {
    let slots = width as usize * 2;
    if history.is_empty() || slots == 0 {
        return Vec::new();
    }

    let mut values = Vec::with_capacity(history.len().saturating_mul(2));
    values.push(history[0]);
    for pair in history.windows(2) {
        values.push((pair[0] + pair[1]) / 2.0);
        values.push(pair[1]);
    }
    if values.len() > slots {
        values.drain(..values.len() - slots);
    }
    let left_padding = slots - values.len();
    values.into_iter().enumerate().flat_map(|(index, value)| {
        let count = if value <= 20.0 { 1 } else if value <= 40.0 { 2 } else if value <= 70.0 { 3 } else { 4 };
        let x = (left_padding + index) as f64 * 0.5 + 0.25;
        (0..count).map(move |dot| (x, dot as f64 + 0.5))
    }).collect()
}
