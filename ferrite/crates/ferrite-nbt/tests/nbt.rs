use ferrite_nbt::*;
use proptest::prelude::*;

fn net(tag: &Tag) -> Vec<u8> {
    let mut out = Vec::new();
    write_network(tag, &mut out).unwrap();
    out
}

/// The `hello world` example from the NBT specification.
const HELLO_WORLD: &[u8] = &[
    0x0a, 0x00, 0x0b, b'h', b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd', 0x08, 0x00,
    0x04, b'n', b'a', b'm', b'e', 0x00, 0x09, b'B', b'a', b'n', b'a', b'n', b'r', b'a', b'm', b'a',
    0x00,
];

#[test]
fn spec_example_named_root() {
    let mut buf = HELLO_WORLD;
    let (name, tag) = read_named(&mut buf, Limits::default()).unwrap();
    assert!(buf.is_empty());
    assert_eq!(name, "hello world");
    assert_eq!(tag, Tag::Compound(Compound::new().with("name", "Bananrama".into())));

    let mut out = Vec::new();
    write_named(&name, &tag, &mut out).unwrap();
    assert_eq!(out, HELLO_WORLD);
}

#[test]
fn network_root_has_no_name() {
    // 0a | 08 0004 "name" 0009 "Bananrama" | 00
    let compound = Tag::Compound(Compound::new().with("name", "Bananrama".into()));
    let bytes = net(&compound);
    assert_eq!(&bytes[..4], [0x0a, 0x08, 0x00, 0x04]);
    assert_eq!(bytes.len(), HELLO_WORLD.len() - 2 - 11);
    let mut s = &bytes[..];
    assert_eq!(read_network(&mut s, Limits::default()), Ok(Some(compound)));
    assert!(s.is_empty());
}

#[test]
fn root_may_be_any_tag_and_end_means_absent() {
    // Text components travel as a bare string tag.
    assert_eq!(net(&Tag::String("hi".into())), [0x08, 0x00, 0x02, b'h', b'i']);
    let mut s: &[u8] = &[0x08, 0x00, 0x02, b'h', b'i'];
    assert_eq!(read_network(&mut s, Limits::default()), Ok(Some(Tag::String("hi".into()))));

    let mut s: &[u8] = &[0x00, 0xaa];
    assert_eq!(read_network(&mut s, Limits::default()), Ok(None));
    assert_eq!(s, [0xaa], "TAG_End consumes exactly one byte");

    assert_eq!(net(&Tag::Int(-2)), [0x03, 0xff, 0xff, 0xff, 0xfe]);
}

#[test]
fn every_tag_type_round_trips_byte_exactly() {
    let c = Compound::new()
        .with("b", Tag::Byte(-5))
        .with("s", Tag::Short(-300))
        .with("i", Tag::Int(i32::MIN))
        .with("l", Tag::Long(i64::MAX))
        .with("f", Tag::Float(1.5))
        .with("d", Tag::Double(-2.25))
        .with("ba", Tag::ByteArray(vec![-1, 0, 1]))
        .with("str", Tag::String("héllo 😀".into()))
        .with("ia", Tag::IntArray(vec![1, -2]))
        .with("la", Tag::LongArray(vec![i64::MIN]))
        .with("empty", Tag::List(List::Empty))
        .with("lb", Tag::List(List::Byte(vec![1, 2])))
        .with("ls", Tag::List(List::String(vec!["a".into(), "".into()])))
        .with(
            "lc",
            Tag::List(List::Compound(vec![
                Compound::new(),
                Compound::new().with("x", Tag::Int(1)),
            ])),
        )
        .with("ll", Tag::List(List::List(vec![List::Int(vec![1]), List::Empty])))
        .with("lba", Tag::List(List::ByteArray(vec![vec![1], vec![]])))
        .with("lia", Tag::List(List::IntArray(vec![vec![7, 8]])))
        .with("lla", Tag::List(List::LongArray(vec![vec![9]])))
        .with(
            "nested",
            Tag::Compound(Compound::new().with("deep", Tag::Compound(Compound::new()))),
        );
    let tag = Tag::Compound(c);
    let bytes = net(&tag);
    let mut s = &bytes[..];
    let back = read_network(&mut s, Limits::default()).unwrap().unwrap();
    assert!(s.is_empty());
    assert_eq!(back, tag);
    assert_eq!(net(&back), bytes, "re-encoding is stable");
}

