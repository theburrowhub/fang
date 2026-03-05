use ratatui::prelude::*;

pub fn format_size_compact(bytes: u64) -> String {
    const KB: u64 = 1024; const MB: u64 = KB * 1024; const GB: u64 = MB * 1024;
    if bytes == 0 { "  -".to_string() }
    else if bytes < KB { format!("{:>4}B", bytes) }
    else if bytes < MB { format!("{:>3}K", bytes / KB) }
    else if bytes < GB { format!("{:>3}M", bytes / MB) }
    else { format!("{:>3}G", bytes / GB) }
}

pub fn format_size_verbose(bytes: u64) -> String {
    const KB: u64 = 1024; const MB: u64 = KB * 1024; const GB: u64 = MB * 1024;
    if bytes < KB { format!("{} B", bytes) }
    else if bytes < MB { format!("{:.1} KB", bytes as f64 / KB as f64) }
    else if bytes < GB { format!("{:.1} MB", bytes as f64 / MB as f64) }
    else { format!("{:.1} GB", bytes as f64 / GB as f64) }
}

pub fn key_hint<'a>(key: &'a str, desc: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled(key, Style::default().fg(Color::Yellow)),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
        Span::styled(desc, Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
    ]
}

pub fn panel_border_style(focused: bool) -> Style {
    if focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_format_size_compact_zero() { assert_eq!(format_size_compact(0), "  -"); }
    #[test]
    fn test_format_size_compact_kb() { assert_eq!(format_size_compact(1024), "  1K"); }
    #[test]
    fn test_format_size_verbose() { assert!(format_size_verbose(1024).contains("1.0 KB")); }
}
