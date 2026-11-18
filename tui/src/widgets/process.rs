// ─── Process Widget ─────────────────────────────────────────────────────
// Shows the complete process list using a scrollable Ratatui Table widget for proper
// column alignment: Pid, Program, User, Mem%, Cpu%.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table,
};
use ratatui::Frame;

use crate::sensors::Sensors;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortColumn {
    Cpu,
    Memory,
    Pid,
    Name,
}

impl SortColumn {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "MEM",
            Self::Pid => "PID",
            Self::Name => "NAME",
        }
    }
}

pub fn render(
    frame: &mut Frame,
    sensors: &Sensors,
    area: Rect,
    scroll: usize,
    sort: SortColumn,
    reverse: bool,
) {
    let block = build_proc_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let procs = &sensors.processes;
    if procs.is_empty() {
        return;
    }
    let sections = build_proc_sections(inner);
    render_proc_header(frame, sections.header, procs.len(), sort, reverse);
    render_proc_table(frame, sections.table, procs, scroll);
}

struct ProcSections {
    header: Rect,
    table: Rect,
}

fn build_proc_sections(inner: Rect) -> ProcSections {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);
    ProcSections {
        header: parts[0],
        table: parts[1],
    }
}

fn build_proc_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "  processes",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
}

fn render_proc_header(
    frame: &mut Frame,
    area: Rect,
    process_count: usize,
    sort: SortColumn,
    reverse: bool,
) {
    let hints = build_header_hints(process_count, sort, reverse);
    let widget = Paragraph::new(hints)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(widget, area);
}

fn build_header_hints(process_count: usize, sort: SortColumn, reverse: bool) -> Line<'static> {
    let descending = matches!(sort, SortColumn::Cpu | SortColumn::Memory) ^ reverse;
    let direction = if descending { "↓" } else { "↑" };
    Line::from(vec![
        Span::styled(
            format!("{}{}", sort.label(), direction),
            Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("1 CPU 2 MEM 3 PID 4 NAME  R flip", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(format!("{} tasks", process_count), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("↑↓ PgUp/Dn", Style::default().fg(Color::DarkGray)),
    ])
}

fn render_proc_table(
    frame: &mut Frame,
    area: Rect,
    procs: &[crate::sensors::ProcessInfo],
    scroll: usize,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let table_width = area.width.saturating_sub(1);
    let table_area = Rect { width: table_width, ..area };
    let widths = build_column_widths(table_width);
    let header_row = build_header_row(&widths);
    let visible_rows = table_area.height.saturating_sub(1) as usize;
    let max_scroll = procs.len().saturating_sub(visible_rows);
    let scroll = scroll.min(max_scroll);
    let end = scroll.saturating_add(visible_rows).min(procs.len());
    let visible = &procs[scroll.min(procs.len())..end];
    let data_rows = build_data_rows(visible, &widths, table_width);
    let table = Table::default()
        .widths(&widths)
        .header(header_row)
        .rows(data_rows);
    frame.render_widget(table, table_area);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(Color::Cyan))
        .track_style(Style::default().fg(Color::DarkGray));
    let mut state = ScrollbarState::default()
        .content_length(procs.len())
        .viewport_content_length(visible_rows)
        .position(scroll);
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

fn build_column_widths(width: u16) -> [Constraint; 5] {
    let avail = width.saturating_sub(5);
    let pid_w = 6.min(avail / 4);
    let user_w = 6.min(avail / 4);
    let mem_w = 5.min(avail / 4);
    let cpu_w = 5.min(avail / 4);
    let prog_w = avail.saturating_sub(pid_w + user_w + mem_w + cpu_w);
    [
        Constraint::Length(pid_w),
        Constraint::Length(prog_w.max(10)),
        Constraint::Length(user_w),
        Constraint::Length(mem_w),
        Constraint::Length(cpu_w),
    ]
}

fn build_header_row(_widths: &[Constraint; 5]) -> Row<'_> {
    let cells = vec![
        Cell::from(" Pid").style(header_style()),
        Cell::from(" Program").style(header_style()),
        Cell::from(" User").style(header_style()),
        Cell::from(" Mem").style(header_style()),
        Cell::from(" Cpu%").style(header_style()),
    ];
    Row::new(cells).style(header_style())
}

fn header_style() -> Style {
    Style::default()
        .fg(Color::Gray)
        .add_modifier(Modifier::BOLD)
}

fn build_data_rows<'a>(
    procs: &'a [crate::sensors::ProcessInfo],
    _widths: &'a [Constraint; 5],
    area_width: u16,
) -> Vec<Row<'a>> {
    let program_width = area_width.saturating_sub(30).max(4) as usize;
    procs.iter().map(|p| {
        let mem_str = format_mem(p.mem_mb);
        let name = truncate(&p.name, program_width);
        let cells = vec![
            Cell::from(format!("{:<6}", p.pid)).style(Style::default().fg(Color::White)),
            Cell::from(name).style(Style::default().fg(Color::Cyan)),
            Cell::from(p.user.as_str()).style(Style::default().fg(Color::Magenta)),
            Cell::from(mem_str).style(Style::default().fg(Color::Green)),
            Cell::from(format!("{:.1}", p.cpu_pct)).style(Style::default().fg(Color::Yellow)),
        ];
        Row::new(cells)
    }).collect()
}

fn format_mem(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.0}G", mb / 1024.0)
    } else {
        format!("{:.0}M", mb)
    }
}

fn truncate(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        let prefix: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", prefix)
    }
}
