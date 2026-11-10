use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use clap::{Parser, ValueEnum};
use ferrite_swarm::{Mode, RunConfig, run};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ModeArg {
    Status,
    Join,
    Ping,
}

/// Load-tests a Minecraft Java server with protocol-correct bots.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Server address, host:port.
    #[arg(long, default_value = "127.0.0.1:25565")]
    addr: String,
    /// What each bot does per connection.
    #[arg(long, value_enum, default_value = "status")]
    mode: ModeArg,
    /// Bots running at once.
    #[arg(long, short, default_value_t = 64)]
    connections: usize,
    /// Test length in seconds.
    #[arg(long, short, default_value_t = 10)]
    duration: u64,
    /// Stop after this many operations.
    #[arg(long)]
    total: Option<u64>,
    /// Tokio worker threads for the load generator itself.
    #[arg(long, default_value_t = 2)]
    threads: usize,
}

fn main() {
    let args = Args::parse();
    let addr: SocketAddr = args
        .addr
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .unwrap_or_else(|| panic!("cannot resolve {}", args.addr));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let report = runtime.block_on(run(RunConfig {
        addr,
        mode: match args.mode {
            ModeArg::Status => Mode::Status,
            ModeArg::Join => Mode::Join,
            ModeArg::Ping => Mode::Ping,
        },
        concurrency: args.connections,
        duration: Duration::from_secs(args.duration),
        total: args.total,
    }));
    print!("{report}");
    if report.failed > 0 && report.ok == 0 {
        std::process::exit(1);
    }
}
