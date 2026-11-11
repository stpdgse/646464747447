use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

mod remote;

use clap::{Parser, ValueEnum};
use ferrite_net::auth::{MojangAuthenticator, MojangConfig};
use ferrite_net::{Lobby, LobbyConfig, OnlineConfig};
use ferrite_protocol::Compression;
use ferrite_protocol::crypto::ServerKey;
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
    /// Run the game: players join a flat world and can move, chat and see each other. With this
    /// off, players are disconnected as soon as configuration ends.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    game: bool,
    /// View distance in chunks.
    #[arg(long, default_value_t = 10)]
    view_distance: u8,
    /// Game mode for new players: 0 survival, 1 creative, 2 adventure, 3 spectator.
    #[arg(long, default_value_t = 1)]
    game_mode: u8,
    /// Directory to save the world, players and containers in (created if missing). Without
    /// it the world lives in memory and is lost when the server stops.
    #[arg(long)]
    world: Option<std::path::PathBuf>,
    /// Seconds of silence after which a connection is dropped.
    #[arg(long, default_value_t = 15)]
    read_timeout: u64,
    /// Authenticate players with Mojang (and encrypt the connection). Turn off only for development
    /// or behind a trusted proxy: without it anyone can log in as any name.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    online_mode: bool,
    /// Base URL of the session server used in online mode.
    #[arg(long, default_value = "https://sessionserver.mojang.com")]
    session_server: String,
    /// Send each player's IP to Mojang so it can reject proxied connections.
    #[arg(long)]
    prevent_proxy_connections: bool,
    /// Concurrent Mojang verifications.
    #[arg(long, default_value_t = 8)]
    auth_workers: usize,
    /// Seconds to wait for Mojang before telling the player the services are down.
    #[arg(long, default_value_t = 6)]
    auth_timeout: u64,
    /// Serve the Query protocol (UDP) on this port, on the same address as the game.
    #[arg(long)]
    query_port: Option<u16>,
    /// Serve RCON (TCP) on this port. Requires a password.
    #[arg(long)]
    rcon_port: Option<u16>,
    /// Address RCON listens on. Defaults to localhost because RCON is unencrypted.
    #[arg(long, default_value = "127.0.0.1")]
    rcon_bind: std::net::IpAddr,
    /// RCON password (prefer the FERRITE_RCON_PASSWORD environment variable: arguments are
    /// visible to every user of the machine).
    #[arg(long, env = "FERRITE_RCON_PASSWORD", hide_env_values = true)]
    rcon_password: Option<String>,
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

    let online = if args.online_mode {
        let started = std::time::Instant::now();
        let key = Arc::new(ServerKey::generate().expect("generating the server key"));
        tracing::info!("generated a {}-bit server key in {:?}", ServerKey::BITS, started.elapsed());
        Some(OnlineConfig {
            key,
            authenticator: Arc::new(MojangAuthenticator::new(MojangConfig {
                base_url: args.session_server.clone(),
                workers: args.auth_workers,
                timeout: Duration::from_secs(args.auth_timeout.saturating_sub(1).max(1)),
                ..MojangConfig::default()
            })),
            prevent_proxy_connections: args.prevent_proxy_connections,
            auth_timeout: Duration::from_secs(args.auth_timeout),
        })
    } else {
        tracing::warn!("online mode is OFF: players are not authenticated and can use any name");
        None
    };

    // The game owns the world; the lobby only hands it connections. The io_uring backend does not
    // run the play state yet, so the game is only attached for the tokio backend.
    let mut _game_thread = None;
    let play: Option<Arc<dyn ferrite_net::PlayHost>> = if args.game
        && matches!(args.backend, Backend::Tokio)
    {
        let (host, thread) = ferrite_game::start(ferrite_game::GameConfig {
            motd: args.motd.clone(),
            max_players: args.max_players,
            view_distance: args.view_distance.clamp(2, 32),
            simulation_distance: args.view_distance.clamp(2, 32),
            game_mode: args.game_mode.min(3),
            world_dir: args.world.clone(),
            online_mode: args.online_mode,
            ..ferrite_game::GameConfig::default()
        });
        _game_thread = Some(thread);
        Some(host)
    } else {
        if args.game {
            tracing::warn!(
                "the io_uring backend does not run the game yet; players are disconnected after joining"
            );
        }
        None
    };
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
        online,
        play,
    }));
    tracing::info!(
        "Ferrite for Minecraft {} (protocol {}) listening on {} with {:?} backend, {threads} threads",
        ferrite_data::GAME_VERSION,
        ferrite_data::PROTOCOL_VERSION,
        args.addr,
        args.backend,
    );

    let rcon = match args.rcon_port {
        Some(port) => {
            let password = args.rcon_password.clone().unwrap_or_default();
            let cfg = ferrite_remote::rcon::RconConfig::new(password)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            Some((SocketAddr::new(args.rcon_bind, port), cfg))
        }
        None => None,
    };
    remote::spawn(
        remote::RemoteConfig { query_port: args.query_port, rcon },
        lobby.clone(),
        args.addr,
    )?;

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
