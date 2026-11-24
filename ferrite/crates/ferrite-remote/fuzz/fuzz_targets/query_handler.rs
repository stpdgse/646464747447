//! Arbitrary datagram sequences from several addresses. The handler must never panic, must only
//! answer well-formed requests, and must never reply with more than a bounded amount of data.
#![no_main]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use ferrite_remote::query::{QueryHandler, ServerInfo};
use libfuzzer_sys::fuzz_target;

struct Info;
impl ServerInfo for Info {
    fn motd(&self) -> String { "m".into() }
    fn version(&self) -> String { "v".into() }
    fn map(&self) -> String { "w".into() }
    fn online(&self) -> u32 { 1 }
    fn max_players(&self) -> u32 { 2 }
    fn players(&self) -> Vec<String> { vec!["p".into()] }
    fn host_port(&self) -> u16 { 1 }
    fn host_ip(&self) -> String { "i".into() }
}

fuzz_target!(|data: &[u8]| {
    let mut handler = QueryHandler::new(Arc::new(Info));
    let now = Instant::now();
    // Each datagram is `[address byte][len byte][len bytes]`.
    let mut rest = data;
    while let [who, len, tail @ ..] = rest {
        let take = usize::from(*len).min(tail.len());
        let from = SocketAddr::from(([10, 0, 0, *who % 4], 1000));
        if let Some(reply) = handler.handle(from, &tail[..take], now) {
            assert!(tail[..take].len() >= 7, "only complete requests are answered");
            assert!(reply.len() < 1024, "replies are small");
        }
        rest = &tail[take..];
    }
});
