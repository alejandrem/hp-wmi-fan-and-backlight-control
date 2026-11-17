// ─── Network Sensor ─────────────────────────────────────────────────────
// Reads network statistics from /sys/class/net/ for rx/tx bytes.
// Calculates download/upload rates in KiB/s or MiB/s.

use std::sync::OnceLock;
use std::time::Instant;

pub struct NetworkInfo {
    #[allow(dead_code)]
    pub interface: String,
    pub ip_address: String,
    pub download_rate: f64,  // KiB/s
    pub upload_rate: f64,    // KiB/s
    #[allow(dead_code)]
    pub download_total: f64, // Total bytes downloaded
    #[allow(dead_code)]
    pub upload_total: f64,   // Total bytes uploaded
}

struct State {
    rx_bytes: u64,
    tx_bytes: u64,
    last_time: Instant,
}

static STATE: OnceLock<std::sync::Mutex<Option<State>>> = OnceLock::new();

pub fn read_network() -> Option<NetworkInfo> {
    let iface = find_active_interface()?;
    let (rx, tx) = read_net_bytes(&iface)?;
    let (rate_rx, rate_tx) = calc_net_rates(rx, tx);
    let ip = get_ip_address(&iface).unwrap_or_else(|| "N/A".into());
    Some(NetworkInfo {
        interface: iface,
        ip_address: ip,
        download_rate: rate_rx,
        upload_rate: rate_tx,
        download_total: rx as f64,
        upload_total: tx as f64,
    })
}

fn read_net_bytes(iface: &str) -> Option<(u64, u64)> {
    let rx = read_sys_u64(&format!("/sys/class/net/{}/statistics/rx_bytes", iface))?;
    let tx = read_sys_u64(&format!("/sys/class/net/{}/statistics/tx_bytes", iface))?;
    Some((rx, tx))
}

fn calc_net_rates(rx: u64, tx: u64) -> (f64, f64) {
    let state = STATE.get_or_init(|| std::sync::Mutex::new(None));
    let mut guard = state.lock().unwrap();
    let now = Instant::now();
    let (rate_rx, rate_tx) = update_net_state(&mut guard, rx, tx, now);
    *guard = Some(State { rx_bytes: rx, tx_bytes: tx, last_time: now });
    (rate_rx, rate_tx)
}

fn update_net_state(
    guard: &mut Option<State>, rx: u64, tx: u64, now: Instant,
) -> (f64, f64) {
    let Some(prev) = guard else { return (0.0, 0.0) };
    let delta = now.duration_since(prev.last_time);
    if delta.as_secs_f64() <= 0.0 {
        return (0.0, 0.0);
    }
    let rx_delta = rx.saturating_sub(prev.rx_bytes) as f64;
    let tx_delta = tx.saturating_sub(prev.tx_bytes) as f64;
    let rate = delta.as_secs_f64();
    (rx_delta / rate / 1024.0, tx_delta / rate / 1024.0)
}

fn find_active_interface() -> Option<String> {
    find_known_interface().or_else(find_first_interface)
}

fn find_known_interface() -> Option<String> {
    for iface in &["wlan0", "eth0", "enp0s3", "wlp2s0"] {
        if is_interface_active(iface) {
            return Some(iface.to_string());
        }
    }
    None
}

fn is_interface_active(iface: &str) -> bool {
    let path = format!("/sys/class/net/{}/statistics/rx_bytes", iface);
    std::fs::read_to_string(&path).is_ok()
}

fn find_first_interface() -> Option<String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return None;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name != "lo" && is_interface_active(&name) {
            return Some(name);
        }
    }
    None
}

fn read_sys_u64(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn get_ip_address(iface: &str) -> Option<String> {
    use std::process::Command;
    let output = Command::new("ip")
        .args(&["-4", "addr", "show", iface])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    for line in text.lines() {
        if line.contains("inet ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if part.starts_with("inet ") {
                    let addr = part["inet ".len()..].trim_end_matches('/');
                    return Some(addr.to_string());
                }
            }
        }
    }
    None
}
