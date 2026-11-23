//! Any bytes presented as a save file are read or refused; a save that reads restores into a world,
//! and that world saves to bytes that read again with the same entities. Half of the inputs are raw
//! bytes; the other half are a script that builds a world, saves it (so the checksum and the schema
//! hash are right and the fuzzer reaches the entity logic) and then damages the file in a few places
//! and **re-seals it**, because a checksum that is wrong would hide every other check.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_sim::reflect::{ClassDesc, ClassRegistry, PropertyDesc, PropertyType, PropertyValue};
use ss2_sim::save::{Limits, SaveHeader, crc32, load, parse, restore, save};
use ss2_sim::{SimpleWorld, World};

fn registry() -> ClassRegistry {
    let mut r = ClassRegistry::new();
    let thing = r
        .register(
            ClassDesc::new("Thing")
                .property(PropertyDesc::new("origin", PropertyType::Vec3).persistent())
                .property(PropertyDesc::new("tag", PropertyType::Text).persistent()),
        )
        .expect("a fixed class is valid");
    r.register(
        ClassDesc::new("Mob")
            .extends(thing)
            .property(PropertyDesc::new("health", PropertyType::F32).persistent())
            .property(PropertyDesc::new("target", PropertyType::Entity).persistent())
            .property(PropertyDesc::new("alive", PropertyType::Bool).persistent())
            .property(PropertyDesc::new("kills", PropertyType::U32).persistent())
            .property(PropertyDesc::new("score", PropertyType::I32).persistent()),
    )
    .expect("a fixed class is valid");
    r
}

/// Build a world from a script: each triple of bytes is an operation.
fn scripted_world(script: &[u8]) -> SimpleWorld {
    let mut world = SimpleWorld::with_limit(registry(), 64);
    let classes = [world.registry().class_id("Thing").unwrap(), world.registry().class_id("Mob").unwrap()];
    let mut live = Vec::new();
    for op in script.chunks(3) {
        let [kind, a, b] = [op[0], *op.get(1).unwrap_or(&0), *op.get(2).unwrap_or(&0)];
        let pick = |n: u8, live: &Vec<ss2_sim::EntityId>| live.get(usize::from(n) % live.len().max(1)).copied();
        match kind % 6 {
            0 => {
                let parent = if b & 1 == 1 { pick(a, &live) } else { None };
                if let Ok(e) = world.spawn(classes[usize::from(a & 1)], parent) {
                    live.push(e);
                }
            }
            1 => {
                if let Some(e) = pick(a, &live) {
                    if world.despawn(e).is_ok() {
                        live.retain(|x| world.is_alive(*x));
                    }
                }
            }
            2 => {
                if let (Some(e), Some(p)) = (pick(a, &live), pick(b, &live)) {
                    let _ = world.set_parent(e, Some(p));
                }
            }
            3 => {
                if let Some(e) = pick(a, &live) {
                    let _ = world.set(e, "tag", PropertyValue::Text("x".repeat(usize::from(b % 20))));
                    let _ = world.set(e, "origin", PropertyValue::Vec3([f32::from(a), f32::from(b), f32::NAN]));
                }
            }
            4 => {
                if let (Some(e), Some(t)) = (pick(a, &live), pick(b, &live)) {
                    let _ = world.set(e, "target", PropertyValue::Entity(t));
                    let _ = world.set(e, "kills", PropertyValue::U32(u32::from(a) << 24 | u32::from(b)));
                }
            }
            _ => {
                if let Some(e) = pick(a, &live) {
                    let _ = world.set(e, "alive", PropertyValue::Bool(b & 1 == 1));
                    let _ = world.set(e, "score", PropertyValue::I32(i32::from(b) - 128));
                    let _ = world.set(e, "health", PropertyValue::F32(f32::from(a)));
                }
            }
        }
    }
    world
}

fn scripted(script: &[u8], edits: &[u8]) -> Vec<u8> {
    let world = scripted_world(script);
    let header = SaveHeader {
        tick: u64::from(edits.first().copied().unwrap_or(0)),
        seed: 1,
        players: 1,
        notes: vec![("k".into(), "v".into())],
    };
    let mut bytes = save(&world, &header).expect("a world of the fixed registry saves");
    let body_len = bytes.len() - 4;
    for edit in edits.chunks(3) {
        if let [a, b, c] = edit
            && body_len > 0
        {
            let position = usize::from(u16::from_le_bytes([*a, *b])) % body_len;
            bytes[position] ^= *c;
        }
    }
    // Re-seal: the checksum must be right for the damage to reach the checks behind it.
    let crc = crc32(&bytes[..body_len]);
    bytes[body_len..].copy_from_slice(&crc.to_le_bytes());
    bytes
}

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let bytes = if mode & 1 == 0 {
        rest.to_vec()
    } else {
        let (script, edits) = rest.split_at(rest.len() / 2);
        scripted(script, edits)
    };
    let limits = Limits { max_entities: 4096, max_text_bytes: 4096, max_file_bytes: 1 << 20 };
    let reg = registry();
    let Ok(file) = parse(&bytes, &reg, &limits) else {
        return;
    };

    // A save that reads restores into an empty world...
    let mut world = SimpleWorld::new(registry());
    let restored = restore(&file, &mut world).expect("a checked save restores into an empty world");
    assert_eq!(world.len(), file.entities.len());
    assert_eq!(restored.id_map.len(), file.entities.len());
    // ...and that world saves to bytes that read again with the same number of entities and the same header.
    let again = save(&world, &file.header).expect("a restored world saves");
    let parsed = parse(&again, &reg, &limits).expect("what was written reads");
    assert_eq!(parsed.entities.len(), file.entities.len());
    assert_eq!(parsed.header, file.header);
    // Loading twice into fresh worlds gives the same bytes.
    let mut second = SimpleWorld::new(registry());
    load(&bytes, &mut second, &limits).expect("a save that parsed loads");
    assert_eq!(save(&second, &file.header).expect("saves"), again);
});