#[test]
fn modified_utf8_matches_java() {
    // NUL is C0 80; U+1F600 is a CESU-8 surrogate pair; U+00E9 and U+20AC are ordinary.
    let mut out = Vec::new();
    write_network(&Tag::String("\0é€😀".into()), &mut out).unwrap();
    let expect: &[u8] = &[
        0x08, 0x00, 0x0c, 0xc0, 0x80, 0xc3, 0xa9, 0xe2, 0x82, 0xac, 0xed, 0xa0, 0xbd, 0xed, 0xb8,
        0x80,
    ];
    // length: 2 + 2 + 3 + 6 = 13 bytes, fix the expected prefix accordingly
    let mut expect = expect.to_vec();
    expect[2] = 13;
    assert_eq!(out, expect);
    let mut s = &out[..];
    assert_eq!(read_network(&mut s, Limits::default()), Ok(Some(Tag::String("\0é€😀".into()))));
}

#[test]
fn malformed_strings_are_rejected() {
    for bad in [
        &[0xc3][..],                   // truncated 2-byte sequence
        &[0xf0, 0x9f, 0x98, 0x80][..], // real 4-byte UTF-8 is not modified UTF-8
        &[0xc1, 0x81][..],             // overlong 'A'
        &[0xed, 0xa0, 0xbd][..],       // lone high surrogate
        &[0xed, 0xb8, 0x80][..],       // lone low surrogate
        &[0x80][..],                   // stray continuation byte
        &[0xe0, 0x80, 0x80][..],       // overlong 3-byte
    ] {
        let mut data = vec![0x08, 0x00, bad.len() as u8];
        data.extend_from_slice(bad);
        assert_eq!(
            read_network(&mut &data[..], Limits::default()),
            Err(NbtError::InvalidString),
            "{bad:02x?}"
        );
    }
}

#[test]
fn quota_is_enforced_before_allocating() {
    // An int array claiming 2^30 elements with no data must fail on the quota, not the allocator.
    let mut data = vec![0x0b];
    data.extend_from_slice(&(1i32 << 30).to_be_bytes());
    assert_eq!(
        read_network(&mut &data[..], Limits::default()),
        Err(NbtError::QuotaExceeded { quota: 2 * 1024 * 1024 })
    );
    // ... and a list of compounds claiming a huge length likewise.
    let mut data = vec![0x09, 0x0a];
    data.extend_from_slice(&i32::MAX.to_be_bytes());
    assert!(matches!(
        read_network(&mut &data[..], Limits::default()),
        Err(NbtError::QuotaExceeded { .. })
    ));

    // Exact accounting: a byte tag costs 9, a compound 48 + 64 + 2*keylen + value.
    let tag = Tag::Compound(Compound::new().with("k", Tag::Byte(1)));
    let cost = 48 + 64 + 2 + 9;
    let bytes = net(&tag);
    assert!(read_network(&mut &bytes[..], Limits::new(cost, 8)).is_ok());
    assert_eq!(
        read_network(&mut &bytes[..], Limits::new(cost - 1, 8)),
        Err(NbtError::QuotaExceeded { quota: cost - 1 })
    );
    // Strings: 36 + 2 * UTF-16 units.
    let bytes = net(&Tag::String("😀a".into()));
    assert!(read_network(&mut &bytes[..], Limits::new(36 + 6, 8)).is_ok());
    assert!(read_network(&mut &bytes[..], Limits::new(36 + 5, 8)).is_err());
}

