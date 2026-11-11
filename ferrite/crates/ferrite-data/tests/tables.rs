use ferrite_data::blocks::{BLOCKS, STATE_COUNT, block_by_name, block_of_state};
use ferrite_data::registries::{self, Registry};
use ferrite_data::{Direction, State, packet_count, packet_name};

#[test]
fn version_constants_match_the_generated_release() {
    assert_eq!(ferrite_data::GAME_VERSION, "26.3");
    assert_eq!(ferrite_data::PROTOCOL_VERSION, 777);
    assert_eq!(ferrite_data::WORLD_VERSION, 5023);
}

#[test]
fn every_state_round_trips_through_its_block() {
    let mut buf = [0u8; 16];
    let mut owned = 0usize;
    for state in 0..STATE_COUNT as u16 {
        let block = block_of_state(state).expect("every state has an owner");
        assert!(block.first_state <= state && state <= block.last_state());
        let n = block.value_indices(state, &mut buf).expect("state belongs to its block");
        assert_eq!(n, block.properties.len());
        assert_eq!(block.state_id(&buf[..n]), Some(state), "{}", block.name);
        owned += 1;
    }
    assert_eq!(owned, STATE_COUNT);
    assert_eq!(BLOCKS.iter().map(|b| usize::from(b.state_count)).sum::<usize>(), STATE_COUNT);
}

#[test]
fn block_table_is_in_registry_order() {
    for (i, block) in BLOCKS.iter().enumerate() {
        assert_eq!(usize::from(block.id), i);
        assert_eq!(registries::block::names[i], block.name);
        assert!(
            block.first_state <= block.default_state && block.default_state <= block.last_state()
        );
    }
}

#[test]
fn known_block_ids_match_vanilla() {
    assert_eq!(block_by_name("air").unwrap().default_state, 0);
    assert_eq!(block_by_name("minecraft:stone").unwrap().default_state, 1);
    assert_eq!(registries::block::STONE, 1);
    assert_eq!(registries::block::AIR, 0);
}

#[test]
fn piston_head_uses_name_sorted_property_order() {
    let head = block_by_name("piston_head").unwrap();
    let names: Vec<_> = head.properties.iter().map(|p| p.name).collect();
    assert_eq!(names, ["facing", "short", "type"]);
    // From the report: state 2386 is type=sticky, facing=north, short=true.
    let facing = head.property("facing").unwrap().0;
    let short = head.property("short").unwrap().0;
    let kind = head.property("type").unwrap().0;
    let mut idx = [0u8; 3];
    idx[facing] = 0; // north
    idx[short] = 0; // "true" is listed first
    idx[kind] = 1; // sticky
    assert_eq!(head.state_id(&idx), Some(2386));
}

#[test]
fn state_id_rejects_bad_input() {
    let button = block_by_name("acacia_button").unwrap();
    assert_eq!(button.state_id(&[0, 0]), None, "wrong arity");
    assert_eq!(button.state_id(&[9, 0, 0]), None, "index out of range");
    let mut tiny = [0u8; 1];
    assert_eq!(button.value_indices(button.first_state, &mut tiny), None, "buffer too small");
    assert_eq!(button.value_indices(button.last_state() + 1, &mut [0; 3]), None, "foreign state");
    assert!(block_of_state(STATE_COUNT as u16).is_none());
}

#[test]
fn registries_are_consistent() {
    assert_eq!(registries::ALL.len(), 95);
    let mut total = 0;
    for reg in registries::ALL {
        total += reg.entries.len();
        for (id, name) in reg.entries.iter().enumerate() {
            assert_eq!(reg.id_of(name), Some(id as u32), "{} {name}", reg.name);
            assert_eq!(reg.name_of(id as u32), Some(*name));
        }
        assert_eq!(reg.id_of("minecraft:definitely_not_real"), None);
        if let Some(default) = reg.default {
            assert!(reg.id_of(default).is_some(), "{} default {default}", reg.name);
        }
    }
    assert_eq!(total, 7053);
    let item = Registry::find("minecraft:item").unwrap();
    assert_eq!(item.entries.len(), 1658);
    assert_eq!(item.default, Some("minecraft:air"));
    assert_eq!(registries::item::ACACIA_BOAT, 983);
    assert_eq!(registries::entity_type::PIG, registries::entity_type::by_name["minecraft:pig"]);
    assert!(Registry::find("minecraft:nope").is_none());
}

#[test]
fn packet_tables_are_complete() {
    use Direction::*;
    use State::*;
    let counts = [
        (Configuration, Clientbound, 21),
        (Configuration, Serverbound, 10),
        (Handshake, Serverbound, 1),
        (Login, Clientbound, 6),
        (Login, Serverbound, 5),
        (Play, Clientbound, 144),
        (Play, Serverbound, 69),
        (Status, Clientbound, 2),
        (Status, Serverbound, 2),
    ];
    for (state, dir, n) in counts {
        assert_eq!(packet_count(state, dir), n, "{state:?}/{dir:?}");
        for id in 0..n as i32 {
            assert!(packet_name(state, dir, id).is_some());
        }
        assert!(packet_name(state, dir, n as i32).is_none());
    }
    assert_eq!(packet_count(Handshake, Clientbound), 0);
    assert_eq!(packet_name(Handshake, Serverbound, 0), Some("intention"));
    assert_eq!(packet_name(Play, Clientbound, -1), None);
    assert_eq!(ferrite_data::generated::packets::handshake::serverbound::INTENTION, 0);
}
