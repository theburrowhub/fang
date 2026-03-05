/// Make panel component stub.
/// Displays Makefile targets and command output.
pub struct MakePanel {
    pub output_lines: Vec<String>,
    pub is_running: bool,
}

impl MakePanel {
    pub fn new() -> Self {
        Self {
            output_lines: Vec::new(),
            is_running: false,
        }
    }

    pub fn add_line(&mut self, line: String) {
        self.output_lines.push(line);
    }

    pub fn clear(&mut self) {
        self.output_lines.clear();
        self.is_running = false;
    }
}

impl Default for MakePanel {
    fn default() -> Self {
        Self::new()
    }
}
