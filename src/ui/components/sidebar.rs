use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use crate::app::state::{AppState, FocusedPanel};
use crate::ui::utils::panel_border_style;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let border_style = panel_border_style(state.focused_panel == FocusedPanel::Sidebar);

    let dir_name = state.current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("/");
    let title = format!(" {} ", dir_name);

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style);

    // If full tree nodes are populated, render them; otherwise fall back to
    // the breadcrumb view derived from current_dir components.
    let (sidebar_items, selected_idx): (Vec<ListItem>, Option<usize>) =
        if !state.sidebar_tree.is_empty() {
            let items = state
                .sidebar_tree
                .iter()
                .enumerate()
                .map(|(i, node)| {
                    let is_selected = i == state.sidebar_selected;
                    let indent = "  ".repeat(node.depth);
                    let icon = if node.is_dir {
                        if node.is_expanded { "\u{25BC} " } else { "\u{25B6} " }
                    } else {
                        "\u{00B7} "
                    };

                    let name = node
                        .path
                        .file_name()
                        .and_then(|n: &std::ffi::OsStr| n.to_str())
                        .unwrap_or("/");

                    let base_style = if node.is_dir {
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let style = if is_selected {
                        base_style.bg(Color::DarkGray)
                    } else {
                        base_style
                    };

                    let line = Line::from(vec![
                        Span::raw(indent),
                        Span::styled(format!("{}{}", icon, name), style),
                    ]);
                    ListItem::new(line)
                })
                .collect();
            (items, Some(state.sidebar_selected))
        } else {
            // Breadcrumb view: each path component indented by its depth
            use std::path::Component;
            let components: Vec<String> = state
                .current_dir
                .components()
                .filter_map(|c| match c {
                    Component::RootDir => Some("/".to_string()),
                    Component::Normal(s) => Some(s.to_str().unwrap_or("?").to_string()),
                    Component::Prefix(p) => {
                        Some(p.as_os_str().to_str().unwrap_or("?").to_string())
                    }
                    _ => None,
                })
                .collect();

            let total = components.len();
            let items = components
                .into_iter()
                .enumerate()
                .map(|(i, name)| {
                    let is_last = i == total.saturating_sub(1);
                    let indent = "  ".repeat(i);
                    let icon = if is_last { "\u{25BC} " } else { "\u{25B6} " };
                    let style = if is_last {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Blue)
                    };
                    let line = Line::from(vec![
                        Span::raw(indent),
                        Span::styled(format!("{}{}", icon, name), style),
                    ]);
                    ListItem::new(line)
                })
                .collect();
            let selected = if total > 0 { Some(total.saturating_sub(1)) } else { None };
            (items, selected)
        };

    let highlight_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let list = List::new(sidebar_items)
        .block(block)
        .highlight_style(highlight_style);

    let mut list_state = ListState::default();
    list_state.select(selected_idx);

    frame.render_stateful_widget(list, area, &mut list_state);
}
