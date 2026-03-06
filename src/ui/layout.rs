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

    // If AI provider selection is active, render main + modal overlay
    if matches!(state.mode, AppMode::AiProviderSelect { .. }) {
        render_main_panels(frame, main_area, state);
        render_ai_modal(frame, area, state);
        return;
    }

    render_main_panels(frame, main_area, state);
}

fn render_main_panels(frame: &mut Frame, area: Rect, state: &AppState) {
    let width = area.width;
    let show_sidebar = state.sidebar_visible && width >= 80;
    let show_preview = state.preview_visible && !state.ai_panel_visible;
    let show_ai = state.ai_panel_visible;
    // Show both preview and AI side-by-side when both are enabled and enough width.
    let show_both_right = state.preview_visible && state.ai_panel_visible && width >= 120;

    // Layout percentages from config.
    let s = state.config.layout.sidebar_pct as u32;
    let l = state.config.layout.file_list_pct as u32;
    let p = state.config.layout.preview_pct() as u32;

    if show_sidebar {
        if show_both_right {
            // sidebar + file_list + preview + AI — split right half evenly
            let [sidebar_area, list_area, right_area] = Layout::horizontal([
                Constraint::Percentage(s as u16),
                Constraint::Percentage(l as u16),
                Constraint::Min(0),
            ])
            .areas(area);
            let [preview_area, ai_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(right_area);
            components::sidebar::render(frame, sidebar_area, state);
            components::file_list::render(frame, list_area, state);
            components::preview::render(frame, preview_area, state);
            components::ai_panel::render(frame, ai_area, state);
        } else if show_preview {
            // sidebar + file_list + preview
            let [sidebar_area, list_area, preview_area] = Layout::horizontal([
                Constraint::Percentage(s as u16),
                Constraint::Percentage(l as u16),
                Constraint::Percentage(p as u16),
            ])
            .areas(area);
            components::sidebar::render(frame, sidebar_area, state);
            components::file_list::render(frame, list_area, state);
            components::preview::render(frame, preview_area, state);
        } else if show_ai {
            // sidebar + file_list + AI (AI takes the preview slot)
            let [sidebar_area, list_area, ai_area] = Layout::horizontal([
                Constraint::Percentage(s as u16),
                Constraint::Percentage(l as u16),
                Constraint::Min(0),
            ])
            .areas(area);
            components::sidebar::render(frame, sidebar_area, state);
            components::file_list::render(frame, list_area, state);
            components::ai_panel::render(frame, ai_area, state);
        } else {
            // sidebar + file_list only
            let [sidebar_area, list_area] =
                Layout::horizontal([Constraint::Percentage(s as u16), Constraint::Min(0)])
                    .areas(area);
            components::sidebar::render(frame, sidebar_area, state);
            components::file_list::render(frame, list_area, state);
        }
    } else {
        // No sidebar — redistribute sidebar share into the remaining panels.
        let lp = l + p;
        let l2 = if lp > 0 { l * 100 / lp } else { 33 };

        if show_both_right {
            // file_list + preview + AI
            let [list_area, right_area] =
                Layout::horizontal([Constraint::Percentage(l2 as u16), Constraint::Min(0)])
                    .areas(area);
            let [preview_area, ai_area] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(right_area);
            components::file_list::render(frame, list_area, state);
            components::preview::render(frame, preview_area, state);
            components::ai_panel::render(frame, ai_area, state);
        } else if show_preview && width >= 50 {
            // file_list + preview
            let [list_area, preview_area] =
                Layout::horizontal([Constraint::Percentage(l2 as u16), Constraint::Min(0)])
                    .areas(area);
            components::file_list::render(frame, list_area, state);
            components::preview::render(frame, preview_area, state);
        } else if show_ai && width >= 50 {
            // file_list + AI
            let [list_area, ai_area] =
                Layout::horizontal([Constraint::Percentage(l2 as u16), Constraint::Min(0)])
                    .areas(area);
            components::file_list::render(frame, list_area, state);
            components::ai_panel::render(frame, ai_area, state);
        } else {
            // Only file list
            components::file_list::render(frame, area, state);
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

fn render_ai_modal(frame: &mut Frame, area: Rect, state: &AppState) {
    let provider_count = components::ai_modal::provider_count(state).max(1) as u16;

    let modal_width = (area.width * 2 / 3).clamp(50, 70);
    let modal_height = provider_count
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

    components::ai_modal::render(frame, modal_area, state);
}
