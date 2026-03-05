use std::path::Path;
use anyhow::Result;

/// Returns a hex dump preview of a binary file (first `max_bytes` bytes).
pub fn hex_preview(path: &Path, max_bytes: usize) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let preview = &bytes[..bytes.len().min(max_bytes)];

    let mut output = String::new();
    for (i, chunk) in preview.chunks(16).enumerate() {
        let offset = i * 16;
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
            .collect();
        output.push_str(&format!("{:08x}  {:47}  |{}|\n", offset, hex.join(" "), ascii));
    }

    Ok(output)
}
