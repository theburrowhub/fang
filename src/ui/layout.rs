use super::components;
use crate::app::state::{AppMode, AppState};
use ratatui::prelude::*;
use ratatui::widgets::Clear;

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Reserve 1 line for header, 1 line for footer (keybindings only)
    let [header_area, main_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    components::header::render(frame, header_area, state);
    components::footer::render(frame, footer_area, state);

    // Git form overlay (second screen)
    if matches!(state.mode, AppMode::GitForm { .. }) {
        render_main_panels(frame, main_area, state);
        render_git_modal(frame, area, state); // keep first screen dimmed behind
        components::git_form::render(frame, area, state);
        return;
    }

    // Full-screen Help overlay
    if let AppMode::Help { scroll } = state.mode {
        components::help::render(frame, area, scroll);
        return;
    }

    // If settings modal is active, render main + settings overlay
    if matches!(state.mode, AppMode::Settings { .. }) {
        render_main_panels(frame, main_area, state);
        components::settings_modal::render(frame, area, state);
        return;
    }

    // If make modal is active, render main + modal overlay
    if state.mode == AppMode::MakeTarget {
        render_main_panels(frame, main_area, state);
        render_make_modal(frame, area, state);
        return;
    }

    // If git menu is active, render main + modal overlay
    if matches!(state.mode, AppMode::GitMenu { .. }) {
        render_main_panels(frame, main_area, state);
        render_git_modal(frame, area, state);
        return;
    }

    render_main_panels(frame, main_area, state);
}

fn render_main_panels(frame: &mut Frame, area: Rect, state: &AppState) {
    let width = area.width;

    if !state.sidebar_visible || width < 80 {
        // No sidebar: redistribute sidebar share into preview so the ratio
        // file_list : preview is preserved.
        let s = state.config.layout.sidebar_pct as u32;
        let l = state.config.layout.file_list_pct as u32;
        let p = state.config.layout.preview_pct() as u32;
        // Scale l and p to fill 100 % without the sidebar
        let lp = l + p;
        let l2 = if lp > 0 { l * 100 / lp } else { 33 };

        if !state.preview_visible || width < 50 {
            components::file_list::render(frame, area, state);
        } else {
            let [list_area, preview_area] =
                Layout::horizontal([Constraint::Percentage(l2 as u16), Constraint::Min(0)])
                    .areas(area);
            components::file_list::render(frame, list_area, state);
            components::preview::render(frame, preview_area, state);
        }
        let _ = s; // suppress warning when sidebar is hidden
    } else {
        // sidebar + file_list (+ optional preview) — all three in %.
        let s = state.config.layout.sidebar_pct as u32;
        let l = state.config.layout.file_list_pct as u32;
        let p = state.config.layout.preview_pct() as u32;

        if state.preview_visible {
            let [sidebar_area, list_area, preview_area] = Layout::horizontal([
                Constraint::Percentage(s as u16),
                Constraint::Percentage(l as u16),
                Constraint::Percentage(p as u16),
            ])
            .areas(area);
            components::sidebar::render(frame, sidebar_area, state);
            components::file_list::render(frame, list_area, state);
            components::preview::render(frame, preview_area, state);
        } else {
            // No preview: sidebar keeps its %, file_list takes the rest.
            let [sidebar_area, list_area] =
                Layout::horizontal([Constraint::Percentage(s as u16), Constraint::Min(0)])
                    .areas(area);
            components::sidebar::render(frame, sidebar_area, state);
            components::file_list::render(frame, list_area, state);
            let _ = l;
            let _ = p;
        }
    }
}

fn render_make_modal(frame: &mut Frame, area: Rect, state: &AppState) {
    // Calculate modal size
    let modal_width = (area.width * 2 / 3).clamp(40, 70);
    let modal_height = ((state.make_targets.len() as u16) + 6)
        .min(area.height.saturating_sub(4))
        .max(8);

    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect {
        x,
        y,
        width: modal_width,
        height: modal_height,
    };

    // Clear area behind modal
    frame.render_widget(Clear, modal_area);

    // Modal content
    components::make_modal::render(frame, modal_area, state);
}

fn render_git_modal(frame: &mut Frame, area: Rect, state: &AppState) {
    use crate::commands::git::N_GIT_OPS;

    let op_count = N_GIT_OPS as u16;
    let modal_width = (area.width * 2 / 3).clamp(50, 70);
    let modal_height = op_count
        .saturating_mul(2)
        .saturating_add(6)
        .min(area.height.saturating_sub(4))
        .max(10);

    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect {
        x,
        y,
        width: modal_width,
        height: modal_height,
    };

    frame.render_widget(Clear, modal_area);
    components::git_modal::render(frame, modal_area, state);
}
