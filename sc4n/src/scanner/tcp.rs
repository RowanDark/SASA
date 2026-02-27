use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::sync::Semaphore;
use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::profiles::ScanProfile;
use crate::scanner::rate::RateLimiter;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanResult {
    pub host: String,
    pub port: u16,
    pub status: PortStatus,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    Open,
    Closed,
    Filtered,
}

impl std::fmt::Display for PortStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PortStatus::Open => write!(f, "open"),
            PortStatus::Closed => write!(f, "closed"),
            PortStatus::Filtered => write!(f, "filtered"),
        }
    }
}

pub struct TcpScanner {
    profile: ScanProfile,
    limiter: RateLimiter,
    semaphore: Arc<Semaphore>,
}

impl TcpScanner {
    pub fn new(profile: ScanProfile) -> Self {
        let limiter = RateLimiter::new(profile.rate_per_sec);
        let semaphore = Arc::new(Semaphore::new(profile.concurrency));
        Self { profile, limiter, semaphore }
    }

    pub async fn scan_ports(
        &self,
        host: &str,
        mut ports: Vec<u16>,
        debug: bool,
        tx: tokio::sync::mpsc::Sender<ScanResult>,
    ) -> anyhow::Result<()> {
        if self.profile.randomize_order {
            let mut rng = rand::thread_rng();
            ports.shuffle(&mut rng);
        }

        let mut handles = Vec::new();
        let mut burst_count = 0;

        for port in ports {
            // Burst pause logic
            burst_count += 1;
            if self.profile.burst_size > 0
                && burst_count % self.profile.burst_size == 0
                && self.profile.burst_pause_ms > 0
            {
                tokio::time::sleep(Duration::from_millis(
                    self.profile.burst_pause_ms
                )).await;
            }

            let host = host.to_string();
            let limiter = self.limiter.clone();
            let semaphore = self.semaphore.clone();
            let tx = tx.clone();
            let timeout_ms = self.profile.timeout_ms;
            let min_jitter = self.profile.min_jitter_ms;
            let max_jitter = self.profile.max_jitter_ms;
            let debug = debug;

            let handle = tokio::spawn(async move {
                // Acquire rate limiter
                limiter.acquire().await;

                // Apply jitter
                if max_jitter > 0 {
                    let jitter = rand::thread_rng()
                        .gen_range(min_jitter..=max_jitter);
                    if jitter > 0 {
                        tokio::time::sleep(
                            Duration::from_millis(jitter)
                        ).await;
                    }
                }

                let _permit = semaphore.acquire().await.unwrap();

                let addr = format!("{}:{}", host, port);
                let socket_addr: SocketAddr = match addr.parse() {
                    Ok(a) => a,
                    Err(_) => {
                        // Try resolving hostname
                        match tokio::net::lookup_host(&addr).await {
                            Ok(mut addrs) => match addrs.next() {
                                Some(a) => a,
                                None => return,
                            },
                            Err(_) => return,
                        }
                    }
                };

                let start = std::time::Instant::now();
                let result = timeout(
                    Duration::from_millis(timeout_ms),
                    TcpStream::connect(socket_addr),
                ).await;

                let latency_ms = start.elapsed().as_millis() as u64;

                let status = match result {
                    Ok(Ok(_)) => PortStatus::Open,
                    Ok(Err(_)) => PortStatus::Closed,
                    Err(_) => PortStatus::Filtered,  // timeout
                };

                // Only send result if open, or debug mode
                if status == PortStatus::Open || debug {
                    let _ = tx.send(ScanResult {
                        host: host.clone(),
                        port,
                        status,
                        latency_ms,
                    }).await;
                }
            });

            handles.push(handle);
        }

        // Await all probes
        futures::future::join_all(handles).await;
        Ok(())
    }
}
