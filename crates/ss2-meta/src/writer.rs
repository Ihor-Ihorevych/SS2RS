//! Writing CTSEMETA files back (roadmap T02-008).
//!
//! [`write()`] is the inverse of [`Container::parse`] plus [`Container::decode_objects`]: writing a
//! file that was just read gives the **same bytes** (checked on every container of the retail
//! install). It also writes edited or newly built data, for fixtures, tools and a cooked cache.

use ss2_core::{Error, Result};

use crate::container::{Container, TypeShape};
use crate::decode::Decoded;

const WHAT: &str = "CTSEMETA file to write";
const NULL_ID: u32 = 0xFFFF_FFFF;

struct Out(Vec<u8>);

impl Out {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn count(&mut self, n: usize) -> Result<()> {
        let n = u32::try_from(n)
            .map_err(|_| Error::invalid(WHAT, "a table has more than 4 billion entries"))?;
        self.u32(n);
        Ok(())
    }

    fn tag(&mut self, tag: &[u8; 4]) {
        self.0.extend_from_slice(tag);
    }

    /// A Latin-1, length-prefixed string.
    fn string(&mut self, s: &str) -> Result<()> {
        let mut bytes = Vec::with_capacity(s.len());
        for c in s.chars() {
            let b = u8::try_from(u32::from(c)).map_err(|_| {
                Error::invalid(
                    WHAT,
                    format!("the character {c:?} cannot be stored (the files are Latin-1)"),
                )
            })?;
            bytes.push(b);
        }
        self.count(bytes.len())?;
        self.0.extend_from_slice(&bytes);
        Ok(())
    }
}

