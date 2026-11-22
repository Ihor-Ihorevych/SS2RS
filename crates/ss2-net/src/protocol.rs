//! The version of the network protocol and the check two peers make before they talk
//! (roadmap T08-030).
//!
//! A [`ProtocolVersion`] is `major.minor`. The rule (also in `docs/PROTOCOL.md`):
//!
//! - a **major** change breaks compatibility; peers with different majors refuse each other;
//! - a **minor** change only adds things an older peer can ignore; peers with the same major talk
//!   at the lower of the two minors;
//! - while the major is **0** (before the first release) every minor change may break, so the
//!   minors must match exactly.
//!
//! A version number is a promise made by people; the [`Schema`] hash is a check made by the
//! machine: a hash of the message layouts that fails when someone changes a field without
//! bumping the version. [`check_handshake`] applies both.

/// FNV-1a, 64 bit: a small, dependency-free, stable hash (not a security measure).
fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

/// The protocol version a build speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion {
    /// Changes when compatibility breaks.
    pub major: u16,
    /// Changes when something backward-compatible is added.
    pub minor: u16,
}

impl ProtocolVersion {
    /// A version.
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Append `text` with a length prefix, so that ("ab", "c") differs from ("a", "bc").
fn push_text(bytes: &mut Vec<u8>, text: &str) {
    bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
    bytes.extend_from_slice(text.as_bytes());
}

/// The layouts of the messages on the wire, in a form that can be hashed. The order of messages
/// and of the fields in a message is part of the layout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schema {
    bytes: Vec<u8>,
}

impl Schema {
    /// An empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message and its fields (`(name, type)` pairs, in wire order).
    #[must_use]
    pub fn message(mut self, name: &str, fields: &[(&str, &str)]) -> Self {
        push_text(&mut self.bytes, name);
        self.bytes
            .extend_from_slice(&(fields.len() as u32).to_le_bytes());
        for (field, kind) in fields {
            push_text(&mut self.bytes, field);
            push_text(&mut self.bytes, kind);
        }
        self
    }

    /// The 64-bit hash of the layouts.
    pub fn hash(&self) -> u64 {
        fnv1a(&self.bytes)
    }
}

/// What a peer announces when it connects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handshake {
    /// The protocol version.
    pub version: ProtocolVersion,
    /// [`Schema::hash`] of the layouts it speaks.
    pub schema_hash: u64,
}

/// The outcome of [`check_handshake`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compat {
    /// The peers can talk, at this version (the lower minor of the two).
    Compatible(ProtocolVersion),
    /// The majors differ (or, before 1.0, the minors do).
    VersionMismatch {
        /// Our version.
        local: ProtocolVersion,
        /// Theirs.
        remote: ProtocolVersion,
    },
    /// The versions agree but the layouts do not: the two builds disagree about the wire format
    /// without having changed the version (a bug in the build or in the release process).
    SchemaMismatch {
        /// Our hash.
        local: u64,
        /// Theirs.
        remote: u64,
    },
}

impl Compat {
    /// A sentence to show the player or the log when the peers cannot talk; `None` when they can.
    pub fn explanation(&self) -> Option<String> {
        match self {
            Self::Compatible(_) => None,
            Self::VersionMismatch { local, remote } => Some(if remote > local {
                format!("the other side is newer (protocol {remote}, this build speaks {local}): update this game")
            } else {
                format!("the other side is older (protocol {remote}, this build speaks {local}): they need to update")
            }),
            Self::SchemaMismatch { .. } => Some(
                "the builds report the same protocol version but disagree about the message formats: they are not from the same source".to_owned(),
            ),
        }
    }
}

