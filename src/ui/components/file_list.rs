use crate::app::state::{AppMode, AppState, FileEntry, FocusedPanel};
use crate::ui::utils::{format_size_compact, panel_border_style};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

/// Return an icon prefix and style for a file entry.
fn entry_style(entry: &FileEntry) -> (&'static str, Style) {
    if entry.is_dir {
        (
            "\u{25B6} ",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
    } else if entry.is_symlink {
        (
            "\u{21AA} ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )
    } else if entry.is_executable {
        ("\u{26A1} ", Style::default().fg(Color::Green))
    } else {
        ("\u{00B7} ", Style::default().fg(Color::White))
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let border_style = panel_border_style(state.focused_panel == FocusedPanel::FileList);

    // Build title
    let dir_name = state
        .current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("/");

    // Count git changes for the title summary
    let (n_modified, n_added, n_other) = {
        use crate::app::state::GitFileStatus;
        let mut m = 0usize;
        let mut a = 0usize;
        let mut o = 0usize;
        for s in state.git_file_status.values() {
            match s {
                GitFileStatus::Modified => m += 1,
                GitFileStatus::Added | GitFileStatus::Untracked => a += 1,
                _ => o += 1,
            }
        }
        (m, a, o)
    };

    let git_summary = if n_modified + n_added + n_other > 0 {
        let mut parts = Vec::new();
        if n_modified > 0 {
            parts.push(format!("~{}", n_modified));
        }
        if n_added > 0 {
            parts.push(format!("+{}", n_added));
        }
        if n_other > 0 {
            parts.push(format!("!{}", n_other));
        }
        format!(" [{}]", parts.join(" "))
    } else {
        String::new()
    };

    let title = if !state.search_query.is_empty() {
        format!(
            " {}{} [{}/{}] ",
            dir_name,
            git_summary,
            state.filtered_indices.len(),
            state.entries.len()
        )
    } else {
        format!(" {}{} ({}) ", dir_name, git_summary, state.entries.len())
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);

    // Visible width for text (minus icon + git-status indicator + size columns)
    let available_width = inner_area.width.saturating_sub(2) as usize;
    let size_width = 5usize;
    let icon_width = 2usize;
    let git_width = 2usize; // "~ " / "+ " / "  "
    let name_width = available_width.saturating_sub(size_width + icon_width + git_width);

    // Collect visible entries once to avoid double allocation
    let visible = state.visible_entries();

    let items: Vec<ListItem> = visible
        .iter()
        .map(|entry| {
            let (icon, style) = entry_style(entry);

            // Truncate name if it overflows the column
            let name = if entry.name.len() > name_width && name_width > 3 {
                format!("{}...", &entry.name[..name_width.saturating_sub(3)])
            } else {
                entry.name.clone()
            };

            let size_str = if entry.is_dir {
                "  dir".to_string()
            } else {
                format_size_compact(entry.size)
            };

            // Pad name to name_width for alignment
            let padded_name = format!("{:<width$}", name, width = name_width);

            // Git status indicator (1 char + space), blank when clean.
            // Also apply the git colour to the filename so changes are
            // immediately visible without scanning the indicator column.
            let (git_char, git_style) = state
                .git_file_status
                .get(&entry.path)
                .map(|s| (s.indicator(), s.style()))
                .unwrap_or((' ', Style::default()));

            // Directories keep their blue colour; files adopt the git colour.
            let name_style = if git_char != ' ' && !entry.is_dir {
                git_style
            } else {
                style
            };

            let line = Line::from(vec![
                Span::styled(icon, style),
                Span::styled(padded_name, name_style),
                Span::styled(format!("{} ", git_char), git_style),
                Span::styled(
                    format!("{:>5}", size_str),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let highlight_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !visible.is_empty() {
        list_state.select(Some(state.selected_index));
    }

    frame.render_stateful_widget(list, area, &mut list_state);

    // If in search mode, overlay the query bar at the bottom of the panel
    if let AppMode::Search { query } = &state.mode {
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(2),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let search_line = Line::from(vec![
            Span::styled("/ ", Style::default().fg(Color::Yellow)),
            Span::styled(query.clone(), Style::default().fg(Color::White)),
        ]);
        frame.render_widget(search_line, search_area);
    }
}
