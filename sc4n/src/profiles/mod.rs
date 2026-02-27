pub mod builtin;

#[derive(Debug, Clone)]
pub struct ScanProfile {
    pub name: &'static str,
    pub concurrency: usize,
    pub rate_per_sec: u64,       // 0 = unlimited
    pub timeout_ms: u64,
    pub min_jitter_ms: u64,
    pub max_jitter_ms: u64,
    pub burst_size: usize,       // probes before mandatory pause
    pub burst_pause_ms: u64,     // how long to pause after burst
    pub randomize_order: bool,
    pub description: &'static str,
}

impl ScanProfile {
    /// Apply CLI overrides on top of profile defaults
    pub fn with_overrides(
        mut self,
        concurrency: Option<usize>,
        rate: Option<u64>,
        timeout_ms: Option<u64>,
        randomize: bool,
    ) -> Self {
        if let Some(c) = concurrency { self.concurrency = c; }
        if let Some(r) = rate { self.rate_per_sec = r; }
        if let Some(t) = timeout_ms { self.timeout_ms = t; }
        self.randomize_order = randomize;
        self
    }
}
