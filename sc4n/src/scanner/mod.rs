pub mod rate;
pub mod tcp;

use crate::profiles::ScanProfile;
use tcp::TcpScanner;

pub use tcp::{ScanResult, PortStatus};

pub struct Scanner {
    pub profile: ScanProfile,
}

impl Scanner {
    pub fn new(profile: ScanProfile) -> Self {
        Self { profile }
    }

    pub fn tcp(&self) -> TcpScanner {
        TcpScanner::new(self.profile.clone())
    }
}
