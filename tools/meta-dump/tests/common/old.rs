/// A format 4 to 7 file written by hand: four `INFO` words and, in version 4, a number before
/// every member name (`docs/re/ctsemeta-leftovers.md`); one class, one object.
pub fn old_file(version: u32) -> Vec<u8> {
    fn u(out: &mut Vec<u8>, v: u32) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    fn s(out: &mut Vec<u8>, v: &str) {
        u(out, v.len() as u32);
        out.extend_from_slice(v.as_bytes());
    }
    let mut o = Vec::new();
    o.extend_from_slice(b"CTSEMETA");
    u(&mut o, 0x1234_ABCD);
    u(&mut o, version);
    s(
        &mut o,
        "$Version: Serious Sam 2; 2.005.00:24818; Development$",
    );
    o.extend_from_slice(b"INFO");
    for w in [0, 1, 2, 1] {
        u(&mut o, w);
    }
    o.extend_from_slice(b"RFIL");
    u(&mut o, 0);
    o.extend_from_slice(b"IDNT");
    u(&mut o, 1);
    u(&mut o, 0);
    s(&mut o, "");
    o.extend_from_slice(b"EXTY");
    u(&mut o, 0);
    o.extend_from_slice(b"INTY");
    u(&mut o, 2);
    o.extend_from_slice(b"DTTY");
    u(&mut o, 0);
    s(&mut o, "SLONG");
    for w in [1, 0, 4, 4] {
        u(&mut o, w);
    }
    o.extend_from_slice(b"DTTY");
    u(&mut o, 1);
    s(&mut o, "CResourceFile");
    for w in [2, 5, 0xFFFF_FFFF] {
        u(&mut o, w);
    }
    o.extend_from_slice(b"STMB");
    u(&mut o, 2);
    for (number, name) in [(1u32, "rf_iNextFreeID"), (2, "rf_iDefaultID")] {
        if version == 4 {
            u(&mut o, number);
        }
        s(&mut o, name);
        u(&mut o, 0);
    }
    o.extend_from_slice(b"EXOB");
    u(&mut o, 0);
    o.extend_from_slice(b"OBTY");
    u(&mut o, 1);
    u(&mut o, 0);
    u(&mut o, 1);
    o.extend_from_slice(b"EDTY");
    u(&mut o, 0);
    o.extend_from_slice(b"OBJS");
    u(&mut o, 1);
    u(&mut o, 0);
    u(&mut o, 1);
    u(&mut o, 7);
    u(&mut o, 9);
    o.extend_from_slice(b"EDOB");
    u(&mut o, 0);
    o.extend_from_slice(b"METAEND ");
    o
}
