use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use crate::app::state::{AppState, AppMode, FocusedPanel, FileEntry};
use crate::ui::utils::{format_size_compact, panel_border_style};

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

    let title = if !state.search_query.is_empty() {
        format!(
            " {} [{}/{}] ",
            dir_name,
            state.filtered_indices.len(),
            state.entries.len()
        )
    } else {
        format!(" {} ({}) ", dir_name, state.entries.len())
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);

    // Visible width for text (minus icon + size columns)
    let available_width = inner_area.width.saturating_sub(2) as usize;
    let size_width = 5usize;
    let icon_width = 2usize;
    let name_width = available_width.saturating_sub(size_width + icon_width);

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

            let line = Line::from(vec![
                Span::styled(icon, style),
                Span::styled(padded_name, style),
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
