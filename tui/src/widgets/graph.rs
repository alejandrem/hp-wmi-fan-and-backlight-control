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

/// Map one real sample to each terminal column, anchoring the newest sample at
/// the right edge. Keeping one vertical stack per column makes its dot count
/// match the percentage threshold without doubling dots across Braille's
/// two horizontal subpixels.
pub(super) fn vertical_dot_history(history: &[f64], width: u16) -> Vec<(f64, f64)> {
    let columns = width as usize;
    if history.is_empty() || columns == 0 {
        return Vec::new();
    }
    let visible = history.len().min(columns);
    let start = history.len() - visible;
    let left_padding = columns - visible;
    history[start..].iter().enumerate().flat_map(|(index, value)| {
        let count = if *value <= 20.0 { 1 } else if *value <= 40.0 { 2 } else if *value <= 70.0 { 3 } else { 4 };
        let x = (left_padding + index) as f64 + 0.5;
        (0..count).map(move |dot| (x, dot as f64 + 0.5))
    }).collect()
}
