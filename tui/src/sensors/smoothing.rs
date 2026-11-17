// ─── Smoothing (Moving Average) ─────────────────────────────────────────
// Applies exponential moving average to sensor readings to reduce
// visual fluctuations. Uses OnceLock + Mutex for thread safety.
// Also maintains a ring buffer of recent values for sparkline graphs.

use std::sync::{Mutex, OnceLock};

const HISTORY_LEN: usize = 60; // ~2 minutes at 2s polling

struct Smoother {
    value: Option<f64>,
    alpha: f64,
    history: Vec<f64>,
}

impl Smoother {
    fn new(alpha: f64) -> Self {
        Smoother {
            value: None,
            alpha,
            history: Vec::with_capacity(HISTORY_LEN),
        }
    }

    fn smooth(&mut self, new_value: f64) -> f64 {
        let smoothed = match self.value {
            Some(prev) => self.alpha * new_value + (1.0 - self.alpha) * prev,
            None => new_value,
        };
        self.value = Some(smoothed);
        self.history.push(smoothed);
        if self.history.len() > HISTORY_LEN {
            self.history.remove(0);
        }
        smoothed
    }

    fn history(&self) -> &[f64] {
        &self.history
    }
}

static CPU_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static IGPU_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static GPU_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static GPU_MEM_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static CPU_POWER_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static IGPU_POWER_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static GPU_POWER_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static TOTAL_POWER_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static NET_RX_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();
static NET_TX_SMOOTHER: OnceLock<Mutex<Smoother>> = OnceLock::new();

pub fn smooth_cpu_usage(value: f64) -> f64 {
    let smoother = CPU_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.3)));
    smoother.lock().unwrap().smooth(value)
}

pub fn smooth_igpu_usage(value: f64) -> f64 {
    let smoother = IGPU_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.3)));
    smoother.lock().unwrap().smooth(value)
}

pub fn smooth_gpu_usage(value: f64) -> f64 {
    let smoother = GPU_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.3)));
    smoother.lock().unwrap().smooth(value)
}

pub fn smooth_gpu_mem_usage(value: f64) -> f64 {
    let smoother = GPU_MEM_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.3)));
    smoother.lock().unwrap().smooth(value)
}

pub fn smooth_cpu_power(value: f64) -> f64 {
    let smoother = CPU_POWER_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.4)));
    smoother.lock().unwrap().smooth(value)
}

pub fn smooth_igpu_power(value: f64) -> f64 {
    let smoother = IGPU_POWER_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.4)));
    smoother.lock().unwrap().smooth(value)
}

pub fn smooth_gpu_power(value: f64) -> f64 {
    let smoother = GPU_POWER_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.4)));
    smoother.lock().unwrap().smooth(value)
}

// Sparkline history accessors — return f32 for Ratatui Sparkline
pub fn cpu_usage_history() -> Vec<f32> {
    let smoother = match CPU_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

pub fn igpu_usage_history() -> Vec<f32> {
    let smoother = match IGPU_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

pub fn gpu_usage_history() -> Vec<f32> {
    let smoother = match GPU_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

#[allow(dead_code)]
pub fn gpu_mem_history() -> Vec<f32> {
    let smoother = match GPU_MEM_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

#[allow(dead_code)]
pub fn cpu_power_history() -> Vec<f32> {
    let smoother = match CPU_POWER_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

#[allow(dead_code)]
pub fn igpu_power_history() -> Vec<f32> {
    let smoother = match IGPU_POWER_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

#[allow(dead_code)]
pub fn gpu_power_history() -> Vec<f32> {
    let smoother = match GPU_POWER_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

pub fn smooth_total_power(value: f64) -> f64 {
    let smoother = TOTAL_POWER_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.4)));
    smoother.lock().unwrap().smooth(value)
}

#[allow(dead_code)]
pub fn smooth_net_rx(value: f64) -> f64 {
    let smoother = NET_RX_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.3)));
    smoother.lock().unwrap().smooth(value)
}

#[allow(dead_code)]
pub fn smooth_net_tx(value: f64) -> f64 {
    let smoother = NET_TX_SMOOTHER.get_or_init(|| Mutex::new(Smoother::new(0.3)));
    smoother.lock().unwrap().smooth(value)
}

pub fn net_rx_history() -> Vec<f32> {
    let smoother = match NET_RX_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}

pub fn net_tx_history() -> Vec<f32> {
    let smoother = match NET_TX_SMOOTHER.get() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let guard = smoother.lock().unwrap();
    guard.history().iter().map(|v| *v as f32).collect()
}
