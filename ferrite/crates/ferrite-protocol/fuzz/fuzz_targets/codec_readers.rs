//! Runs every bounded reader over arbitrary input; results are discarded, only panics matter.
#![no_main]

use ferrite_protocol::codec::*;
use ferrite_protocol::varint;
use ferrite_protocol::{BlockPos, VarInt, VarLong};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = read_str(&mut &data[..], 16);
    let _ = read_str(&mut &data[..], DEFAULT_MAX_STRING);
    let _ = read_byte_array(&mut &data[..], 1024);
    let _ = read_list::<VarInt>(&mut &data[..], 64);
    let _ = read_list::<VarLong>(&mut &data[..], 1 << 20);
    let _ = read_list::<Option<BlockPos>>(&mut &data[..], 64);
    let _ = decode_exact::<Option<uuid_like::Id>>(data);
    let _ = varint::peek_i32(data);
    let _ = varint::peek_i64(data);

    // Whatever a VarInt decodes to must re-encode to something that decodes to the same value.
    if let Ok(varint::Peek::Done(v, n)) = varint::peek_i32(data) {
        assert!(n <= data.len());
        let (buf, m) = varint::encode_i32(v);
        assert_eq!(varint::peek_i32(&buf[..m]), Ok(varint::Peek::Done(v, m)));
        assert!(m <= n || n == 5, "canonical encoding is never longer");
    }
});

mod uuid_like {
    use ferrite_protocol::Decode;

    pub struct Id(#[allow(dead_code)] pub [u8; 16]);
    impl<'a> Decode<'a> for Id {
        fn decode(buf: &mut &'a [u8]) -> ferrite_protocol::Result<Self> {
            <[u8; 16]>::decode(buf).map(Id)
        }
    }
}
