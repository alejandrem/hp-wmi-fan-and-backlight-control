// ─── GPU (NVIDIA RTX) Usage Calculator ──────────────────────────────────
// Reads NVIDIA GPU usage via nvidia-smi query.

/// Returns the GPU model name from nvidia-smi (e.g., "RTX 4050").
pub fn gpu_model_name() -> String {
    let output = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=name", "--format=csv,noheader"])
        .output()
        .ok();

    if let Some(out) = output {
        let text = match String::from_utf8(out.stdout) {
            Ok(t) => t,
            Err(_) => return "GPU".into(),
        };
        let full = text.trim().to_string();
        // Shorten: "NVIDIA GeForce RTX 4050 Laptop GPU" -> "RTX 4050"
        if let Some(start) = full.find("RTX") {
            let rest = &full[start..];
            // Take "RTX 4050" (up to space after digits)
            let end = rest[3..].find(|c: char| c.is_alphabetic()).map(|i| i + 3).unwrap_or(rest.len());
            return rest[..end].trim().to_string();
        }
        return full;
    }
    "GPU".into()
}

/// Returns the GPU usage percentage (0..100) from nvidia-smi.
pub fn read_gpu_usage() -> Option<f64> {
    let output = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=utilization.gpu", "--format=csv,noheader"])
        .output()
        .ok()?;

    let text = String::from_utf8(output.stdout).ok()?;
    // Format: " 42 %"
    text.trim()
        .split('%')
        .next()
        .and_then(|s| s.trim().parse::<f64>().ok())
}

/// Returns the GPU memory usage percentage (0..100) from nvidia-smi.
pub fn read_gpu_mem_usage() -> Option<f64> {
    let output = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=memory.used,memory.total", "--format=csv,noheader"])
        .output()
        .ok()?;

    let text = String::from_utf8(output.stdout).ok()?;
    // Format: "2 MiB, 6141 MiB"
    let fields: Vec<&str> = text.trim().split(',').collect();
    if fields.len() >= 2 {
        let used = parse_mib(fields[0].trim())?;
        let total = parse_mib(fields[1].trim())?;
        if total > 0.0 {
            return Some((used / total) * 100.0);
        }
    }
    None
}

fn parse_mib(s: &str) -> Option<f64> {
    // Parse "2 MiB" -> 2.0
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    if parts.len() >= 1 {
        parts[0].parse::<f64>().ok()
    } else {
        None
    }
}