/// Serialize `container` (the frame: header, tables, type table) and `decoded` (the objects and
/// the extra records) into a CTSEMETA file.
///
/// What is written, in the one section order of the format: the header and `$Version` string; the
/// `INFO` words as they stand in `container.info` (four words for versions 4 to 7, five for 8:
/// call [`Container::refresh_info`] after changing a table); `RFIL`, `IDNT` and `EXTY` with
/// sequential indices; the type table with its `ADIM` blocks; `EXOB`; `OBTY` and `OBJS` from
/// `decoded.objects`; `EDTY` and the records after `EDOB` from `decoded.extra`; the footer. Object
/// values are encoded with `decoded.types`, so the types of `container` and the type system of
/// `decoded` must belong together.
///
/// # Errors
/// [`Error::Invalid`] if a value does not fit its type, a string holds a character above U+00FF,
/// a version-4 member has no member number, or a table is too large for the format.
pub fn write(container: &Container<'_>, decoded: &Decoded) -> Result<Vec<u8>> {
    let version = container.header.format_version;
    let mut o = Out(Vec::new());
    o.0.extend_from_slice(b"CTSEMETA");
    o.u32(0x1234_ABCD);
    o.u32(version);
    o.string(&container.header.version_string)?;

    let info = &container.info;
    o.tag(b"INFO");
    match (version, info.flag) {
        (8, Some(flag)) => o.u32(flag),
        (8, None) => o.u32(1),
        (_, _) => {}
    }
    for w in [
        info.file_count,
        info.ident_count,
        info.type_count,
        info.object_count,
    ] {
        o.u32(w);
    }

    o.tag(b"RFIL");
    o.count(container.files.len())?;
    for (i, f) in container.files.iter().enumerate() {
        o.u32(i as u32);
        o.u32(f.flags);
        o.string(&f.path)?;
    }

    o.tag(b"IDNT");
    o.count(container.idents.len())?;
    for (i, name) in container.idents.iter().enumerate() {
        o.u32(i as u32);
        o.string(name)?;
    }

    o.tag(b"EXTY");
    o.count(container.external_types.len())?;
    for t in &container.external_types {
        o.u32(t.id);
        o.string(&t.name)?;
    }

    o.tag(b"INTY");
    o.count(container.types.len())?;
    let mut dims = container.array_dims.iter().peekable();
    for i in 0..=container.types.len() {
        while let Some(d) = dims.next_if(|d| d.after_type == i) {
            o.tag(b"ADIM");
            o.count(d.dims.len())?;
            d.dims.iter().for_each(|&x| o.u32(x));
        }
        let Some(t) = container.types.get(i) else {
            break;
        };
        o.tag(b"DTTY");
        o.u32(t.id);
        o.string(&t.name)?;
        o.u32(t.kind);
        match &t.shape {
            TypeShape::Short { words } => words.iter().for_each(|&w| o.u32(w)),
            TypeShape::Struct {
                flags,
                parent,
                members,
            } => {
                o.u32(*flags);
                o.u32(parent.unwrap_or(NULL_ID));
                o.tag(b"STMB");
                o.count(members.len())?;
                for m in members {
                    if version == 4 {
                        let number = m.number.ok_or_else(|| {
                            Error::invalid(
                                WHAT,
                                format!(
                                    "member `{}` of `{}` has no member number (version 4)",
                                    m.name, t.name
                                ),
                            )
                        })?;
                        o.u32(number);
                    }
                    o.string(&m.name)?;
                    o.u32(m.type_id);
                }
            }
        }
    }

    o.tag(b"EXOB");
    o.count(container.external_objects.len())?;
    for e in &container.external_objects {
        for w in [e.object_id, e.file_ref, e.flags, e.type_id] {
            o.u32(w);
        }
    }

    o.tag(b"OBTY");
    o.count(decoded.objects.len())?;
    for obj in &decoded.objects {
        o.u32(obj.id);
        o.u32(obj.type_id);
    }

    o.tag(b"EDTY");
    o.count(decoded.extra.len())?;
    for obj in &decoded.extra {
        o.u32(obj.id);
        o.u32(obj.type_id);
    }

    o.tag(b"OBJS");
    o.count(decoded.objects.len())?;
    for obj in &decoded.objects {
        o.u32(obj.id);
        o.u32(obj.type_id);
        decoded
            .types
            .encode_value(&mut o.0, obj.type_id, &obj.value)?;
    }

    o.tag(b"EDOB");
    o.count(decoded.extra.len())?;
    for obj in &decoded.extra {
        o.u32(obj.id);
        o.u32(obj.type_id);
        decoded
            .types
            .encode_value(&mut o.0, obj.type_id, &obj.value)?;
    }
    o.0.extend_from_slice(b"METAEND ");
    Ok(o.0)
}

