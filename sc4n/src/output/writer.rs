use std::fs::OpenOptions;
use std::io::Write;
use crate::scanner::ScanResult;

pub struct ResultWriter {
    path: String,
}

impl ResultWriter {
    pub fn new(path: &str) -> Self {
        Self { path: path.to_string() }
    }

    pub fn write(&self, result: &ScanResult) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(result)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }
}
