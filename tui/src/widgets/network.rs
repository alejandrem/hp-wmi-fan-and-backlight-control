// ─── Network Widget ─────────────────────────────────────────────────────
// Shows IP, download/upload rates with peaks and totals, and sparklines.
// Layout matches btop: download section with V arrow, upload with A arrow,
// sparkline graph for historical trend.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
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
    render_net_download(frame, net, &rows[1..]);
    render_net_upload(frame, net, &rows[1..]);
    draw_net_sparkline(frame, sensors, rows.last().copied().unwrap_or(area));
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
            Constraint::Length(1), // IP
            Constraint::Length(1), // Download rate
            Constraint::Length(1), // Download peak/total
            Constraint::Length(1), // Upload rate
            Constraint::Length(1), // Upload peak/total
            Constraint::Min(1),    // Sparkline
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

fn render_net_download(
    frame: &mut Frame, net: &crate::sensors::NetworkInfo, rows: &[Rect],
) {
    let rate_row = rows.first().copied();
    let detail_row = rows.get(1).copied();
    draw_download_rate(frame, rate_row, net);
    draw_download_detail(frame, detail_row, net);
}

fn draw_download_rate(
    frame: &mut Frame, area: Option<Rect>, net: &crate::sensors::NetworkInfo,
) {
    let Some(area) = area else { return };
    let text = format!("V {:.1} KiB/s", net.download_rate);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn draw_download_detail(
    frame: &mut Frame, area: Option<Rect>, net: &crate::sensors::NetworkInfo,
) {
    let Some(area) = area else { return };
    let peak = format_rate(net.download_peak);
    let total = format_bytes(net.download_total);
    let text = Line::from(vec![
        Span::styled(format!("V Top: {}", peak), Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(format!("V Total: {}", total), Style::default().fg(Color::DarkGray)),
    ]);
    let widget = Paragraph::new(text);
    frame.render_widget(widget, area);
}

fn render_net_upload(
    frame: &mut Frame, net: &crate::sensors::NetworkInfo, rows: &[Rect],
) {
    let rate_row = rows.get(2).copied();
    let detail_row = rows.get(3).copied();
    draw_upload_rate(frame, rate_row, net);
    draw_upload_detail(frame, detail_row, net);
}

fn draw_upload_rate(
    frame: &mut Frame, area: Option<Rect>, net: &crate::sensors::NetworkInfo,
) {
    let Some(area) = area else { return };
    let text = format!("A {:.1} KiB/s", net.upload_rate);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn draw_upload_detail(
    frame: &mut Frame, area: Option<Rect>, net: &crate::sensors::NetworkInfo,
) {
    let Some(area) = area else { return };
    let peak = format_rate(net.upload_peak);
    let total = format_bytes(net.upload_total);
    let text = Line::from(vec![
        Span::styled(format!("A Top: {}", peak), Style::default().fg(Color::Red)),
        Span::raw("  "),
        Span::styled(format!("A Total: {}", total), Style::default().fg(Color::DarkGray)),
    ]);
    let widget = Paragraph::new(text);
    frame.render_widget(widget, area);
}

fn draw_net_sparkline(
    frame: &mut Frame, _sensors: &Sensors, area: Rect,
) {
    let combined = build_net_history();
    if combined.is_empty() {
        return;
    }
    let sparkline = Sparkline::default()
        .data(&combined)
        .style(Color::Cyan);
    frame.render_widget(sparkline, area);
}

fn build_net_history() -> Vec<u64> {
    use crate::sensors::smoothing;
    let rx_hist = smoothing::net_rx_history();
    let tx_hist = smoothing::net_tx_history();
    if rx_hist.is_empty() && tx_hist.is_empty() {
        return Vec::new();
    }
    let max_len = rx_hist.len().max(tx_hist.len());
    let combined: Vec<u64> = (0..max_len).map(|i| {
        let rx = rx_hist.get(i).copied().unwrap_or(0.0);
        let tx = tx_hist.get(i).copied().unwrap_or(0.0);
        (rx + tx) as u64
    }).collect();
    reverse_for_right_to_left(&combined)
}

fn reverse_for_right_to_left(data: &[u64]) -> Vec<u64> {
    data.iter().rev().copied().collect()
}

fn format_rate(rate: f64) -> String {
    if rate >= 1024.0 {
        format!("{:.1} MiB/s", rate / 1024.0)
    } else {
        format!("{:.1} KiB/s", rate)
    }
}

fn format_bytes(bytes: f64) -> String {
    let gib = bytes / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{:.2} GiB", gib)
    } else {
        let mib = bytes / 1024.0 / 1024.0;
        format!("{:.0} MiB", mib)
    }
}
