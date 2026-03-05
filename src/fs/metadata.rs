use crate::app::state::FileEntry;

pub fn format_size(_bytes: u64) -> String {
    "0 B".to_string()
}

pub fn get_file_icon(_entry: &FileEntry) -> &'static str {
    ""
}
