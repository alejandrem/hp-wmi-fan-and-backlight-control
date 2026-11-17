// ─── Network Widget ─────────────────────────────────────────────────────
// Shows IP, download/upload rates, and sparkline.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Sparkline};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_net_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let net = match &sensors.network {
        Some(n) => n,
        None => return,
    };
    let rows = build_net_rows(inner);
    render_net_ip(frame, rows[0], net);
    render_net_download(frame, rows[1], net);
    render_net_upload(frame, rows[2], net);
    draw_net_sparkline(frame, sensors, rows[3]);
}

fn build_net_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "  net",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
}

fn build_net_rows(inner: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner)
        .to_vec()
}

fn render_net_ip(frame: &mut Frame, area: Rect, net: &crate::sensors::NetworkInfo) {
    let widget = Paragraph::new(Span::styled(
        &net.ip_address,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn render_net_download(frame: &mut Frame, area: Rect, net: &crate::sensors::NetworkInfo) {
    let text = format!("V {:.1} KiB/s", net.download_rate);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn render_net_upload(frame: &mut Frame, area: Rect, net: &crate::sensors::NetworkInfo) {
    let text = format!("A {:.1} KiB/s", net.upload_rate);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn draw_net_sparkline(frame: &mut Frame, _sensors: &Sensors, area: Rect) {
    let combined = combine_net_history();
    if combined.is_empty() {
        return;
    }
    let sparkline = Sparkline::default()
        .data(&combined)
        .style(Color::Cyan);
    frame.render_widget(sparkline, area);
}

fn combine_net_history() -> Vec<u64> {
    use crate::sensors::smoothing;
    let rx_hist = to_u64(smoothing::net_rx_history());
    let tx_hist = to_u64(smoothing::net_tx_history());
    if rx_hist.is_empty() && tx_hist.is_empty() {
        return Vec::new();
    }
    let max_len = rx_hist.len().max(tx_hist.len());
    (0..max_len).map(|i| {
        rx_hist.get(i).copied().unwrap_or(0)
            + tx_hist.get(i).copied().unwrap_or(0)
    }).collect()
}

fn to_u64(data: Vec<f32>) -> Vec<u64> {
    data.iter().map(|v| (*v * 10.0) as u64).collect()
}
