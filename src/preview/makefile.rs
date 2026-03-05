use std::path::Path;
use anyhow::Result;
use crate::app::state::MakeTarget;
use crate::commands::make;

/// Returns the list of targets parsed from a Makefile for preview purposes.
pub fn preview_targets(path: &Path) -> Result<Vec<MakeTarget>> {
    make::parse_targets(path)
}

/// Formats targets as a human-readable string for display.
pub fn format_targets(targets: &[MakeTarget]) -> String {
    targets
        .iter()
        .map(|t| {
            if let Some(desc) = &t.description {
                format!("{:20} {}", t.name, desc)
            } else {
                t.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
