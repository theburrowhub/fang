//! Persistent application configuration: `~/.config/fang/config.toml`
//!
//! Compatible with the `[ai]` section that PR #18 (AI integration) introduces.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Layout settings ──────────────────────────────────────────────────────────

/// Panel size and visibility configuration.
///
/// Size invariant: `file_list_pct + preview_pct == 100`
/// (`preview_pct` is derived and not stored on disk.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// File-list panel width (%, default 35).
    #[serde(default = "default_file_list_pct")]
    pub file_list_pct: u16,

    /// Whether the preview panel is visible on launch (default true).
    #[serde(default = "default_true")]
    pub preview_visible: bool,
}

impl LayoutConfig {
    pub fn preview_pct(&self) -> u16 {
        100u16.saturating_sub(self.file_list_pct)
    }

    pub fn clamp(&mut self) {
        self.file_list_pct = self.file_list_pct.clamp(10, 90);
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            file_list_pct: default_file_list_pct(),
            preview_visible: true,
        }
    }
}

fn default_file_list_pct() -> u16 {
    35
}
fn default_true() -> bool {
    true
}

// ── MSLP (skip permissions) ──────────────────────────────────────────────────

/// When enabled, the Claude Code CLI is invoked with `--dangerously-skip-permissions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MslpConfig {
    #[serde(default)]
    pub enabled: bool,
}

// ── Top-level config file ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub mslp: MslpConfig,
}

// ── Persistence ───────────────────────────────────────────────────────────────

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("fang").join("config.toml"))
}

pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };
    let mut cfg: Config = toml::from_str(&content).unwrap_or_default();
    cfg.layout.clamp();
    cfg
}

pub fn save(config: &Config) -> Result<(), String> {
    let path = config_path().ok_or("cannot determine config directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

// ── Editable settings list ────────────────────────────────────────────────────

/// How the value of a `SettingEntry` behaves in the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    /// Numeric range — +/- change by 1.
    Editable { min: u16, max: u16 },
    /// Derived / read-only (shown in gray, cannot be edited).
    Derived,
    /// On/off toggle — any edit key flips 0 ↔ 1.
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingEntry {
    pub key: &'static str,
    pub description: &'static str,
    /// For numeric entries: the current integer value.
    /// For Toggle entries: 0 = off, 1 = on.
    pub value: u16,
    pub kind: EntryKind,
}

impl SettingEntry {
    pub fn increment(&mut self) {
        match self.kind {
            EntryKind::Editable { min: _, max } => {
                if self.value < max {
                    self.value += 1;
                }
            }
            EntryKind::Toggle => {
                self.value = if self.value == 0 { 1 } else { 0 };
            }
            EntryKind::Derived => {}
        }
    }
    pub fn decrement(&mut self) {
        match self.kind {
            EntryKind::Editable { min, max: _ } => {
                if self.value > min {
                    self.value -= 1;
                }
            }
            EntryKind::Toggle => {
                self.value = if self.value == 0 { 1 } else { 0 };
            }
            EntryKind::Derived => {}
        }
    }
    pub fn is_editable(&self) -> bool {
        !matches!(self.kind, EntryKind::Derived)
    }
    #[allow(dead_code)]
    pub fn is_toggle(&self) -> bool {
        matches!(self.kind, EntryKind::Toggle)
    }
    pub fn as_bool(&self) -> bool {
        self.value != 0
    }
}

pub fn entries_from_config(cfg: &Config) -> Vec<SettingEntry> {
    let preview = cfg.layout.preview_pct();
    vec![
        // ── Panel sizes ───────────────────────────────────────────────────
        SettingEntry {
            key: "layout.file_list_pct",
            description: "File list width",
            value: cfg.layout.file_list_pct,
            kind: EntryKind::Editable { min: 10, max: 90 },
        },
        SettingEntry {
            key: "layout.preview_pct",
            description: "Preview width",
            value: preview,
            kind: EntryKind::Derived,
        },
        // ── Panel visibility (default state on launch) ────────────────────
        SettingEntry {
            key: "layout.preview_visible",
            description: "Preview visible on launch",
            value: if cfg.layout.preview_visible { 1 } else { 0 },
            kind: EntryKind::Toggle,
        },
        // ── MSLP (Claude CLI skip permissions) ──────────────────────────
        SettingEntry {
            key: "mslp.enabled",
            description: "Skip Claude permissions (dangerous)",
            value: if cfg.mslp.enabled { 1 } else { 0 },
            kind: EntryKind::Toggle,
        },
    ]
}

pub fn apply_entries(cfg: &mut Config, entries: &[SettingEntry]) {
    for e in entries {
        match e.key {
            "layout.file_list_pct" => cfg.layout.file_list_pct = e.value,
            "layout.preview_visible" => cfg.layout.preview_visible = e.as_bool(),
            "mslp.enabled" => cfg.mslp.enabled = e.as_bool(),
            _ => {}
        }
    }
    cfg.layout.clamp();
}

pub fn refresh_derived(entries: &mut [SettingEntry], cfg: &Config) {
    let preview = cfg.layout.preview_pct();
    if let Some(e) = entries.iter_mut().find(|e| e.key == "layout.preview_pct") {
        e.value = preview;
    }
}