#[test]
fn depth_limit_applies_to_lists_and_compounds() {
    let mut tag = Tag::Compound(Compound::new());
    for _ in 0..4 {
        tag = Tag::Compound(Compound::new().with("x", tag));
    }
    let bytes = net(&tag); // 5 nested compounds
    assert!(read_network(&mut &bytes[..], Limits::new(1 << 20, 5)).is_ok());
    assert_eq!(
        read_network(&mut &bytes[..], Limits::new(1 << 20, 4)),
        Err(NbtError::TooDeep { depth: 4 })
    );

    // A deeply nested list bomb does not blow the stack: the limit stops it first.
    let mut bomb = Vec::new();
    for _ in 0..100_000 {
        bomb.extend_from_slice(&[0x09, 0, 0, 0, 1]);
    }
    bomb.insert(0, 0x09);
    assert!(matches!(
        read_network(&mut &bomb[..], Limits::default()),
        Err(NbtError::TooDeep { .. })
    ));
}

#[test]
fn structural_errors() {
    let l = Limits::default();
    assert_eq!(read_network(&mut &[][..], l), Err(NbtError::Eof));
    assert_eq!(read_network(&mut &[0x0d][..], l), Err(NbtError::UnknownTag(13)));
    assert_eq!(read_network(&mut &[0x03, 0, 0][..], l), Err(NbtError::Eof));
    assert_eq!(
        read_network(&mut &[0x07, 0xff, 0xff, 0xff, 0xff][..], l),
        Err(NbtError::NegativeLength(-1))
    );
    // Non-empty list of TAG_End is invalid; an empty one (or negative length) is fine.
    assert_eq!(
        read_network(&mut &[0x09, 0x00, 0, 0, 0, 2][..], l),
        Err(NbtError::EndList { len: 2 })
    );
    assert_eq!(
        read_network(&mut &[0x09, 0x00, 0, 0, 0, 0][..], l),
        Ok(Some(Tag::List(List::Empty)))
    );
    assert_eq!(
        read_network(&mut &[0x09, 0x03, 0xff, 0xff, 0xff, 0xff][..], l),
        Ok(Some(Tag::List(List::Empty)))
    );
    // Unterminated compound.
    assert_eq!(read_network(&mut &[0x0a, 0x01, 0x00, 0x01, b'a', 0x05][..], l), Err(NbtError::Eof));
    // Named roots cannot be TAG_End.
    assert_eq!(read_named(&mut &[0x00][..], l), Err(NbtError::UnexpectedTag(0)));
}

#[test]
fn duplicate_keys_keep_the_last_value_like_vanilla() {
    let data = [0x0a, 0x01, 0x00, 0x01, b'a', 1, 0x01, 0x00, 0x01, b'a', 2, 0x00];
    let tag = read_network(&mut &data[..], Limits::default()).unwrap().unwrap();
    assert_eq!(tag.as_compound().unwrap().get("a"), Some(&Tag::Byte(2)));
    assert_eq!(tag.as_compound().unwrap().len(), 1);
}

#[test]
fn oversized_strings_cannot_be_written() {
    let mut out = Vec::new();
    let long = "a".repeat(65536);
    assert_eq!(write_network(&Tag::String(long), &mut out), Err(NbtError::StringTooLong(65536)));
    // The limit is on encoded bytes: 21846 three-byte chars = 65538 bytes.
    let euros = "€".repeat(21846);
    assert!(write_network(&Tag::String(euros), &mut Vec::new()).is_err());
    assert!(write_network(&Tag::String("a".repeat(65535)), &mut Vec::new()).is_ok());
}

fn arb_list(depth: u32) -> BoxedStrategy<List> {
    let leaf = prop_oneof![
        Just(List::Empty),
        proptest::collection::vec(any::<i8>(), 1..4).prop_map(List::Byte),
        proptest::collection::vec(any::<i16>(), 1..4).prop_map(List::Short),
        proptest::collection::vec(any::<i32>(), 1..4).prop_map(List::Int),
        proptest::collection::vec(any::<i64>(), 1..4).prop_map(List::Long),
        proptest::collection::vec(any::<u32>().prop_map(|b| f32::from_bits(b & 0x7f7f_ffff)), 1..4)
            .prop_map(List::Float),
        proptest::collection::vec("\\PC{0,8}", 1..4).prop_map(List::String),
        proptest::collection::vec(proptest::collection::vec(any::<i32>(), 0..3), 1..3)
            .prop_map(List::IntArray),
    ];
    if depth == 0 {
        leaf.boxed()
    } else {
        prop_oneof![
            leaf,
            proptest::collection::vec(arb_compound(depth - 1), 1..3).prop_map(List::Compound),
            proptest::collection::vec(arb_list(depth - 1), 1..3).prop_map(List::List),
        ]
        .boxed()
    }
}

