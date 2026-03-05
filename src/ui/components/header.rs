use ratatui::{prelude::*, widgets::Paragraph};
use crate::app::state::AppState;

/// Renders a 1-line header with app name, git branch badge, and dev env badges.
///
/// Layout:
///   Left:  " fang  [⎇ branch]  [badge]  [badge] "
///   Right: " /path/to/current/dir "
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.height == 0 {
        return;
    }

    // ── Left section: app name + badges ─────────────────────────────────────
    let mut left_spans: Vec<Span<'_>> = vec![
        Span::raw(" "),
        Span::styled("fang", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
    ];

    // Git branch badge — use Nerd Font branch icon (falls back gracefully for non-NF terminals)
    if let Some(branch) = &state.header_info.git_branch {
        left_spans.push(Span::styled(
            format!("\u{e0a0} {}", branch),
            Style::default().fg(Color::Yellow),
        ));
        left_spans.push(Span::raw("  "));
    }

    // Dev env badges
    for (name, version) in &state.header_info.dev_envs {
        left_spans.push(Span::styled(
            format!("{} {}", name, version),
            Style::default().fg(Color::Cyan),
        ));
        left_spans.push(Span::raw("  "));
    }

    // ── Right section: current path ─────────────────────────────────────────
    // to_string_lossy returns a Cow — format! directly into right_text to avoid a double alloc
    let right_text = format!("{}  ", state.current_dir.to_string_lossy());

    // Build full header line: left content + padding + right content.
    // Measure left width to calculate how much padding to add.
    let left_width: usize = left_spans.iter().map(|s| s.content.chars().count()).sum();
    let right_width = right_text.chars().count();
    let total_width = area.width as usize;
    let padding = total_width.saturating_sub(left_width + right_width);

    let mut spans = left_spans;
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }
    spans.push(Span::styled(right_text, Style::default().fg(Color::DarkGray)));

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Reset)),
        area,
    );
}
