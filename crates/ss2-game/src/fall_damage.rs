//! Fall damage hook (T09-007 slice of the player movement integration).
//!
//! `docs/re/puppet-damage.md` section 6.3 read `OnHullTouched`, `ApplyDamageImpact` and
//! `ApplyMaterialDamage` in full and found **no fall-height or fall-speed damage formula** for
//! any puppet, player included — a negative result, not a proof of absence (that page's own open
//! questions ask a reviewer to search the whole decompiled tree again with a wider net before
//! anyone assumes "no fall damage" is correct). The two related numbers the data does have are
//! the fall height below which no damage would apply at all (`ma_fAllowedFallHeight`, 2.5 m,
//! `docs/re/player-movement-constants.md`, data, high) and an unread step-function graph
//! (`php_ghFallingHitDamage`: input 20 gives 0, input 60 gives 100,
//! `docs/re/puppet-record-members.md`) with **no confirmed reader** — neither is a formula this
//! module can reproduce.
//!
//! `ss2_physics::controller::Walker::landed_fall_height` measures how far (metres) a walker fell
//! before landing, every tick that it happens; `ss2_game::play::PlaySession::update` already
//! reads it and calls [`fall_damage`] with it. **[`fall_damage`] always returns `None` today**: no
//! formula is invented here. It is kept as a named hook, with the one known number
//! ([`NO_DAMAGE_FALL_HEIGHT`]) exposed, so that whoever resolves T01-436 (the open follow-up from
//! `puppet-damage.md`) has one call site to fill in instead of wiring a new one from scratch.

/// Fall height below which no damage would apply even once a formula exists
/// (`ma_fAllowedFallHeight`, data, high confidence; `docs/re/player-movement-constants.md`).
pub const NO_DAMAGE_FALL_HEIGHT: f32 = 2.5;

/// What a fall of `height` metres (`ss2_physics::controller::Walker::landed_fall_height`) would
/// do to the player, once a formula exists (T01-436). **Always `None`**: see the module docs —
/// `docs/re/puppet-damage.md` found no fall-height or fall-speed damage formula for any puppet in
/// this build, and this crate does not invent one.
pub fn fall_damage(height: f32) -> Option<u32> {
    let _ = height;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_formula_exists_yet_so_the_hook_never_applies_damage() {
        for h in [0.0, 1.0, NO_DAMAGE_FALL_HEIGHT, 10.0, 100.0, f32::INFINITY] {
            assert_eq!(fall_damage(h), None, "height {h}");
        }
    }
}
