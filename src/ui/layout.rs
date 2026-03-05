use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use crate::app::state::AppState;

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let block = Block::default()
        .title(" Fang ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = format!(
        "Dir: {}\nEntries: {}\nMode: {:?}\nPress 'q' to quit, '/' to search, 'm' for make",
        state.current_dir.display(),
        state.entries.len(),
        state.mode,
    );
    let para = Paragraph::new(text);
    frame.render_widget(para, inner);
}
