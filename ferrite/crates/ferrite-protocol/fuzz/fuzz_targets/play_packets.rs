//! Decodes arbitrary bytes as every play packet a client can send, and as every clientbound play
//! packet Ferrite can read. Anything that decodes must re-encode to something that decodes and re-encodes identically.
#![no_main]

use ferrite_protocol::codec::{Decode, Encode, decode_exact};
use ferrite_protocol::packets::play::*;
use libfuzzer_sys::fuzz_target;

fn check<T>(data: &[u8])
where
    T: for<'a> Decode<'a> + Encode,
{
    // Compared as bytes, not values: floats make `PartialEq` useless for NaN.
    if let Ok(value) = decode_exact::<T>(data) {
        let mut out = Vec::new();
        if value.encode(&mut out).is_ok() {
            let again = decode_exact::<T>(&out).expect("what we encode decodes");
            let mut out2 = Vec::new();
            again.encode(&mut out2).expect("a decoded value encodes");
            assert_eq!(out, out2, "encoding is stable");
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // Serverbound: untrusted input from clients.
    check::<AcceptTeleportation>(data);
    check::<ServerboundKeepAlive>(data);
    check::<ChunkBatchReceived>(data);
    check::<MovePlayerPos>(data);
    check::<MovePlayerPosRot>(data);
    check::<MovePlayerRot>(data);
    check::<PlayerAction>(data);
    check::<UseItemOn>(data);
    check::<UseItem>(data);
    check::<SetCarriedItem>(data);
    check::<SetCreativeModeSlot>(data);
    check::<PlayerCommand>(data);
    check::<ChatCommand>(data);
    check::<Chat>(data);
    check::<PlayClientInformation>(data);
    check::<ServerboundCustomPayload>(data);
    // Clientbound: what a bot or proxy reads from a server.
    check::<Login>(data);
    check::<PlayerPosition>(data);
    check::<LevelChunkWithLight>(data);
    check::<LightUpdate>(data);
    check::<MoveEntityPos>(data);
    check::<MoveEntityPosRot>(data);
    check::<SetTime>(data);
    check::<ContainerSetContent>(data);
    check::<Commands>(data);
    // Hand-written decoders with their own state.
    let _ = decode_exact::<PlayerInfoUpdate>(data);
    // Quantised velocities do not re-encode to a fixed point, so these are decode-only.
    let _ = decode_exact::<LpVec3>(data);
    let _ = decode_exact::<AddEntity>(data);
});
