//! A scriptable client for driving a server: it joins, then reads commands from standard input
//! and reports what the server answers on standard output. The physics recorder
//! (`scripts/record-physics.py`) uses it to right-click levers and buttons on the official server,
//! which no server command can do.
//!
//! ```text
//! ferrite-interact 127.0.0.1:25565 [--name Bot]
//!   stdin:  use <x> <y> <z> <face> <sequence>     right-click the block at x y z
//!           place <item> <x> <y> <z> <face> <cx> <cy> <cz> <sequence>
//!                                                  put <item> (e.g. minecraft:stone) in the first
//!                                                  hotbar slot (creative) and right-click x y z at
//!                                                  the given face and cursor position
//!           dig <start|stop|abort> <x> <y> <z> <face> <sequence>
//!                                                  send a digging action (survival mining)
//!           ground                                 report standing on the ground (the server
//!                                                  counts a client that never moved as airborne)
//!           quit
//!   stdout: ready                                  joined and the first chunks have arrived
//!           ack <sequence>                         the server acknowledged that click
//!           tp <n>                                 the n-th teleport was confirmed (the server
//!                                                  ignores clicks until the previous one is)
//! ```

use std::net::ToSocketAddrs;
use std::time::Duration;

use clap::Parser;
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::play::{
    BlockChangedAck, ItemStack, MovePlayerStatusOnly, PackedPos, PlayerAction, SetCreativeModeSlot,
    UseItemOn,
};
use ferrite_swarm::PlayBot;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    addr: String,
    #[arg(long, default_value = "InteractBot")]
    name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let addr = args.addr.to_socket_addrs()?.next().ok_or_else(|| anyhow::anyhow!("no address"))?;
    let mut bot = PlayBot::join(addr, &args.name).await?;
    // Wait until the world has started arriving.
    loop {
        let p = bot.next().await?;
        if p.name == "chunk_batch_finished" {
            break;
        }
    }
    println!("ready");
    let mut teleports = 0u32;
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let Some(line) = line? else { break };
                let parts: Vec<&str> = line.split_whitespace().collect();
                match parts.as_slice() {
                    ["use", x, y, z, face, seq] => {
                        bot.send(&UseItemOn {
                            hand: VarInt(0),
                            pos: PackedPos::new(x.parse()?, y.parse()?, z.parse()?),
                            face: VarInt(face.parse()?),
                            cursor_x: 0.5,
                            cursor_y: 0.5,
                            cursor_z: 0.5,
                            inside: false,
                            world_border_hit: false,
                            sequence: VarInt(seq.parse()?),
                        })
                        .await?;
                    }
                    ["place", item, x, y, z, face, cx, cy, cz, seq] => {
                        let id = ferrite_data::registries::item::by_name
                            .get(item.strip_prefix("minecraft:").map_or(*item, |s| s))
                            .or_else(|| ferrite_data::registries::item::by_name.get(*item))
                            .copied()
                            .ok_or_else(|| anyhow::anyhow!("unknown item {item}"))?;
                        bot.send(&SetCreativeModeSlot {
                            slot: 36,
                            item: ItemStack { count: 1, item: id as i32 },
                        })
                        .await?;
                        bot.send(&UseItemOn {
                            hand: VarInt(0),
                            pos: PackedPos::new(x.parse()?, y.parse()?, z.parse()?),
                            face: VarInt(face.parse()?),
                            cursor_x: cx.parse()?,
                            cursor_y: cy.parse()?,
                            cursor_z: cz.parse()?,
                            inside: false,
                            world_border_hit: false,
                            sequence: VarInt(seq.parse()?),
                        })
                        .await?;
                    }
                    ["dig", what, x, y, z, face, seq] => {
                        let action = match *what {
                            "start" => 0,
                            "abort" => 1,
                            "stop" => 2,
                            n if n.parse::<i32>().is_ok() => n.parse()?,
                            other => anyhow::bail!("unknown dig action {other}"),
                        };
                        bot.send(&PlayerAction {
                            action: VarInt(action),
                            pos: PackedPos::new(x.parse()?, y.parse()?, z.parse()?),
                            face: face.parse()?,
                            sequence: VarInt(seq.parse()?),
                        })
                        .await?;
                    }
                    ["ground"] => {
                        bot.send(&MovePlayerStatusOnly { flags: 1 }).await?;
                    }
                    ["quit"] => break,
                    _ => eprintln!("unknown command: {line}"),
                }
            }
            packet = tokio::time::timeout(Duration::from_millis(20), bot.next()) => {
                // Keep answering keep-alives and report acknowledgements.
                if let Ok(p) = packet {
                    let p = p?;
                    if std::env::var_os("FERRITE_INTERACT_TRACE").is_some() {
                        eprintln!("pkt {} {:?}", p.name, p.body);
                    }
                    if p.name == "player_position" {
                        teleports += 1;
                        println!("tp {teleports}");
                    }
                    if p.name == "block_changed_ack" {
                        let ack: BlockChangedAck = ferrite_protocol::codec::decode_exact(&p.body)?;
                        println!("ack {}", ack.sequence.0);
                    }
                }
            }
        }
    }
    Ok(())
}