/// Decide whether two peers can talk, from what each announced.
pub fn check_handshake(local: &Handshake, remote: &Handshake) -> Compat {
    let (l, r) = (local.version, remote.version);
    if l.major != r.major || (l.major == 0 && l.minor != r.minor) {
        return Compat::VersionMismatch {
            local: l,
            remote: r,
        };
    }
    let negotiated = ProtocolVersion::new(l.major, l.minor.min(r.minor));
    // Different minors speak different layouts by design; the hash can only be compared at the
    // same version.
    if l == r && local.schema_hash != remote.schema_hash {
        return Compat::SchemaMismatch {
            local: local.schema_hash,
            remote: remote.schema_hash,
        };
    }
    Compat::Compatible(negotiated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs(major: u16, minor: u16, hash: u64) -> Handshake {
        Handshake {
            version: ProtocolVersion::new(major, minor),
            schema_hash: hash,
        }
    }

    #[test]
    fn the_hash_matches_the_published_fnv1a_test_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    fn schema() -> Schema {
        Schema::new()
            .message("Input", &[("tick", "u64"), ("buttons", "u32")])
            .message("Ack", &[("tick", "u64")])
    }

    #[test]
    fn the_schema_hash_is_stable_and_sensitive_to_every_detail() {
        assert_eq!(schema().hash(), schema().hash());
        // A pinned value: if this fails the encoding of the schema changed, which changes the hash
        // of every build, so bump the protocol version and `docs/PROTOCOL.md` with it.
        assert_eq!(schema().hash(), 0xbc9d_2f90_f2dc_082a);
        let base = schema().hash();
        let renamed_field = Schema::new()
            .message("Input", &[("tck", "u64"), ("buttons", "u32")])
            .message("Ack", &[("tick", "u64")]);
        let retyped = Schema::new()
            .message("Input", &[("tick", "u32"), ("buttons", "u32")])
            .message("Ack", &[("tick", "u64")]);
        let reordered = Schema::new()
            .message("Input", &[("buttons", "u32"), ("tick", "u64")])
            .message("Ack", &[("tick", "u64")]);
        let swapped_messages = Schema::new()
            .message("Ack", &[("tick", "u64")])
            .message("Input", &[("tick", "u64"), ("buttons", "u32")]);
        let extra_field = Schema::new()
            .message(
                "Input",
                &[("tick", "u64"), ("buttons", "u32"), ("aim", "f32")],
            )
            .message("Ack", &[("tick", "u64")]);
        for (name, other) in [
            ("renamed field", renamed_field),
            ("retyped", retyped),
            ("reordered", reordered),
            ("swapped messages", swapped_messages),
            ("extra field", extra_field),
        ] {
            assert_ne!(other.hash(), base, "{name} must change the hash");
        }
    }

    #[test]
    fn field_boundaries_are_not_ambiguous() {
        let a = Schema::new().message("M", &[("ab", "c")]);
        let b = Schema::new().message("M", &[("a", "bc")]);
        assert_ne!(a.hash(), b.hash());
    }

    #[test]
    fn the_same_version_and_schema_is_compatible() {
        assert_eq!(
            check_handshake(&hs(1, 3, 7), &hs(1, 3, 7)),
            Compat::Compatible(ProtocolVersion::new(1, 3))
        );
    }

    #[test]
    fn different_minors_of_one_major_talk_at_the_lower_minor_in_both_directions() {
        let (old, new) = (hs(1, 2, 111), hs(1, 5, 222));
        assert_eq!(
            check_handshake(&old, &new),
            Compat::Compatible(ProtocolVersion::new(1, 2))
        );
        assert_eq!(
            check_handshake(&new, &old),
            Compat::Compatible(ProtocolVersion::new(1, 2))
        );
    }

    #[test]
    fn different_majors_refuse_each_other() {
        let (a, b) = (hs(1, 9, 1), hs(2, 0, 1));
        assert_eq!(
            check_handshake(&a, &b),
            Compat::VersionMismatch {
                local: a.version,
                remote: b.version
            }
        );
        assert_eq!(
            check_handshake(&b, &a),
            Compat::VersionMismatch {
                local: b.version,
                remote: a.version
            }
        );
    }

    #[test]
    fn before_1_0_every_minor_must_match() {
        let (a, b) = (hs(0, 3, 1), hs(0, 4, 1));
        assert!(matches!(
            check_handshake(&a, &b),
            Compat::VersionMismatch { .. }
        ));
        assert_eq!(
            check_handshake(&a, &hs(0, 3, 1)),
            Compat::Compatible(ProtocolVersion::new(0, 3))
        );
    }

    #[test]
    fn the_same_version_with_another_schema_is_a_build_error_not_a_version_error() {
        let result = check_handshake(&hs(1, 3, 10), &hs(1, 3, 11));
        assert_eq!(
            result,
            Compat::SchemaMismatch {
                local: 10,
                remote: 11
            }
        );
        assert!(result.explanation().unwrap().contains("same source"));
    }

    #[test]
    fn the_schemas_of_different_minors_are_not_compared() {
        assert!(matches!(
            check_handshake(&hs(1, 3, 10), &hs(1, 4, 99)),
            Compat::Compatible(_)
        ));
    }

    #[test]
    fn the_explanations_tell_the_player_which_side_to_update() {
        let newer = check_handshake(&hs(1, 0, 0), &hs(2, 0, 0))
            .explanation()
            .unwrap();
        assert!(
            newer.contains("newer")
                && newer.contains("2.0")
                && newer.contains("1.0")
                && newer.contains("update this game"),
            "{newer}"
        );
        let older = check_handshake(&hs(2, 0, 0), &hs(1, 0, 0))
            .explanation()
            .unwrap();
        assert!(
            older.contains("older") && older.contains("they need to update"),
            "{older}"
        );
        assert!(
            check_handshake(&hs(1, 0, 0), &hs(1, 0, 0))
                .explanation()
                .is_none()
        );
    }

    #[test]
    fn versions_order_and_print_naturally() {
        assert!(ProtocolVersion::new(1, 10) > ProtocolVersion::new(1, 9));
        assert!(ProtocolVersion::new(2, 0) > ProtocolVersion::new(1, 99));
        assert_eq!(ProtocolVersion::new(1, 10).to_string(), "1.10");
    }
}
