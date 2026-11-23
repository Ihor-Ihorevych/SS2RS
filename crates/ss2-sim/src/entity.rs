//! Entity and class identifiers (roadmap T00-017, RFC 0009).
//!
//! An [`EntityId`] is an **index plus a generation**: when an entity is despawned its index may be
//! reused, but the generation changes, so a handle to the dead entity is recognised as dead instead
//! of silently meaning the new one. It packs into a `u64` for the wire and for replays
//! ([`EntityId::to_u64`]). A [`ClassId`] names a kind of entity in the
//! [`ClassRegistry`](crate::reflect::ClassRegistry).

/// A handle to an entity: an index into the world's slots and the generation of that slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId {
    index: u32,
    generation: u32,
}

impl EntityId {
    /// The handle that refers to no entity (the default value of an entity-reference property).
    pub const NULL: Self = Self {
        index: u32::MAX,
        generation: u32::MAX,
    };

    /// A handle from its parts.
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// The slot.
    pub const fn index(self) -> u32 {
        self.index
    }

    /// How many entities have lived in the slot before this one.
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// Whether this is [`EntityId::NULL`].
    pub const fn is_null(self) -> bool {
        self.index == u32::MAX && self.generation == u32::MAX
    }

    /// The handle as 64 bits: the generation in the high half, the index in the low half.
    pub const fn to_u64(self) -> u64 {
        ((self.generation as u64) << 32) | self.index as u64
    }

    /// The inverse of [`EntityId::to_u64`].
    pub const fn from_u64(bits: u64) -> Self {
        Self {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        }
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::NULL
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "null")
        } else {
            write!(f, "{}v{}", self.index, self.generation)
        }
    }
}

/// A kind of entity, assigned by the class registry in registration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassId(pub u16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_packs_into_64_bits_and_back() {
        for (i, g) in [
            (0, 0),
            (1, 0),
            (0, 1),
            (123_456, 789),
            (u32::MAX - 1, u32::MAX - 1),
            (u32::MAX, 0),
        ] {
            let id = EntityId::new(i, g);
            assert_eq!(EntityId::from_u64(id.to_u64()), id);
            assert_eq!((id.index(), id.generation()), (i, g));
        }
        assert_eq!(EntityId::new(5, 2).to_u64(), (2u64 << 32) | 5);
    }

    #[test]
    fn the_null_handle_is_null_and_the_default_and_nothing_else_is() {
        assert!(EntityId::NULL.is_null());
        assert_eq!(EntityId::default(), EntityId::NULL);
        assert!(!EntityId::new(0, 0).is_null());
        assert!(!EntityId::new(u32::MAX, 0).is_null());
        assert!(!EntityId::new(0, u32::MAX).is_null());
        assert_eq!(EntityId::NULL.to_string(), "null");
        assert_eq!(EntityId::new(7, 3).to_string(), "7v3");
    }

    #[test]
    fn handles_order_by_index_then_generation() {
        let mut v = vec![
            EntityId::new(2, 0),
            EntityId::new(1, 5),
            EntityId::new(1, 2),
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                EntityId::new(1, 2),
                EntityId::new(1, 5),
                EntityId::new(2, 0)
            ]
        );
    }
}
