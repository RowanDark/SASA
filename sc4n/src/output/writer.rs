use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use crate::scanner::ScanResult;

pub struct ResultWriter {
    writer: Mutex<BufWriter<File>>,
}

impl ResultWriter {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self { writer: Mutex::new(BufWriter::new(file)) })
    }

    pub fn write(&self, result: &ScanResult) -> anyhow::Result<()> {
        let mut w = self.writer.lock().unwrap();
        let line = serde_json::to_string(result)?;
        writeln!(w, "{}", line)?;
        Ok(())
    }
}
