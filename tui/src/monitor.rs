// ─── ALETOP (Entry Point) ───────────────────────────────────────────────
// Main TUI loop: reads sensors, handles events, renders btop-style layout.

mod sensors;
mod widgets;

use crossterm::event::{
    self, EnableMouseCapture, DisableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use ratatui::Terminal;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use widgets::process::SortColumn;
use widgets::fans_temp::FanControlButton;

#[derive(Clone, Copy)]
enum FanCommand {
    Auto,
    Max,
    Rpm(u16),
}

impl FanCommand {
    fn label(self) -> String {
        match self {
            Self::Auto => "AUTO".into(),
            Self::Max => "MAX".into(),
            Self::Rpm(rpm) => format!("{rpm} RPM"),
        }
    }

    fn success_message(self) -> String {
        match self {
            Self::Auto => "Ventiladores activados en modo AUTO; el control vuelve al firmware.".into(),
            Self::Max => "Ventiladores activados en modo MAX.".into(),
            Self::Rpm(rpm) => format!("Ventiladores configurados a {rpm} RPM."),
        }
    }
}

enum KeyAction {
    Continue,
    Quit,
    Fan(FanCommand),
}

enum FanCommandResult {
    NeedPassword(FanCommand),
    Finished { message: String, success: bool },
}

struct FanNotice {
    title: String,
    body: String,
    success: Option<bool>,
    expires_at: Option<Instant>,
}

struct App {
    sensors: sensors::Sensors,
    sensor_rx: Receiver<sensors::Sensors>,
    process_scroll: usize,
    process_page_size: usize,
    process_sort: SortColumn,
    process_reverse: bool,
    fan_rpm: u16,
    rpm_buffer: String,
    rpm_editing: bool,
    fan_message: String,
    fan_modal: Option<FanNotice>,
    fan_notice_count: usize,
    sudo_password: String,
    password_prompt: bool,
    pending_fan_command: Option<FanCommand>,
    fan_command_running: bool,
    fan_result_tx: Sender<FanCommandResult>,
    fan_result_rx: Receiver<FanCommandResult>,
}

impl App {
    fn new() -> Self {
        let current = sensors::Sensors::empty();
        let (sensor_tx, sensor_rx) = mpsc::sync_channel(1);
        let (fan_result_tx, fan_result_rx) = mpsc::channel();
        std::thread::spawn(move || {
            // Keep the screen responsive while avoiding a full sensor and
            // /proc process scan twice per second.
            let interval = Duration::from_secs(1);
            if sensor_tx.send(sensors::Sensors::read()).is_err() {
                return;
            }
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
            fan_rpm: 3000,
            rpm_buffer: String::new(),
            rpm_editing: false,
            fan_message: String::new(),
            fan_modal: None,
            fan_notice_count: 0,
            sudo_password: String::new(),
            password_prompt: false,
            pending_fan_command: None,
            fan_command_running: false,
            fan_result_tx,
            fan_result_rx,
        };
        app.sort_processes();
        app
    }

    fn update(&mut self) -> bool {
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
        while let Ok(result) = self.fan_result_rx.try_recv() {
            changed = true;
            match result {
                FanCommandResult::NeedPassword(command) => {
                    self.fan_command_running = false;
                    self.pending_fan_command = Some(command);
                    self.password_prompt = true;
                    self.sudo_password.clear();
                    self.fan_message = "sudo password: (Enter=continue, Esc=cancel)".into();
                    self.fan_modal = Some(FanNotice {
                        title: "Autenticación de sudo".into(),
                        body: "sudo password: ".into(),
                        success: None,
                        expires_at: None,
                    });
                }
                FanCommandResult::Finished { message, success } => {
                    self.fan_command_running = false;
                    self.pending_fan_command = None;
                    self.password_prompt = false;
                    self.fan_message = message.clone();
                    let seconds = if self.fan_notice_count == 0 { 5 } else { 2 };
                    self.fan_notice_count += 1;
                    self.fan_modal = Some(fan_notice(&message, success, seconds));
                }
            }
        }
        if self.fan_modal.as_ref().is_some_and(|notice| {
            notice.expires_at.is_some_and(|expires_at| Instant::now() >= expires_at)
        }) {
            self.fan_modal = None;
            changed = true;
        }
        changed
    }

    fn request_fan_command(&mut self, command: FanCommand) {
        if self.fan_command_running || self.password_prompt {
            self.fan_message = "A fan command is already in progress".into();
            return;
        }
        self.fan_command_running = true;
        self.fan_message = format!("Checking permission for {}…", command.label());
        self.fan_modal = Some(FanNotice {
            title: format!("Aplicando {}…", command.label()),
            body: format!("Enviando la orden para {}.", command.label()),
            success: None,
            expires_at: None,
        });
        self.start_fan_worker(command, None);
    }

    fn submit_sudo_password(&mut self) {
        let Some(command) = self.pending_fan_command.take() else { return };
        let password = std::mem::take(&mut self.sudo_password);
        self.password_prompt = false;
        self.fan_command_running = true;
        self.fan_message = format!("Applying {}…", command.label());
        self.fan_modal = Some(FanNotice {
            title: format!("Aplicando {}…", command.label()),
            body: "La orden está en curso; Ale Top seguirá abierta.".into(),
            success: None,
            expires_at: None,
        });
        self.start_fan_worker(command, Some(password));
    }

    fn start_fan_worker(&self, command: FanCommand, password: Option<String>) {
        let result_tx = self.fan_result_tx.clone();
        std::thread::spawn(move || {
            let result = execute_fan_command(command, password);
            let _ = result_tx.send(result);
        });
    }

    fn update_password_message(&mut self) {
        let mask = "•".repeat(self.sudo_password.chars().count());
        self.fan_message = format!("sudo password: {mask}  (Enter=continue, Esc=cancel)");
        if let Some(notice) = self.fan_modal.as_mut() {
            notice.body = format!("sudo password: {mask}");
        }
    }

    fn clear_sudo_password(&mut self) {
        self.sudo_password.clear();
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

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> KeyAction {
        if self.password_prompt {
            match code {
                KeyCode::Esc => {
                    self.password_prompt = false;
                    self.pending_fan_command = None;
                    self.clear_sudo_password();
                    self.fan_message = "Fan command cancelled".into();
                    let seconds = if self.fan_notice_count == 0 { 5 } else { 2 };
                    self.fan_notice_count += 1;
                    self.fan_modal = Some(fan_notice("Operación cancelada.", false, seconds));
                }
                KeyCode::Enter => self.submit_sudo_password(),
                KeyCode::Backspace => {
                    self.sudo_password.pop();
                    self.update_password_message();
                }
                KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => return KeyAction::Quit,
                KeyCode::Char(ch) if !ch.is_control() => {
                    self.sudo_password.push(ch);
                    self.update_password_message();
                }
                _ => {}
            }
            return KeyAction::Continue;
        }

        if self.fan_modal.is_some() {
            if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
                return KeyAction::Quit;
            }
            return KeyAction::Continue;
        }

        if self.rpm_editing {
            match code {
                KeyCode::Esc => {
                    self.rpm_editing = false;
                    self.rpm_buffer.clear();
                    self.fan_message.clear();
                }
                KeyCode::Enter => {
                    let rpm = if self.rpm_buffer.is_empty() {
                        self.fan_rpm
                    } else {
                        match self.rpm_buffer.parse::<u16>() {
                            Ok(value) if value <= 6000 => value,
                            _ => {
                                self.fan_message = "RPM must be 0-6000".into();
                                return KeyAction::Continue;
                            }
                        }
                    };
                    self.fan_rpm = rpm;
                    self.rpm_editing = false;
                    self.rpm_buffer.clear();
                    return KeyAction::Fan(FanCommand::Rpm(rpm));
                }
                KeyCode::Backspace => {
                    self.rpm_buffer.pop();
                    self.fan_message.clear();
                }
                KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => return KeyAction::Quit,
                KeyCode::Char(digit) if digit.is_ascii_digit() && self.rpm_buffer.len() < 4 => {
                    let mut candidate = self.rpm_buffer.clone();
                    candidate.push(digit);
                    if candidate.parse::<u16>().is_ok_and(|value| value <= 6000) {
                        self.rpm_buffer = candidate;
                        self.fan_message.clear();
                    } else {
                        self.fan_message = "Maximum is 6000 RPM".into();
                    }
                }
                _ => {}
            }
            return KeyAction::Continue;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => KeyAction::Quit,
            KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => KeyAction::Quit,
            KeyCode::F(1) => KeyAction::Fan(FanCommand::Auto),
            KeyCode::F(2) => KeyAction::Fan(FanCommand::Max),
            KeyCode::F(3) => {
                self.rpm_editing = true;
                self.rpm_buffer.clear();
                self.fan_message = "Type 0-6000 RPM, Enter to apply".into();
                KeyAction::Continue
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.fan_rpm = self.fan_rpm.saturating_add(100).min(6000);
                KeyAction::Continue
            }
            KeyCode::Char('-') => {
                self.fan_rpm = self.fan_rpm.saturating_sub(100);
                KeyAction::Continue
            }
            _ => {
                self.handle_process_key(code);
                KeyAction::Continue
            }
        }
    }

    fn handle_fan_button(&mut self, button: FanControlButton) -> Option<FanCommand> {
        match button {
            FanControlButton::Auto => {
                self.rpm_editing = false;
                Some(FanCommand::Auto)
            }
            FanControlButton::Max => {
                self.rpm_editing = false;
                Some(FanCommand::Max)
            }
            FanControlButton::RpmDown => {
                self.fan_rpm = self.fan_rpm.saturating_sub(100);
                None
            }
            FanControlButton::RpmInput => {
                self.rpm_editing = true;
                self.rpm_buffer.clear();
                self.fan_message = "Type 0-6000 RPM, Enter to apply".into();
                None
            }
            FanControlButton::RpmUp => {
                self.fan_rpm = self.fan_rpm.saturating_add(100).min(6000);
                None
            }
            FanControlButton::ApplyRpm => {
                let rpm = if self.rpm_editing {
                    self.rpm_buffer.parse::<u16>().ok().filter(|value| *value <= 6000)
                        .unwrap_or(self.fan_rpm)
                } else {
                    self.fan_rpm
                };
                self.fan_rpm = rpm;
                self.rpm_editing = false;
                self.rpm_buffer.clear();
                Some(FanCommand::Rpm(rpm))
            }
        }
    }

    fn fan_rpm_display(&self) -> String {
        if self.rpm_editing {
            if self.rpm_buffer.is_empty() { "____".into() } else { self.rpm_buffer.clone() }
        } else {
            self.fan_rpm.to_string()
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
    let mut needs_draw = true;
    let mut next_modal_draw = Instant::now();

    loop {
        if needs_draw {
            let size = terminal.size().unwrap();
            let layout = build_layout(ratatui::layout::Rect::new(0, 0, size.width, size.height));
            app.process_page_size = layout.process.height.saturating_sub(4).max(1) as usize;
            terminal.draw(|f| draw(f, &app)).unwrap();
            needs_draw = false;
        }
        if event::poll(Duration::from_millis(50)).unwrap() {
            match event::read().unwrap() {
                Event::Key(KeyEvent { code, kind, modifiers, .. })
                    if kind == KeyEventKind::Press =>
                {
                    match app.handle_key(code, modifiers) {
                        KeyAction::Quit => break,
                        KeyAction::Continue => needs_draw = true,
                        KeyAction::Fan(command) => {
                            app.request_fan_command(command);
                            needs_draw = true;
                        }
                    }
                }
                Event::Mouse(mouse)
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                {
                    let size = terminal.size().unwrap();
                    let layout = build_layout(ratatui::layout::Rect::new(0, 0, size.width, size.height));
                    if app.fan_modal.is_none() {
                        if let Some(button) = widgets::fans_temp::hit_test_control(
                            layout.fans_temp,
                            mouse.column,
                            mouse.row,
                        ) {
                            if let Some(command) = app.handle_fan_button(button) {
                                app.request_fan_command(command);
                            }
                        }
                    }
                    needs_draw = true;
                }
                Event::Resize(_, _) => needs_draw = true,
                _ => {}
            }
        }
        if app.update() {
            needs_draw = true;
        }
        if app.fan_modal.is_some() && Instant::now() >= next_modal_draw {
            needs_draw = true;
            next_modal_draw = Instant::now() + Duration::from_secs(1);
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
    handle.queue(EnableMouseCapture).unwrap();
    handle.flush().unwrap();
}

fn restore_terminal() {
    crossterm::terminal::disable_raw_mode().expect("Failed to disable raw mode");
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    use crossterm::QueueableCommand;
    handle.queue(DisableMouseCapture).unwrap();
    handle.queue(crossterm::terminal::LeaveAlternateScreen).unwrap();
    handle.queue(crossterm::cursor::Show).unwrap();
    handle.flush().unwrap();
}

fn execute_fan_command(action: FanCommand, password: Option<String>) -> FanCommandResult {
    if password.is_none() {
        let auth = Command::new("sudo").args(["-n", "-v"]).output();
        if !matches!(auth, Ok(output) if output.status.success()) {
            return FanCommandResult::NeedPassword(action);
        }
    }

    let mut command = Command::new("sudo");
    if password.is_some() {
        command.args(["-S", "-p", ""]);
    } else {
        command.arg("-n");
    }
    command.arg(fanctl_program());
    match action {
        FanCommand::Auto => { command.arg("auto"); }
        FanCommand::Max => { command.arg("max"); }
        FanCommand::Rpm(rpm) => { command.arg("rpm").arg(rpm.to_string()); }
    }

    let mut secret = password.map(String::into_bytes);
    if secret.is_some() {
        command.stdin(Stdio::piped());
    }
    let child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            if let Some(secret) = secret.as_mut() { secret.fill(0); }
            return FanCommandResult::Finished {
                message: format!("Could not start sudo: {error}"),
                success: false,
            };
        }
    };

    if let (Some(secret), Some(mut stdin)) = (secret.as_mut(), child.stdin.take()) {
        let _ = stdin.write_all(secret);
        let _ = stdin.write_all(b"\n");
        secret.fill(0);
    }
    if let Some(secret) = secret.as_mut() { secret.fill(0); }

    match child.wait_with_output() {
        Ok(output) if output.status.success() => FanCommandResult::Finished {
            message: action.success_message(),
            success: true,
        },
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if detail.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                detail
            };
            if detail.is_empty() {
                FanCommandResult::Finished {
                    message: format!("{} failed ({})", action.label(), output.status),
                    success: false,
                }
            } else {
                FanCommandResult::Finished {
                    message: format!("{} failed: {detail}", action.label()),
                    success: false,
                }
            }
        }
        Err(error) => FanCommandResult::Finished {
            message: format!("Could not read fanctl result: {error}"),
            success: false,
        },
    }
}

fn fan_notice(message: &str, success: bool, seconds: u64) -> FanNotice {
    FanNotice {
        title: if success { "¡Cambio aplicado!" } else { "No se pudo cambiar el ventilador" }.into(),
        body: message.to_owned(),
        success: Some(success),
        expires_at: Some(Instant::now() + Duration::from_secs(seconds)),
    }
}

fn fanctl_program() -> std::ffi::OsString {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("fanctl")))
        .filter(|path| path.is_file())
        .map(|path| path.into_os_string())
        .unwrap_or_else(|| "fanctl".into())
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = build_layout(area);
    widgets::header::render(frame, &app.sensors, chunks.header);
    widgets::top_left::render(frame, &app.sensors, chunks.top_left);
    widgets::top_right::render(frame, &app.sensors, chunks.top_right);
    widgets::memory::render(frame, &app.sensors, chunks.memory);
    widgets::disk::render(frame, &app.sensors, chunks.disk);
    let rpm_text = app.fan_rpm_display();
    widgets::fans_temp::render(
        frame,
        &app.sensors,
        chunks.fans_temp,
        &rpm_text,
        app.rpm_editing,
        &app.fan_message,
    );
    widgets::network::render(frame, &app.sensors, chunks.network);
    widgets::process::render(
        frame,
        &app.sensors,
        chunks.process,
        app.process_scroll,
        app.process_sort,
        app.process_reverse,
    );
    if let Some(notice) = &app.fan_modal {
        draw_fan_modal(frame, notice, app.password_prompt);
    }
}

fn draw_fan_modal(frame: &mut Frame, notice: &FanNotice, password_prompt: bool) {
    let screen = frame.area();
    let width = screen.width.min(64);
    let height = screen.height.min(9);
    if width < 10 || height < 5 {
        return;
    }
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, area);
    let accent = match notice.success {
        Some(true) => Color::Green,
        Some(false) => Color::Red,
        None => Color::Cyan,
    };
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", notice.title),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().fg(Color::White).bg(Color::Black));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text = Paragraph::new(vec![
        Line::from(Span::styled(
            notice.body.as_str(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            match notice.expires_at {
                Some(expires_at) => format!(
                    "Se cierra automáticamente en {} s",
                    expires_at.saturating_duration_since(Instant::now()).as_secs_f64().ceil() as u64,
                ),
                None if password_prompt => "Escribe la contraseña · Enter continúa · Esc cancela".into(),
                None => "Aplicando; el resultado aparecerá aquí…".into(),
            },
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(text, inner);
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
