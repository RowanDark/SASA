use super::ScanProfile;

pub fn aggressive() -> ScanProfile {
    ScanProfile {
        name: "aggressive",
        concurrency: 1000,
        rate_per_sec: 0,          // unlimited
        timeout_ms: 500,
        min_jitter_ms: 0,
        max_jitter_ms: 10,
        burst_size: 10000,
        burst_pause_ms: 0,
        randomize_order: false,
        description: "Maximum speed. Use on your own infrastructure only.",
    }
}

pub fn balanced() -> ScanProfile {
    ScanProfile {
        name: "balanced",
        concurrency: 200,
        rate_per_sec: 500,
        timeout_ms: 1000,
        min_jitter_ms: 5,
        max_jitter_ms: 50,
        burst_size: 500,
        burst_pause_ms: 100,
        randomize_order: true,
        description: "Default profile. Good for authorized assessments.",
    }
}

pub fn stealth() -> ScanProfile {
    ScanProfile {
        name: "stealth",
        concurrency: 10,
        rate_per_sec: 20,
        timeout_ms: 2000,
        min_jitter_ms: 100,
        max_jitter_ms: 500,
        burst_size: 20,
        burst_pause_ms: 2000,
        randomize_order: true,
        description: "Low and slow. Designed to avoid IDS detection.",
    }
}

pub fn paranoid() -> ScanProfile {
    ScanProfile {
        name: "paranoid",
        concurrency: 1,
        rate_per_sec: 1,
        timeout_ms: 5000,
        min_jitter_ms: 10000,
        max_jitter_ms: 60000,
        burst_size: 1,
        burst_pause_ms: 30000,
        randomize_order: true,
        description: "One probe at a time with random long delays. Maximum stealth.",
    }
}
