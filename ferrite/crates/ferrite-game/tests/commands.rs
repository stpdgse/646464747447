//! Commands against the official server's answers (`scripts/record-commands.py`).
//!
//! Each scenario starts a fresh server with an operator who joins in survival mode on peaceful
//! difficulty (what the recording started from), sends the recorded commands and compares what
//! comes back: the system messages and a few watched packets.

use std::sync::Arc;
use std::time::Duration;

use ferrite_game::{GameConfig, GameThread};
use ferrite_net::{Lobby, LobbyConfig, tokio_backend};
use ferrite_protocol::packets::play::{ChatCommand, SystemChat};
use ferrite_swarm::PlayBot;
use tokio::net::TcpListener;

const WATCH: [&str; 9] = [
    "ticking_state",
    "ticking_step",
    "set_time",
    "player_abilities",
    "game_event",
    "player_info_update",
    "change_difficulty",
    "set_experience",
    "set_health",
];

struct Server {
    addr: std::net::SocketAddr,
    _game: GameThread,
}

async fn start() -> Server {
    let cfg = GameConfig {
        game_mode: 0,
        difficulty: 0,
        max_players: 4,
        view_distance: 2,
        simulation_distance: 2,
        animal_chance: 0.0,
        ops: vec![("InteractBot".into(), 4)],
        ..GameConfig::default()
    };
    let (host, game) = ferrite_game::start(cfg);
    let lobby = Arc::new(Lobby::new(LobbyConfig { play: Some(host), ..LobbyConfig::default() }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(tokio_backend::serve(listener, lobby, std::future::pending::<()>()));
    Server { addr, _game: game }
}

/// What the bot receives for a while after something was sent, as recording-format lines.
async fn collect(bot: &mut PlayBot) -> Vec<String> {
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if tokio::time::Instant::now() > deadline {
            return out;
        }
        match tokio::time::timeout(Duration::from_millis(350), bot.next()).await {
            Ok(Ok(p)) => {
                if p.name == "system_chat" {
                    let c: SystemChat = ferrite_protocol::codec::decode_exact(&p.body).unwrap();
                    out.push(format!(
                        "sys {} {}",
                        ferrite_nbt::snbt::to_sorted_snbt(&c.content.0),
                        u8::from(c.overlay)
                    ));
                } else if WATCH.contains(&p.name) {
                    let hex: String = p.body.iter().map(|b| format!("{b:02x}")).collect();
                    // (The list-order update after joining is not about the command.)
                    if p.name == "player_info_update" && hex.starts_with("10") {
                        continue;
                    }
                    // (Nor is the time sync every 20 ticks, which carries no clock update.)
                    if p.name == "set_time" && hex.len() == 18 && hex.ends_with("00") {
                        continue;
                    }
                    // (The game time at the front of a time packet depends on the world's age.)
                    let hex = if p.name == "set_time" { hex[16..].to_string() } else { hex };
                    out.push(format!("evt {} {hex}", p.name));
                }
            }
            _ => return out,
        }
    }
}

struct Scenario {
    name: String,
    /// `(command, expected output lines)`.
    steps: Vec<(String, Vec<String>)>,
    setup: Vec<String>,
}

fn scenarios() -> Vec<Scenario> {
    let text = include_str!("data/commands.txt");
    let mut out: Vec<Scenario> = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix("scenario ") {
            out.push(Scenario { name: name.to_string(), steps: Vec::new(), setup: Vec::new() });
        } else if let Some(rcon) = line.strip_prefix("rcon ") {
            out.last_mut().unwrap().setup.push(rcon.to_string());
        } else if let Some(cmd) = line.strip_prefix("> ") {
            out.last_mut().unwrap().steps.push((cmd.to_string(), Vec::new()));
        } else {
            out.last_mut().unwrap().steps.last_mut().unwrap().1.push(line.to_string());
        }
    }
    out
}

#[tokio::test]
async fn commands_answer_like_the_official_server() {
    let mut problems = Vec::new();
    let mut checked = 0;
    for sc in scenarios() {
        let server = start().await;
        let mut bot = PlayBot::join(server.addr, "InteractBot").await.unwrap();
        // Let the world arrive, then run the scenario's setup commands and discard their output.
        loop {
            let p = bot.next().await.unwrap();
            if p.name == "chunk_batch_finished" {
                break;
            }
        }
        collect(&mut bot).await;
        for cmd in &sc.setup {
            bot.send(&ChatCommand { command: ferrite_protocol::types::Str(cmd.clone()) })
                .await
                .unwrap();
            collect(&mut bot).await;
        }
        for (cmd, want) in &sc.steps {
            bot.send(&ChatCommand { command: ferrite_protocol::types::Str(cmd.clone()) })
                .await
                .unwrap();
            let got = collect(&mut bot).await;
            checked += 1;
            if &got != want {
                problems.push(format!(
                    "[{}] /{cmd}\n  vanilla:\n    {}\n  ferrite:\n    {}",
                    sc.name,
                    want.join("\n    "),
                    got.join("\n    ")
                ));
            }
        }
    }
    assert!(checked > 20, "{checked} commands");
    assert!(
        problems.is_empty(),
        "{} of {checked} commands differ:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