fn arb_compound(depth: u32) -> BoxedStrategy<Compound> {
    proptest::collection::vec(("\\PC{0,6}", arb_tag(depth)), 0..5)
        .prop_map(|entries| {
            let mut c = Compound::new();
            for (k, v) in entries {
                c.insert(k, v);
            }
            c
        })
        .boxed()
}

fn arb_tag(depth: u32) -> BoxedStrategy<Tag> {
    let leaf = prop_oneof![
        any::<i8>().prop_map(Tag::Byte),
        any::<i16>().prop_map(Tag::Short),
        any::<i32>().prop_map(Tag::Int),
        any::<i64>().prop_map(Tag::Long),
        any::<u32>().prop_map(|b| Tag::Float(f32::from_bits(b & 0x7f7f_ffff))),
        any::<u64>().prop_map(|b| Tag::Double(f64::from_bits(b & 0x7fff_ffff_ffff_ffff))),
        proptest::collection::vec(any::<i8>(), 0..6).prop_map(Tag::ByteArray),
        "\\PC{0,12}".prop_map(Tag::String),
        proptest::collection::vec(any::<i32>(), 0..6).prop_map(Tag::IntArray),
        proptest::collection::vec(any::<i64>(), 0..6).prop_map(Tag::LongArray),
    ];
    if depth == 0 {
        leaf.boxed()
    } else {
        prop_oneof![
            3 => leaf,
            1 => arb_list(depth - 1).prop_map(Tag::List),
            1 => arb_compound(depth - 1).prop_map(Tag::Compound),
        ]
        .boxed()
    }
}

proptest! {
    #[test]
    fn arbitrary_tags_round_trip(tag in arb_tag(3), name in "\\PC{0,10}") {
        let bytes = net(&tag);
        let mut s = &bytes[..];
        prop_assert_eq!(read_network(&mut s, Limits::default()), Ok(Some(tag.clone())));
        prop_assert!(s.is_empty());

        let mut named = Vec::new();
        write_named(&name, &tag, &mut named).unwrap();
        let mut s = &named[..];
        prop_assert_eq!(read_named(&mut s, Limits::default()), Ok((name, tag)));
    }

    #[test]
    fn garbage_never_panics_or_over_allocates(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let _ = read_network(&mut &bytes[..], Limits::new(4096, 16));
        let _ = read_named(&mut &bytes[..], Limits::new(4096, 16));
    }

    #[test]
    fn modified_utf8_round_trips(s in "\\PC{0,40}\0?\u{1F600}?") {
        let mut out = Vec::new();
        write_network(&Tag::String(s.clone()), &mut out).unwrap();
        prop_assert_eq!(read_network(&mut &out[..], Limits::default()), Ok(Some(Tag::String(s))));
    }
}

#[test]
fn nan_payloads_survive_bit_exactly() {
    // `Tag`'s PartialEq follows IEEE (NaN != NaN), so compare the bytes: a NaN with arbitrary
    // payload bits must be preserved, since clients and the saved world depend on exact data.
    for bits in [0xffff_ffffu32, 0x7fc0_0001, 0xff80_0001, 0x7f80_0000, 0x8000_0000] {
        let bytes = [&[0x05u8][..], &bits.to_be_bytes()].concat();
        let tag = read_network(&mut &bytes[..], Limits::default()).unwrap().unwrap();
        let Tag::Float(f) = tag else { panic!() };
        assert_eq!(f.to_bits(), bits);
        assert_eq!(net(&Tag::Float(f)), bytes);
    }
    for bits in [u64::MAX, 0x7ff8_0000_0000_0001, 0xfff0_0000_0000_0001] {
        let bytes = [&[0x06u8][..], &bits.to_be_bytes()].concat();
        let tag = read_network(&mut &bytes[..], Limits::default()).unwrap().unwrap();
        let Tag::Double(d) = tag else { panic!() };
        assert_eq!(d.to_bits(), bits);
        assert_eq!(net(&Tag::Double(d)), bytes);
    }
}
