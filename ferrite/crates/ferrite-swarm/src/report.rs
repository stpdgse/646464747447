use std::fmt;
use std::time::Duration;

/// Results of a run.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub ok: u64,
    pub failed: u64,
    pub elapsed: Duration,
    /// Latency of each successful operation, sorted ascending.
    pub latencies: Vec<Duration>,
    /// The first few distinct error messages, for diagnosis.
    pub sample_errors: Vec<String>,
}

impl Report {
    pub fn per_second(&self) -> f64 {
        self.ok as f64 / self.elapsed.as_secs_f64().max(f64::EPSILON)
    }

    /// The `p`th percentile (0.0..=1.0), or zero if nothing succeeded.
    pub fn percentile(&self, p: f64) -> Duration {
        if self.latencies.is_empty() {
            return Duration::ZERO;
        }
        let idx = ((self.latencies.len() as f64 - 1.0) * p).round() as usize;
        self.latencies[idx]
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let us = |d: Duration| d.as_secs_f64() * 1e6;
        writeln!(f, "ok:        {}", self.ok)?;
        writeln!(f, "failed:    {}", self.failed)?;
        writeln!(f, "elapsed:   {:.2}s", self.elapsed.as_secs_f64())?;
        writeln!(f, "rate:      {:.0} ops/s", self.per_second())?;
        writeln!(
            f,
            "latency:   p50 {:.0}us  p90 {:.0}us  p99 {:.0}us  p99.9 {:.0}us  max {:.0}us",
            us(self.percentile(0.50)),
            us(self.percentile(0.90)),
            us(self.percentile(0.99)),
            us(self.percentile(0.999)),
            us(self.percentile(1.0)),
        )?;
        for e in &self.sample_errors {
            writeln!(f, "error:     {e}")?;
        }
        Ok(())
    }
}
