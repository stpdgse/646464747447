use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::client::{join_once, ping_session, status_once};
use crate::report::Report;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Handshake, status request and ping on a fresh connection.
    Status,
    /// Log in and complete configuration up to the first play packet.
    Join,
    /// One long-lived connection per bot, ping/pong in a loop (steady-state throughput, no
    /// connection churn). Every ping/pong counts as one operation.
    Ping,
}

#[derive(Clone, Debug)]
pub struct RunConfig {
    pub addr: SocketAddr,
    pub mode: Mode,
    /// Number of bots running at once.
    pub concurrency: usize,
    /// Stop after this long...
    pub duration: Duration,
    /// ...or after this many operations, whichever comes first.
    pub total: Option<u64>,
}

/// Runs the load and gathers the results. Each bot opens a connection, does one operation, closes
/// it, and starts over, so the numbers include accept and teardown costs.
pub async fn run(cfg: RunConfig) -> Report {
    let started = Instant::now();
    let deadline = started + cfg.duration;
    let issued = Arc::new(AtomicU64::new(0));

    let mut bots = Vec::with_capacity(cfg.concurrency);
    for bot in 0..cfg.concurrency {
        let (cfg, issued) = (cfg.clone(), issued.clone());
        bots.push(tokio::spawn(async move {
            let mut report = Report::default();
            if cfg.mode == Mode::Ping {
                let stop = cfg.total.map(|t| t / cfg.concurrency.max(1) as u64);
                let mut done = 0u64;
                let result = ping_session(cfg.addr, deadline, |latency| {
                    // Past a per-bot quota the bot just stops recording; the deadline ends it.
                    if stop.is_none_or(|s| done < s) {
                        done += 1;
                        report.ok += 1;
                        report.latencies.push(latency);
                    }
                })
                .await;
                if let Err(e) = result {
                    report.failed += 1;
                    report.sample_errors.push(e.to_string());
                }
                return report;
            }
            while Instant::now() < deadline {
                let n = issued.fetch_add(1, Ordering::Relaxed);
                if cfg.total.is_some_and(|t| n >= t) {
                    break;
                }
                let result = match cfg.mode {
                    Mode::Status => status_once(cfg.addr).await,
                    Mode::Ping => unreachable!("handled above"),
                    Mode::Join => {
                        // Names are at most 16 characters: `b<bot>_<seq>` in hex.
                        join_once(cfg.addr, &format!("b{bot:x}_{n:x}")).await.map(|o| o.latency)
                    }
                };
                match result {
                    Ok(latency) => {
                        report.ok += 1;
                        report.latencies.push(latency);
                    }
                    Err(e) => {
                        report.failed += 1;
                        if report.sample_errors.len() < 3 {
                            report.sample_errors.push(e.to_string());
                        }
                    }
                }
            }
            report
        }));
    }

    let mut total = Report::default();
    for bot in bots {
        let r = bot.await.expect("a bot task panicked");
        total.ok += r.ok;
        total.failed += r.failed;
        total.latencies.extend(r.latencies);
        for e in r.sample_errors {
            if total.sample_errors.len() < 3 && !total.sample_errors.contains(&e) {
                total.sample_errors.push(e);
            }
        }
    }
    total.elapsed = started.elapsed();
    total.latencies.sort_unstable();
    total
}
