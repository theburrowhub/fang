use ratatui::prelude::*;

/// Format bytes as a compact human-readable string for file lists.
/// Right-aligned, 5 chars wide: "  -", "123B", " 12K", " 1M", etc.
pub fn format_size_compact(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes == 0 {
        "  -".to_string()
    } else if bytes < KB {
        format!("{:>4}B", bytes)
    } else if bytes < MB {
        format!("{:>3}K", bytes / KB)
    } else if bytes < GB {
        format!("{:>3}M", bytes / MB)
    } else {
        format!("{:>3}G", bytes / GB)
    }
}

/// Format bytes as a human-readable string with decimal precision for display.
/// E.g. "1.2 KB", "3.4 MB".
pub fn format_size_verbose(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}

/// Build spans for a key hint: `[KEY] desc  `
pub fn key_hint<'a>(key: &'a str, desc: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled(key, Style::default().fg(Color::Yellow)),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
        Span::styled(desc, Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
    ]
}

/// Compute the border style for a panel: Cyan when focused, DarkGray otherwise.
pub fn panel_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}
