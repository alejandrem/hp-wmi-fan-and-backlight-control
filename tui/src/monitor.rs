// ─── ALETOP (Entry Point) ───────────────────────────────────────────────
// Main TUI loop: reads sensors, handles events, renders btop-style layout.

mod sensors;
mod widgets;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use ratatui::Terminal;
use std::io;
use std::io::Write;
use std::time::Duration;

struct App {
    sensors: sensors::Sensors,
}

impl App {
    fn new() -> Self {
        App {
            sensors: sensors::Sensors::read(),
        }
    }

    fn update(&mut self) {
        self.sensors = sensors::Sensors::read();
    }
}

fn main() {
    if !atty::is(atty::Stream::Stdout) {
        let sensors = sensors::Sensors::read();
        eprintln!("Non-TTY mode: sensors read OK");
        debug_sensors(&sensors);
        return;
    }

    setup_terminal();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)
        .expect("Failed to create terminal");
    let mut app = App::new();

    let mut last_update = std::time::Instant::now();
    loop {
        terminal.draw(|f| draw(f, &app)).unwrap();
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(KeyEvent { code, kind, modifiers, .. }) = event::read().unwrap() {
                if kind == KeyEventKind::Press {
                    if code == KeyCode::Char('q') || code == KeyCode::Esc {
                        break;
                    }
                    if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
                        break;
                    }
                }
            }
        }
        if last_update.elapsed() >= Duration::from_millis(2000) {
            app.update();
            last_update = std::time::Instant::now();
        }
    }

    restore_terminal();
}

fn setup_terminal() {
    crossterm::terminal::enable_raw_mode().expect("Failed to enable raw mode");
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    use crossterm::QueueableCommand;
    handle.queue(crossterm::terminal::EnterAlternateScreen).unwrap();
    handle.queue(crossterm::cursor::Hide).unwrap();
    handle.flush().unwrap();
}

fn restore_terminal() {
    crossterm::terminal::disable_raw_mode().expect("Failed to disable raw mode");
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    use crossterm::QueueableCommand;
    handle.queue(crossterm::terminal::LeaveAlternateScreen).unwrap();
    handle.queue(crossterm::cursor::Show).unwrap();
    handle.flush().unwrap();
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = build_layout(area);
    widgets::header::render(frame, &app.sensors, chunks.header);
    widgets::top_left::render(frame, &app.sensors, chunks.top_left);
    widgets::top_right::render(frame, &app.sensors, chunks.top_right);
    widgets::memory::render(frame, &app.sensors, chunks.memory);
    widgets::disk::render(frame, &app.sensors, chunks.disk);
    widgets::fans_temp::render(frame, &app.sensors, chunks.fans_temp);
    widgets::network::render(frame, &app.sensors, chunks.network);
    widgets::process::render(frame, &app.sensors, chunks.process);
}

struct LayoutAreas {
    header: ratatui::layout::Rect,
    top_left: ratatui::layout::Rect,
    top_right: ratatui::layout::Rect,
    memory: ratatui::layout::Rect,
    disk: ratatui::layout::Rect,
    fans_temp: ratatui::layout::Rect,
    network: ratatui::layout::Rect,
    process: ratatui::layout::Rect,
}

fn build_layout(area: ratatui::layout::Rect) -> LayoutAreas {
    let main = build_main_layout(area);
    let header = main[0];
    let (top_left, top_right) = build_top_section(main[1]);
    let (memory, disk, fans_temp) = build_middle_section(main[2]);
    let (network, process) = build_bottom_section(main[3]);

    LayoutAreas {
        header, top_left, top_right,
        memory, disk, fans_temp, network, process,
    }
}

fn build_main_layout(area: ratatui::layout::Rect) -> Vec<ratatui::layout::Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
        ])
        .split(area)
        .to_vec()
}

fn build_top_section(area: ratatui::layout::Rect) -> (ratatui::layout::Rect, ratatui::layout::Rect) {
    let cols: Vec<_> = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area)
        .to_vec();
    (cols[0], cols[1])
}

fn build_middle_section(area: ratatui::layout::Rect) -> (ratatui::layout::Rect, ratatui::layout::Rect, ratatui::layout::Rect) {
    let cols: Vec<_> = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
        ])
        .split(area)
        .to_vec();
    (cols[0], cols[1], cols[2])
}

fn build_bottom_section(area: ratatui::layout::Rect) -> (ratatui::layout::Rect, ratatui::layout::Rect) {
    let cols: Vec<_> = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area)
        .to_vec();
    (cols[0], cols[1])
}

fn debug_sensors(s: &sensors::Sensors) {
    eprintln!("CPU: {} ({}c), usage: {:.1}%", s.cpu_model, s.cpu_cores, s.cpu_usage);
    eprintln!("iGPU: {}, usage: {:?}", s.igpu_model, s.igpu_usage);
    eprintln!("GPU: {}, usage: {:?}", s.gpu_model, s.gpu_usage);
    eprintln!("Memory: {:.1} GiB total", s.memory.total_gib);
    eprintln!("Disks: {}", s.disks.len());
    eprintln!("Network: {:?}", s.network.as_ref().map(|n| &n.ip_address));
    eprintln!("Processes: {}", s.processes.len());
    eprintln!("Uptime: {}", s.uptime);
    eprintln!("Load avg: {:.2} {:.2} {:.2}", s.load_avg[0], s.load_avg[1], s.load_avg[2]);
}
