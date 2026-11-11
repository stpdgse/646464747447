//! Joins a server as a minimal client and writes every clientbound play packet to a text file:
//! one line per packet, `<id> <name> <hex body>`. Used to pin play packet layouts to what the
//! official server really sends.
//!
//!     ferrite-capture 127.0.0.1:25565 captures/join.txt [--name Bot] [--seconds 6]

use std::fmt::Write as _;
use std::net::ToSocketAddrs;
use std::time::Duration;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Server address.
    addr: String,
    /// Output file.
    out: std::path::PathBuf,
    #[arg(long, default_value = "CaptureBot")]
    name: String,
    /// How long to stay in the play state.
    #[arg(long, default_value_t = 6)]
    seconds: u64,
    /// Keep only the first N packets of each kind (0 = keep everything).
    #[arg(long, default_value_t = 0)]
    per_name: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let addr = args.addr.to_socket_addrs()?.next().ok_or_else(|| anyhow::anyhow!("no address"))?;
    let packets =
        ferrite_swarm::capture_play(addr, &args.name, Duration::from_secs(args.seconds)).await?;
    let mut text = String::from("# clientbound play packets: <id> <name> <hex body>\n");
    let mut counts = std::collections::HashMap::<&str, usize>::new();
    for p in &packets {
        let n = counts.entry(p.name).or_default();
        *n += 1;
        if args.per_name > 0 && *n > args.per_name {
            continue;
        }
        let mut hex = String::with_capacity(p.body.len() * 2);
        for b in &p.body {
            let _ = write!(hex, "{b:02x}");
        }
        let _ = writeln!(text, "{} {} {}", p.id, p.name, hex);
    }
    std::fs::write(&args.out, text)?;
    eprintln!("captured {} packets into {}", packets.len(), args.out.display());
    Ok(())
}
