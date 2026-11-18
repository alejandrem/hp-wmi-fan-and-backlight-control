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
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use widgets::process::SortColumn;

struct App {
    sensors: sensors::Sensors,
    sensor_rx: Receiver<sensors::Sensors>,
    process_scroll: usize,
    process_page_size: usize,
    process_sort: SortColumn,
    process_reverse: bool,
}

impl App {
    fn new() -> Self {
        let current = sensors::Sensors::read();
        let (sensor_tx, sensor_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let interval = Duration::from_millis(500);
            let mut next_sample = std::time::Instant::now() + interval;
            loop {
                std::thread::sleep(next_sample.saturating_duration_since(std::time::Instant::now()));
                if sensor_tx.send(sensors::Sensors::read()).is_err() {
                    break;
                }
                let now = std::time::Instant::now();
                next_sample += interval;
                if next_sample <= now {
                    next_sample = now + interval;
                }
            }
        });
        let mut app = App {
            sensors: current,
            sensor_rx,
            process_scroll: 0,
            process_page_size: 1,
            process_sort: SortColumn::Cpu,
            process_reverse: false,
        };
        app.sort_processes();
        app
    }

    fn update(&mut self) {
        let mut changed = false;
        while let Ok(sensors) = self.sensor_rx.try_recv() {
            self.sensors = sensors;
            changed = true;
        }
        if changed {
            self.sort_processes();
            self.process_scroll = self.process_scroll.min(
                self.sensors.processes.len().saturating_sub(self.process_page_size),
            );
        }
    }

    fn sort_processes(&mut self) {
        let sort = self.process_sort;
        let reverse = self.process_reverse;
        self.sensors.processes.sort_by(|a, b| {
            let order = match sort {
                SortColumn::Cpu => b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Memory => b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Pid => a.pid.cmp(&b.pid),
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            };
            if reverse { order.reverse() } else { order }
        });
    }

    fn handle_process_key(&mut self, code: KeyCode) {
        let max_scroll = self.sensors.processes.len().saturating_sub(self.process_page_size);
        let needs_sort = match code {
            KeyCode::Up => { self.process_scroll = self.process_scroll.saturating_sub(1); false },
            KeyCode::Down => { self.process_scroll = self.process_scroll.saturating_add(1).min(max_scroll); false },
            KeyCode::PageUp => { self.process_scroll = self.process_scroll.saturating_sub(10); false },
            KeyCode::PageDown => { self.process_scroll = self.process_scroll.saturating_add(10).min(max_scroll); false },
            KeyCode::Home => { self.process_scroll = 0; false },
            KeyCode::End => { self.process_scroll = max_scroll; false },
            KeyCode::Char('1') => { self.process_sort = SortColumn::Cpu; true },
            KeyCode::Char('2') => { self.process_sort = SortColumn::Memory; true },
            KeyCode::Char('3') => { self.process_sort = SortColumn::Pid; true },
            KeyCode::Char('4') => { self.process_sort = SortColumn::Name; true },
            KeyCode::Char('r') | KeyCode::Char('R') => { self.process_reverse = !self.process_reverse; true },
            _ => return,
        };
        if needs_sort {
            self.sort_processes();
        }
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

    loop {
        let size = terminal.size().unwrap();
        let layout = build_layout(ratatui::layout::Rect::new(0, 0, size.width, size.height));
        app.process_page_size = layout.process.height.saturating_sub(4).max(1) as usize;
        terminal.draw(|f| draw(f, &app)).unwrap();
        if event::poll(Duration::from_millis(16)).unwrap() {
            if let Event::Key(KeyEvent { code, kind, modifiers, .. }) = event::read().unwrap() {
                if kind == KeyEventKind::Press {
                    if code == KeyCode::Char('q') || code == KeyCode::Esc
                        || (code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    app.handle_process_key(code);
                }
            }
        }
        app.update();
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
    widgets::process::render(
        frame,
        &app.sensors,
        chunks.process,
        app.process_scroll,
        app.process_sort,
        app.process_reverse,
    );
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
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Min(8),
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
