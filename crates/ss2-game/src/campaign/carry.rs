//! **Carry-over**: what of a player's state crosses the boundary between two levels.
//!
//! **Spec**: `player-rules.md` and `help-digest.md` say that a *chapter* holds the player's **start inventory** (weapons, ammo, health, armor,
//! preferred weapon); they say nothing about what a player takes from one level into the next. **Chosen** here:
//!
//! * A player who is alive carries health, armor, weapons, ammo and items, and the next level raises each of them to the next chapter's start
//!   inventory where it is lower ([`PlayerCarry::merged_with_floor`], rule `CarryFloor`): the better of what they have and what the chapter gives.
//! * A player who died, who joined after the level began, or whose carry is missing starts with the chapter's start inventory alone
//!   (`DeadAndLateStart`).
//! * With [`CarryPolicy::Reset`] nobody carries anything and every level starts from its chapter (what the data alone describes).
//!
//! The shape of the inventory is a plain map of string key to count; this module does not know what a key means (a weapon, an ammunition type, a
//! key card), nor any cap on a count (the player entity, T09-006, owns the caps).

use std::collections::BTreeMap;

use super::BytesError;
use super::codec::{Reader, Writer};
use crate::level_progress::MAX_PLAYERS;

/// The most entries of one map (weapons, ammo or items) of one player.
pub const MAX_ENTRIES: usize = 256;
/// The longest key of a map, in bytes.
pub const MAX_KEY_LEN: usize = 64;

const MAGIC: [u8; 4] = *b"S2CO";
const VERSION: u16 = 1;

/// Why a value was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarryError {
    /// A slot at or past [`MAX_PLAYERS`].
    BadSlot {
        /// The slot.
        slot: u8,
    },
    /// A key that is empty, longer than [`MAX_KEY_LEN`] or has a control character.
    BadKey {
        /// Which map (`weapons`, `ammo`, `items`, `preferred weapon`).
        map: &'static str,
    },
    /// More than [`MAX_ENTRIES`] entries.
    TooManyEntries {
        /// Which map.
        map: &'static str,
    },
    /// An entry with a count of zero (an absent key is how "none" is written).
    ZeroCount {
        /// Which map.
        map: &'static str,
    },
    /// The preferred weapon is not among the weapons.
    PreferredNotOwned,
}

impl std::fmt::Display for CarryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSlot { slot } => write!(f, "slot {slot} is not a player slot"),
            Self::BadKey { map } => write!(
                f,
                "a key of {map} is empty, too long or has a control character"
            ),
            Self::TooManyEntries { map } => write!(f, "too many entries in {map}"),
            Self::ZeroCount { map } => write!(f, "an entry of {map} has a count of zero"),
            Self::PreferredNotOwned => f.write_str("the preferred weapon is not among the weapons"),
        }
    }
}

impl std::error::Error for CarryError {}

/// What a player carries (and what a chapter's start inventory holds): the same shape.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerCarry {
    /// Health points.
    pub health: u32,
    /// Armor points.
    pub armor: u32,
    /// The weapon in hand when the level ended, which must be one of `weapons`.
    pub preferred_weapon: Option<String>,
    /// Weapons owned, key to count (1 for a single weapon, 2 for a pair).
    pub weapons: BTreeMap<String, u32>,
    /// Ammunition, key (the ammunition type) to rounds.
    pub ammo: BTreeMap<String, u32>,
    /// Everything else (key cards, stored power-ups), key to count.
    pub items: BTreeMap<String, u32>,
}

fn valid_key(k: &str) -> bool {
    !k.is_empty() && k.len() <= MAX_KEY_LEN && !k.chars().any(char::is_control)
}

fn check_map(name: &'static str, m: &BTreeMap<String, u32>) -> Result<(), CarryError> {
    if m.len() > MAX_ENTRIES {
        return Err(CarryError::TooManyEntries { map: name });
    }
    for (k, v) in m {
        if !valid_key(k) {
            return Err(CarryError::BadKey { map: name });
        }
        if *v == 0 {
            return Err(CarryError::ZeroCount { map: name });
        }
    }
    Ok(())
}

fn raise(base: &BTreeMap<String, u32>, floor: &BTreeMap<String, u32>) -> BTreeMap<String, u32> {
    let mut out = base.clone();
    for (k, v) in floor {
        let e = out.entry(k.clone()).or_insert(0);
        *e = (*e).max(*v);
    }
    out
}

impl PlayerCarry {
    /// Check the limits.
    ///
    /// # Errors
    /// The first problem, see [`CarryError`].
    pub fn validate(&self) -> Result<(), CarryError> {
        check_map("weapons", &self.weapons)?;
        check_map("ammo", &self.ammo)?;
        check_map("items", &self.items)?;
        if let Some(w) = &self.preferred_weapon {
            if !valid_key(w) {
                return Err(CarryError::BadKey {
                    map: "preferred weapon",
                });
            }
            if !self.weapons.contains_key(w) {
                return Err(CarryError::PreferredNotOwned);
            }
        }
        Ok(())
    }

    /// **Chosen** (`CarryFloor`): this state raised to `floor` where it is lower: health and armor are the larger of the two, each key of the
    /// maps has the larger count, and the preferred weapon stays this player's when there is one, else it is the floor's.
    #[must_use]
    pub fn merged_with_floor(&self, floor: &Self) -> Self {
        Self {
            health: self.health.max(floor.health),
            armor: self.armor.max(floor.armor),
            preferred_weapon: self
                .preferred_weapon
                .clone()
                .or_else(|| floor.preferred_weapon.clone()),
            weapons: raise(&self.weapons, &floor.weapons),
            ammo: raise(&self.ammo, &floor.ammo),
            items: raise(&self.items, &floor.items),
        }
    }
}

