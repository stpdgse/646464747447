//! Wires the Query and RCON services to the running server.

use std::net::SocketAddr;
use std::sync::Arc;

use ferrite_net::Lobby;
use ferrite_remote::query::{self, ServerInfo};
use ferrite_remote::rcon::{self, CommandRunner, RconConfig};
use tokio::net::{TcpListener, UdpSocket};

/// What Query tells the world about this server.
pub struct Info {
    pub lobby: Arc<Lobby>,
    pub bind: SocketAddr,
}

impl ServerInfo for Info {
    fn motd(&self) -> String {
        self.lobby.config().motd.clone()
    }
    fn version(&self) -> String {
        ferrite_data::GAME_VERSION.to_owned()
    }
    fn map(&self) -> String {
        "world".to_owned()
    }
    fn online(&self) -> u32 {
        self.lobby.online()
    }
    fn max_players(&self) -> u32 {
        self.lobby.config().max_players
    }
    fn players(&self) -> Vec<String> {
        // Player names are not tracked yet: gameplay does not exist, so nobody stays connected.
        Vec::new()
    }
    fn host_port(&self) -> u16 {
        self.bind.port()
    }
    fn host_ip(&self) -> String {
        self.bind.ip().to_string()
    }
}

/// The console commands that exist so far.
pub struct Console {
    pub lobby: Arc<Lobby>,
}

impl CommandRunner for Console {
    fn run(&self, command: &str) -> String {
        let command = command.trim();
        match command.split_whitespace().next() {
            // Text copied from the vanilla server (see scripts/oracle.sh).
            Some("list") => format!(
                "There are {} of a max of {} players online: ",
                self.lobby.online(),
                self.lobby.config().max_players
            ),
            _ => format!("Unknown or incomplete command. See below for error{command}<--[HERE]"),
        }
    }
}

pub struct RemoteConfig {
    pub query_port: Option<u16>,
    pub rcon: Option<(SocketAddr, RconConfig)>,
}

/// Runs Query and RCON on a dedicated single-threaded runtime in the background, so it works the
/// same whichever connection backend is in use.
pub fn spawn(cfg: RemoteConfig, lobby: Arc<Lobby>, bind: SocketAddr) -> std::io::Result<()> {
    if cfg.query_port.is_none() && cfg.rcon.is_none() {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    // Bind before spawning so a port conflict is reported at startup, not lost in a thread.
    let (query_socket, rcon_listener) = runtime.block_on(async {
        let q = match cfg.query_port {
            Some(port) => Some(UdpSocket::bind(SocketAddr::new(bind.ip(), port)).await?),
            None => None,
        };
        let r = match &cfg.rcon {
            Some((addr, _)) => Some(TcpListener::bind(addr).await?),
            None => None,
        };
        Ok::<_, std::io::Error>((q, r))
    })?;
    if let Some(q) = &query_socket {
        tracing::info!("query listening on udp {}", q.local_addr()?);
    }
    if let Some(r) = &rcon_listener {
        tracing::warn!(
            "rcon listening on {} (unencrypted: keep it on a trusted network)",
            r.local_addr()?
        );
    }

    std::thread::Builder::new().name("ferrite-remote".into()).spawn(move || {
        runtime.block_on(async move {
            let (query_lobby, rcon_lobby) = (lobby.clone(), lobby);
            let query = async move {
                if let Some(socket) = query_socket {
                    let info = Arc::new(Info { lobby: query_lobby, bind });
                    if let Err(e) = query::serve(socket, info).await {
                        tracing::error!("query stopped: {e}");
                    }
                } else {
                    std::future::pending::<()>().await;
                }
            };
            let rcon = async move {
                if let (Some(listener), Some((_, rcon_cfg))) = (rcon_listener, cfg.rcon) {
                    let runner = Arc::new(Console { lobby: rcon_lobby });
                    if let Err(e) = rcon::serve(listener, Arc::new(rcon_cfg), runner).await {
                        tracing::error!("rcon stopped: {e}");
                    }
                } else {
                    std::future::pending::<()>().await;
                }
            };
            tokio::join!(query, rcon);
        });
    })?;
    Ok(())
}
