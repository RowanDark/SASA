use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use rand::seq::SliceRandom;
use rand::Rng;
use futures::stream::{FuturesUnordered, StreamExt};

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
}

impl TcpScanner {
    pub fn new(profile: ScanProfile) -> Self {
        let limiter = RateLimiter::new(profile.rate_per_sec);
        Self { profile, limiter }
    }

    pub async fn scan_ports(
        &self,
        host: &str,
        mut ports: Vec<u16>,
        debug: bool,
        tx: tokio::sync::mpsc::Sender<ScanResult>,
    ) -> anyhow::Result<()> {
        // Resolve hostname once before spawning any tasks (Perf #1)
        let target_ip: IpAddr = if let Ok(ip) = host.parse::<IpAddr>() {
            ip
        } else {
            // Pass an owned String so the future has no borrow lifetime on a local.
            tokio::net::lookup_host(format!("{}:0", host))
                .await?
                .next()
                .ok_or_else(|| anyhow::anyhow!("Could not resolve host: {}", host))?
                .ip()
        };

        if self.profile.randomize_order {
            let mut rng = rand::thread_rng();
            ports.shuffle(&mut rng);
        }

        // Gate spawn count to concurrency limit via FuturesUnordered (Perf #2)
        let max_in_flight = self.profile.concurrency;
        let mut futs: FuturesUnordered<_> = FuturesUnordered::new();
        let mut burst_count = 0usize;

        for port in ports {
            // Burst pause logic
            burst_count += 1;
            if self.profile.burst_size > 0
                && burst_count.is_multiple_of(self.profile.burst_size)
                && self.profile.burst_pause_ms > 0
            {
                tokio::time::sleep(Duration::from_millis(
                    self.profile.burst_pause_ms,
                ))
                .await;
            }

            // Gate spawning to concurrency limit
            while futs.len() >= max_in_flight {
                futs.next().await;
            }

            let limiter = self.limiter.clone();
            let tx = tx.clone();
            let host_str = host.to_string();
            let timeout_ms = self.profile.timeout_ms;
            let min_jitter = self.profile.min_jitter_ms;
            let max_jitter = self.profile.max_jitter_ms;

            futs.push(tokio::spawn(async move {
                limiter.acquire().await;

                if max_jitter > 0 {
                    let jitter = rand::thread_rng().gen_range(min_jitter..=max_jitter);
                    if jitter > 0 {
                        tokio::time::sleep(Duration::from_millis(jitter)).await;
                    }
                }

                let socket_addr = SocketAddr::new(target_ip, port);
                let start = std::time::Instant::now();
                let result = timeout(
                    Duration::from_millis(timeout_ms),
                    TcpStream::connect(socket_addr),
                )
                .await;
                let latency_ms = start.elapsed().as_millis() as u64;

                let status = match result {
                    Ok(Ok(_)) => PortStatus::Open,
                    Ok(Err(_)) => PortStatus::Closed,
                    Err(_) => PortStatus::Filtered,
                };

                if status == PortStatus::Open || debug {
                    let _ = tx
                        .send(ScanResult {
                            host: host_str,
                            port,
                            status,
                            latency_ms,
                        })
                        .await;
                }
            }));
        }

        // Drain remainder
        while futs.next().await.is_some() {}
        Ok(())
    }
}
