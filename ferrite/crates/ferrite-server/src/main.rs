use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, ValueEnum};
use ferrite_net::{Lobby, LobbyConfig};
use ferrite_protocol::Compression;
use ferrite_protocol::session::SessionConfig;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Backend {
    /// Portable work-stealing runtime.
    Tokio,
    /// Thread-per-core io_uring (Linux only).
    Uring,
}

/// A from-scratch Minecraft: Java Edition server.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:25565")]
    addr: SocketAddr,
    #[arg(long, value_enum, default_value = "tokio")]
    backend: Backend,
    /// Worker threads (default: one per available core).
    #[arg(long)]
    threads: Option<usize>,
    #[arg(long, default_value = "A Ferrite server")]
    motd: String,
    #[arg(long, default_value_t = 20)]
    max_players: u32,
    /// Compress packets of at least this many bytes; -1 disables compression.
    #[arg(long, default_value_t = 256, allow_hyphen_values = true)]
    compression_threshold: i32,
    /// Seconds of silence after which a connection is dropped.
    #[arg(long, default_value_t = 15)]
    read_timeout: u64,
    /// Print counters every N seconds (0 disables).
    #[arg(long, default_value_t = 0)]
    stats_interval: u64,
}

fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    let threads = args
        .threads
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(1);

    let lobby = Arc::new(Lobby::new(LobbyConfig {
        motd: args.motd.clone(),
        max_players: args.max_players,
        read_timeout: Duration::from_secs(args.read_timeout),
        session: SessionConfig {
            compression: (args.compression_threshold >= 0)
                .then_some(Compression { threshold: args.compression_threshold as usize }),
            brand: "ferrite".into(),
            ..SessionConfig::default()
        },
    }));
    tracing::info!(
        "Ferrite for Minecraft {} (protocol {}) listening on {} with {:?} backend, {threads} threads",
        ferrite_data::GAME_VERSION,
        ferrite_data::PROTOCOL_VERSION,
        args.addr,
        args.backend,
    );

    if args.stats_interval > 0 {
        let lobby = lobby.clone();
        let every = Duration::from_secs(args.stats_interval);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(every);
                tracing::info!("online {} | {:?}", lobby.online(), lobby.stats.snapshot());
            }
        });
    }

    match args.backend {
        Backend::Tokio => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(threads)
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind(args.addr).await?;
                ferrite_net::tokio_backend::serve(listener, lobby, async {
                    let _ = tokio::signal::ctrl_c().await;
                    tracing::info!("shutting down");
                })
                .await
            })
        }
        #[cfg(all(feature = "uring", target_os = "linux"))]
        Backend::Uring => ferrite_net::uring_backend::serve(args.addr, threads, lobby)?.wait(),
        #[cfg(not(all(feature = "uring", target_os = "linux")))]
        Backend::Uring => {
            Err(std::io::Error::other("the io_uring backend needs Linux and the `uring` feature"))
        }
    }
}
