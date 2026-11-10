//! A scriptable client for driving a server: it joins, then reads commands from standard input
//! and reports what the server answers on standard output. The physics recorder
//! (`scripts/record-physics.py`) uses it to right-click levers and buttons on the official server,
//! which no server command can do.
//!
//! ```text
//! ferrite-interact 127.0.0.1:25565 [--name Bot]
//!   stdin:  use <x> <y> <z> <face> <sequence>     right-click the block at x y z
//!           quit
//!   stdout: ready                                  joined and the first chunks have arrived
//!           ack <sequence>                         the server acknowledged that click
//! ```

use std::net::ToSocketAddrs;
use std::time::Duration;

use clap::Parser;
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::play::{BlockChangedAck, PackedPos, UseItemOn};
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
                    ["quit"] => break,
                    _ => eprintln!("unknown command: {line}"),
                }
            }
            packet = tokio::time::timeout(Duration::from_millis(20), bot.next()) => {
                // Keep answering keep-alives and report acknowledgements.
                if let Ok(p) = packet {
                    let p = p?;
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