impl Container<'_> {
    /// Set the four counts of [`Container::info`] from the tables (the flag is kept). Call it
    /// after adding or removing files, identifiers, types or objects and before [`write()`].
    pub fn refresh_info(&mut self) {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        self.info.file_count = count(self.files.len());
        self.info.ident_count = count(self.idents.len());
        self.info.type_count = count(self.external_types.len() + self.types.len());
        self.info.object_count = count(self.external_objects.len() + self.object_table.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, ObjectEntry, Value};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, SampleIds, TypeBlock};

    fn round_trip(bytes: &[u8]) -> Vec<u8> {
        let c = Container::parse(bytes).unwrap();
        let d = c.decode_objects().unwrap();
        write(&c, &d).unwrap()
    }

    #[test]
    fn the_sample_file_round_trips_byte_for_byte() {
        let bytes = MetaBuilder::sample().build().unwrap();
        assert_eq!(round_trip(&bytes), bytes);
    }

    #[test]
    fn files_with_extra_records_and_old_info_round_trip() {
        let mut b = MetaBuilder::new();
        b.info0(0);
        b.file_ref("Content/A.tex", 0x0004_0000);
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::class(1, "CEditThing", 1, 5, None, &[("1", 0)]));
        b.object(0, 1, ObjectBytes::new().i32(1).finish());
        b.edty_pair(50, 1);
        b.after_edob(1, &ObjectBytes::new().u32(50).u32(1).i32(77).finish());
        let bytes = b.build().unwrap();
        assert_eq!(round_trip(&bytes), bytes);
    }

    #[test]
    fn an_edited_value_is_written_and_read_back() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let c = Container::parse(&bytes).unwrap();
        let mut d = c.decode_objects().unwrap();
        let Value::Struct(item) = &mut d.objects[0].value else {
            panic!()
        };
        item.values[1] = Value::F32(31.5);
        let edited = write(&c, &d).unwrap();
        assert_ne!(edited, bytes);
        assert_eq!(edited.len(), bytes.len());
        let c2 = Container::parse(&edited).unwrap();
        let d2 = c2.decode_objects().unwrap();
        let item2 = d2.objects[0].value.as_struct().unwrap();
        assert_eq!(
            d2.types.member(item2, "CTestItem", "2"),
            Some(&Value::F32(31.5))
        );
    }

    #[test]
    fn a_new_object_can_be_added_with_refreshed_counts() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let mut c = Container::parse(&bytes).unwrap();
        let mut d = c.decode_objects().unwrap();
        let mut copy = d.objects[0].clone();
        copy.id = 77;
        d.objects.push(copy);
        c.object_table.push(ObjectEntry {
            object_id: 77,
            type_id: SampleIds::ITEM,
        });
        c.refresh_info();
        let out = write(&c, &d).unwrap();
        let c2 = Container::parse(&out).unwrap();
        assert!(
            c2.info_mismatches().is_empty(),
            "{:?}",
            c2.info_mismatches()
        );
        assert_eq!(c2.decode_objects().unwrap().objects.len(), 3);
    }

    #[test]
    fn values_that_do_not_fit_are_refused() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let c = Container::parse(&bytes).unwrap();
        let try_edit = |f: &dyn Fn(&mut Vec<Value>)| {
            let mut d = c.decode_objects().unwrap();
            let Value::Struct(item) = &mut d.objects[0].value else {
                panic!()
            };
            f(&mut item.values);
            write(&c, &d).unwrap_err().to_string()
        };
        assert!(try_edit(&|v| v[0] = Value::F32(1.0)).contains("expects"));
        assert!(
            try_edit(&|v| v[3] = Value::Enum {
                type_id: SampleIds::MODE,
                value: 300
            })
            .contains("fits 1 byte")
        );
        assert!(try_edit(&|v| v[4] = Value::List(vec![Value::F32(1.0)])).contains("3 elements"));
        assert!(try_edit(&|v| v.truncate(2)).contains("all its members"));
    }

    #[test]
    fn a_character_above_latin1_is_refused() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let mut c = Container::parse(&bytes).unwrap();
        let d = c.decode_objects().unwrap();
        c.files[0].path = "Content/\u{20ac}.tex".into();
        assert!(write(&c, &d).unwrap_err().to_string().contains("Latin-1"));
    }

    /// Data-gated: every container of the retail install is rewritten to the identical bytes
    /// (versions 4 to 8, worlds with extra records included). Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_file_of_the_real_install_round_trips() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut files, mut bytes_total) = (0usize, 0usize);
        let mut failures = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if vfs.stat(&path).unwrap().size < 24 {
                    continue;
                }
                let mut head = [0u8; 8];
                if std::io::Read::read_exact(&mut vfs.open(&path).unwrap(), &mut head).is_err()
                    || &head != b"CTSEMETA"
                {
                    continue;
                }
                let original = vfs.read(&path).unwrap();
                let result = Container::parse(&original)
                    .and_then(|c| c.decode_objects().and_then(|d| write(&c, &d)));
                match result {
                    Ok(out) if out == original => {
                        files += 1;
                        bytes_total += out.len();
                    }
                    Ok(out) => {
                        let at = out.iter().zip(&original).position(|(a, b)| a != b);
                        failures.push(format!(
                            "{}: differs (first at {at:?}, {} vs {} bytes)",
                            path.as_str(),
                            out.len(),
                            original.len()
                        ));
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "{files} files round-trip byte for byte ({bytes_total} bytes); {} failures",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(5)]
        );
        assert!(files > 8000, "only {files} files");
    }
}