/// What the next level does with the carry-over. **Chosen**.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CarryPolicy {
    /// Carry what the players have, raised to the chapter's start inventory.
    #[default]
    Keep,
    /// Everybody starts from the chapter's start inventory.
    Reset,
}

/// One player's place in a [`CarryOver`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    /// A living player's state.
    Carried(PlayerCarry),
    /// A player who was dead when the level was left.
    Dead,
}

/// The carry-over of a whole team, by player slot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CarryOver {
    slots: BTreeMap<u8, Slot>,
}

impl CarryOver {
    /// An empty carry-over: every player starts from the chapter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set what a slot carries; returns what it had.
    ///
    /// # Errors
    /// [`CarryError`]; nothing changes.
    pub fn set(&mut self, slot: u8, value: Slot) -> Result<Option<Slot>, CarryError> {
        if slot >= MAX_PLAYERS {
            return Err(CarryError::BadSlot { slot });
        }
        if let Slot::Carried(c) = &value {
            c.validate()?;
        }
        Ok(self.slots.insert(slot, value))
    }

    /// Forget a slot (that player starts from the chapter, like a late joiner).
    pub fn remove(&mut self, slot: u8) -> Option<Slot> {
        self.slots.remove(&slot)
    }

    /// What a slot carries.
    #[must_use]
    pub fn get(&self, slot: u8) -> Option<&Slot> {
        self.slots.get(&slot)
    }

    /// How many slots have an entry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// True when no slot has an entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The entries in slot order.
    pub fn iter(&self) -> impl Iterator<Item = (u8, &Slot)> {
        self.slots.iter().map(|(k, v)| (*k, v))
    }

    /// **Chosen**: what the player in `slot` starts the next level with, given the next chapter's start inventory. `Keep`: a carried player gets
    /// [`PlayerCarry::merged_with_floor`]; a dead player, a slot with no entry (a late joiner) and any slot under `Reset` get `start`.
    #[must_use]
    pub fn resolve(&self, slot: u8, start: &PlayerCarry, policy: CarryPolicy) -> PlayerCarry {
        match (policy, self.slots.get(&slot)) {
            (CarryPolicy::Keep, Some(Slot::Carried(c))) => c.merged_with_floor(start),
            _ => start.clone(),
        }
    }

    /// The state as bytes: `"S2CO"`, version `u16` (1), the number of slots, then per slot in ascending order its number, a tag (0 dead, 1 carried)
    /// and for a carried one health, armor, the preferred weapon, and the three maps; then the CRC-32. The same state gives the same bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new(MAGIC, VERSION);
        self.write(&mut w);
        w.finish()
    }

    pub(super) fn write(&self, w: &mut Writer) {
        w.u32(u32::try_from(self.slots.len()).unwrap_or(u32::MAX));
        for (slot, value) in &self.slots {
            w.u8(*slot);
            match value {
                Slot::Dead => w.u8(0),
                Slot::Carried(c) => {
                    w.u8(1);
                    w.u32(c.health);
                    w.u32(c.armor);
                    match &c.preferred_weapon {
                        Some(p) => {
                            w.u8(1);
                            w.str(p);
                        }
                        None => w.u8(0),
                    }
                    for m in [&c.weapons, &c.ammo, &c.items] {
                        w.u32(u32::try_from(m.len()).unwrap_or(u32::MAX));
                        for (k, v) in m {
                            w.str(k);
                            w.u32(*v);
                        }
                    }
                }
            }
        }
    }

    /// Read what [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    /// [`BytesError`] for anything damaged, truncated, over a limit, out of order or inconsistent.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BytesError> {
        let mut r = Reader::open(bytes, MAGIC, VERSION)?;
        let c = Self::read(&mut r)?;
        r.finish()?;
        Ok(c)
    }

    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, BytesError> {
        let n = r.count(usize::from(MAX_PLAYERS), 2)?;
        let mut out = Self::new();
        let mut prev: Option<u8> = None;
        for _ in 0..n {
            let slot = r.u8()?;
            if slot >= MAX_PLAYERS || prev.is_some_and(|p| slot <= p) {
                return Err(BytesError::Invalid("slots out of order or out of range"));
            }
            prev = Some(slot);
            let value = match r.u8()? {
                0 => Slot::Dead,
                1 => Slot::Carried(read_player(r)?),
                _ => return Err(BytesError::Invalid("unknown slot tag")),
            };
            out.slots.insert(slot, value);
        }
        Ok(out)
    }
}

fn read_player(r: &mut Reader<'_>) -> Result<PlayerCarry, BytesError> {
    let health = r.u32()?;
    let armor = r.u32()?;
    let preferred_weapon = if r.bool()? {
        Some(r.string(MAX_KEY_LEN)?)
    } else {
        None
    };
    let weapons = read_map(r)?;
    let ammo = read_map(r)?;
    let items = read_map(r)?;
    let c = PlayerCarry {
        health,
        armor,
        preferred_weapon,
        weapons,
        ammo,
        items,
    };
    c.validate()
        .map_err(|_| BytesError::Invalid("a player's state is out of range"))?;
    Ok(c)
}

fn read_map(r: &mut Reader<'_>) -> Result<BTreeMap<String, u32>, BytesError> {
    let n = r.count(MAX_ENTRIES, 6)?;
    let mut m = BTreeMap::new();
    let mut prev: Option<String> = None;
    for _ in 0..n {
        let k = r.string(MAX_KEY_LEN)?;
        let v = r.u32()?;
        if prev.as_ref().is_some_and(|p| *p >= k) {
            return Err(BytesError::Invalid("keys out of order or repeated"));
        }
        prev = Some(k.clone());
        m.insert(k, v);
    }
    Ok(m)
}

#[cfg(test)]
#[path = "carry_tests.rs"]
mod tests;
