//! **Difficulty** (roadmap T09-031, `docs/DIFFICULTY.md`): the five levels, the table of modifiers per level, the integer arithmetic that
//! applies them, the co-op rule and what a campaign remembers about the level it was started on. Pure logic: no floating point in any
//! calculation (a modifier is a count of thousandths, [`Milli`]), no `HashMap`, no clock, no randomness, so the same inputs always give
//! the same numbers on every machine.
//!
//! Every value and rule is marked as one of two kinds ([`Provenance`]):
//!
//! * **Spec**: written down in `docs/re/player-rules.md` (read from `GlobalGameParameters.rsc`): the seven members of each of the five
//!   levels ([`DifficultyTable::PC`]) and, for reference only, the Xbox table ([`DifficultyTable::XBOX`]).
//! * **Chosen**: not in the data, so ours: what a rounding does, the co-op rule ([`CoopScaling`]), the limits a table must keep to, and
//!   when the level may be changed ([`ChangePolicy`]).
//!
//! **Nothing calls this module yet**; it has only been used by its own tests. Reading the table from `GlobalGameParameters.rsc` is
//! follow-up T09-031b (the data-gated test shows how, and checks the constants against the real file).

use crate::level_progress::MAX_PLAYERS;

/// Whether a value comes from the reverse-engineering specs or is our own choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Written down in `docs/re/` (read from the original's data).
    Spec,
    /// Not in the data: chosen here and documented in `docs/DIFFICULTY.md`.
    Chosen,
}

// ---- the levels ---------------------------------------------------------------------------------------------

/// A difficulty level. **Spec**: the names and their order (`player-rules.md`: Tourist, Easy, Normal, Hard, Serious). The derived ordering
/// is the order of difficulty: `Tourist < Easy < Normal < Hard < Serious`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Difficulty {
    /// The easiest; the only level where health recharges.
    Tourist,
    /// Easy.
    Easy,
    /// The reference level: **Chosen** as the default (the menu's first campaign choice is not in the data).
    #[default]
    Normal,
    /// Hard.
    Hard,
    /// The hardest.
    Serious,
}

impl Difficulty {
    /// All five, easiest first.
    pub const ALL: [Self; 5] = [
        Self::Tourist,
        Self::Easy,
        Self::Normal,
        Self::Hard,
        Self::Serious,
    ];

    /// The position in [`Self::ALL`]: 0 for Tourist to 4 for Serious.
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Tourist => 0,
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
            Self::Serious => 4,
        }
    }

    /// The level at `index` in [`Self::ALL`], if there is one.
    #[must_use]
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Tourist),
            1 => Some(Self::Easy),
            2 => Some(Self::Normal),
            3 => Some(Self::Hard),
            4 => Some(Self::Serious),
            _ => None,
        }
    }

    /// The stable lower-case name: the value stored in a save's `difficulty` note ([`NOTE_KEY`]) and the key of a mod's table.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tourist => "tourist",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
            Self::Serious => "serious",
        }
    }

    /// The level called `name`, ignoring ASCII case; no trimming (a name with a space around it is not a name).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|d| d.name().eq_ignore_ascii_case(name))
    }

    /// The next harder level, or `None` for Serious.
    #[must_use]
    pub const fn harder(self) -> Option<Self> {
        Self::from_index(self.index() + 1)
    }

    /// The next easier level, or `None` for Tourist.
    #[must_use]
    pub const fn easier(self) -> Option<Self> {
        match self.index() {
            0 => None,
            i => Self::from_index(i - 1),
        }
    }
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The key of the note in a save (`ss2_sim::save`'s notes) and in a replay that holds [`Difficulty::name`]. **Chosen** (the key the
/// save module's own tests already use).
pub const NOTE_KEY: &str = "difficulty";

// ---- fixed point ----------------------------------------------------------------------------------------------

/// How an inexact result is made whole. All three act on the **size** of the result, so the rule is the same for a negative number.
/// **Chosen.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    /// Toward zero.
    Down,
    /// To the nearest whole number; exactly half goes **away from zero**.
    Nearest,
    /// Away from zero.
    Up,
}

/// Multiply `v` by `num / den` exactly, in 128-bit integers, and round the result as asked (on the size of the result, so a negative
/// number rounds the same way as its positive twin). The callers keep `|v|` at most 2^64 and `num` at most 2^40 (a thousandths factor, or
/// two of them multiplied), so the product is under 2^104 and nothing overflows.
fn scale(v: i128, num: u128, den: u128, rounding: Rounding) -> i128 {
    debug_assert!(v.unsigned_abs() <= 1 << 64 && num < 1 << 40);
    let magnitude = v.unsigned_abs() * num;
    let (q, r) = (magnitude / den, magnitude % den);
    let q = match rounding {
        Rounding::Down => q,
        Rounding::Nearest => q + u128::from(r * 2 >= den),
        Rounding::Up => q + u128::from(r > 0),
    };
    // q is at most the magnitude, below 2^104: it fits.
    let q = q.cast_signed();
    if v < 0 { -q } else { q }
}

/// A non-negative result as a `u32`, saturating.
fn saturate_u32(v: i128) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// A result as an `i64`, saturating at both ends.
fn saturate_i64(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

/// A multiplier in **thousandths** (1000 is x1, 1125 is x1.125). Every spec value of the tables is a whole number of thousandths, so
/// nothing is rounded to build them; arithmetic is integer and exact apart from the rounding named at each call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Milli(u32);

/// What is wrong with a number that should become a [`Milli`] or a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueProblem {
    /// Not a number.
    NotANumber,
    /// Infinite.
    Infinite,
    /// Below zero.
    Negative,
    /// Zero where the member must be above zero.
    Zero,
    /// Above [`Milli::MAX_TABLE`].
    TooLarge,
    /// A flag that is neither 0 nor 1.
    NotAFlag,
}

impl std::fmt::Display for ValueProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotANumber => "not a number",
            Self::Infinite => "infinite",
            Self::Negative => "negative",
            Self::Zero => "zero, but it must be above zero",
            Self::TooLarge => "too large",
            Self::NotAFlag => "not 0 or 1",
        })
    }
}

impl Milli {
    /// Zero (x0).
    pub const ZERO: Self = Self(0);
    /// One (x1): changes nothing.
    pub const ONE: Self = Self(1000);
    /// The largest value a table entry may have: x100 (**Chosen**; the original's largest is x2).
    pub const MAX_TABLE: Self = Self(100_000);

    /// A multiplier of `thousandths` thousandths. Not checked against [`Self::MAX_TABLE`]: tables are checked when they are built.
    #[must_use]
    pub const fn from_thousandths(thousandths: u32) -> Self {
        Self(thousandths)
    }

    /// The number of thousandths.
    #[must_use]
    pub const fn thousandths(self) -> u32 {
        self.0
    }

    /// Turn a number read from data or a mod into thousandths, **rounding to the nearest thousandth** (half away from zero), so the
    /// `f32` 0.9 of the original's file (0.89999998...) becomes 900. This is the only place a float is used, and only to read data.
    ///
    /// # Errors
    /// NaN, infinite, negative (below zero; `-0.0` is zero) or above [`Self::MAX_TABLE`].
    pub fn from_f64(value: f64) -> Result<Self, ValueProblem> {
        if value.is_nan() {
            return Err(ValueProblem::NotANumber);
        }
        if value.is_infinite() {
            return Err(ValueProblem::Infinite);
        }
        if value < 0.0 {
            return Err(ValueProblem::Negative);
        }
        let thousandths = (value * 1000.0).round();
        if thousandths > f64::from(Self::MAX_TABLE.0) {
            return Err(ValueProblem::TooLarge);
        }
        // 0 <= thousandths <= 100_000 here, so the conversion is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self(thousandths as u32))
    }

    /// `self` times `other`, rounded to the nearest thousandth (half up), saturating at `u32::MAX` thousandths.
    #[must_use]
    pub fn times(self, other: Self) -> Self {
        let v = scale(
            i128::from(self.0),
            u128::from(other.0),
            1000,
            Rounding::Nearest,
        );
        Self(saturate_u32(v))
    }

    /// `value * self`, rounded as asked, saturating at `u32::MAX`.
    #[must_use]
    pub fn apply_u32(self, value: u32, rounding: Rounding) -> u32 {
        saturate_u32(scale(i128::from(value), u128::from(self.0), 1000, rounding))
    }

    /// `value * self`, rounded as asked, saturating at `u64::MAX`.
    #[must_use]
    pub fn apply_u64(self, value: u64, rounding: Rounding) -> u64 {
        u64::try_from(scale(i128::from(value), u128::from(self.0), 1000, rounding))
            .unwrap_or(u64::MAX)
    }

    /// `value * self`, rounded as asked, saturating at `i64::MIN` and `i64::MAX`. `value` may be a fixed-point quantity (a raw
    /// Q-format number): scaling by a multiplier does not move the point.
    #[must_use]
    pub fn apply_i64(self, value: i64, rounding: Rounding) -> i64 {
        saturate_i64(scale(i128::from(value), u128::from(self.0), 1000, rounding))
    }

    /// `value / self`, rounded as asked, saturating at `u32::MAX`. Dividing by zero gives `u32::MAX` (and 0 for a `value` of 0).
    /// This is how a *rate* multiplier turns a period: twice the think frequency is half the ticks between thoughts.
    #[must_use]
    pub fn divide_u32(self, value: u32, rounding: Rounding) -> u32 {
        if value == 0 {
            return 0;
        }
        if self.0 == 0 {
            return u32::MAX;
        }
        saturate_u32(scale(i128::from(value), 1000, u128::from(self.0), rounding))
    }
}

impl std::fmt::Display for Milli {
    /// `1`, `1.125`, `0.2`: the shortest decimal that is exact.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (whole, frac) = (self.0 / 1000, self.0 % 1000);
        if frac == 0 {
            return write!(f, "{whole}");
        }
        let digits = format!("{frac:03}");
        write!(f, "{whole}.{}", digits.trim_end_matches('0'))
    }
}

// ---- the table ----------------------------------------------------------------------------------------------

/// A member of a level's modifiers. Used to say which one is wrong, and to give each its provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// `dif_fEnemyHealthMultiplier`.
    EnemyHealth,
    /// `dif_fEnemySpeedMultiplier`.
    EnemySpeed,
    /// `dif_fThinkFrequencyMultiplier`.
    ThinkFrequency,
    /// `dif_fPlayerDamageMultiplier`.
    PlayerDamage,
    /// `dif_bPlayerHealthRecharging`.
    HealthRecharging,
    /// `dif_fInitialDelayMultiplier`.
    InitialDelay,
    /// `dif_fAutoAimingFactor`.
    AutoAim,
}

impl Field {
    /// All seven, in the order of the original's class.
    pub const ALL: [Self; 7] = [
        Self::EnemyHealth,
        Self::EnemySpeed,
        Self::ThinkFrequency,
        Self::PlayerDamage,
        Self::HealthRecharging,
        Self::InitialDelay,
        Self::AutoAim,
    ];

    /// The member of `CDifficultyParams` the value comes from.
    #[must_use]
    pub const fn member(self) -> &'static str {
        match self {
            Self::EnemyHealth => "dif_fEnemyHealthMultiplier",
            Self::EnemySpeed => "dif_fEnemySpeedMultiplier",
            Self::ThinkFrequency => "dif_fThinkFrequencyMultiplier",
            Self::PlayerDamage => "dif_fPlayerDamageMultiplier",
            Self::HealthRecharging => "dif_bPlayerHealthRecharging",
            Self::InitialDelay => "dif_fInitialDelayMultiplier",
            Self::AutoAim => "dif_fAutoAimingFactor",
        }
    }

    /// Always [`Provenance::Spec`]: the *values* of every field are in the data. (What [`Field::PlayerDamage`] *means* is an inference:
    /// see [`Self::role_is_inferred`].)
    #[must_use]
    pub const fn provenance(self) -> Provenance {
        Provenance::Spec
    }

    /// True where the spec reads the member's role from its values rather than from a name that says it: `dif_fPlayerDamageMultiplier`
    /// is 0.2 on Tourist and 2 on Serious, so it is the damage the **player takes**; and the roles of the think frequency, the initial
    /// delay and the auto-aim factor are as the spec says but their exact use in the AI is not in the data.
    #[must_use]
    pub const fn role_is_inferred(self) -> bool {
        matches!(
            self,
            Self::PlayerDamage | Self::ThinkFrequency | Self::InitialDelay | Self::AutoAim
        )
    }
}

/// Why a table, a modifier set or a setting is refused. Nothing changes after any error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DifficultyError {
    /// A modifier value is not acceptable. `level` is `Some` when it was found while building a whole table.
    BadValue {
        /// The level whose entry it was, when known.
        level: Option<Difficulty>,
        /// The member.
        field: Field,
        /// What is wrong.
        problem: ValueProblem,
    },
    /// A player count of 0.
    NoPlayers,
    /// A player count above [`MAX_PLAYERS`].
    TooManyPlayers {
        /// The count asked for.
        players: u8,
    },
    /// A co-op scaling step above [`Milli::MAX_TABLE`].
    BadScaling,
    /// The campaign's policy does not allow the change now.
    ChangeRefused(ChangeRefusal),
    /// Saved bytes that cannot be read (the text says why).
    BadBytes(&'static str),
}

impl std::fmt::Display for DifficultyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadValue {
                level: Some(level),
                field,
                problem,
            } => write!(f, "{level}: {} is {problem}", field.member()),
            Self::BadValue {
                level: None,
                field,
                problem,
            } => write!(f, "{} is {problem}", field.member()),
            Self::NoPlayers => f.write_str("a game needs at least one player"),
            Self::TooManyPlayers { players } => {
                write!(f, "{players} players is over the limit of {MAX_PLAYERS}")
            }
            Self::BadScaling => f.write_str("a co-op scaling step is over the limit"),
            Self::ChangeRefused(why) => write!(f, "the difficulty cannot be changed: {why}"),
            Self::BadBytes(why) => write!(f, "cannot read the saved difficulty: {why}"),
        }
    }
}

impl std::error::Error for DifficultyError {}

/// The seven modifiers of one level (`CDifficultyParams`). Built only by [`Modifiers::new`] and [`Modifiers::from_raw`] (which check
/// them) or taken from [`DifficultyTable::PC`], so a `Modifiers` is always valid.
///
/// **Chosen** limits: health, speed and think frequency are above zero (an enemy that cannot be killed-from-zero, move or think is a
/// broken table), the others may be zero; all are at most [`Milli::MAX_TABLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Modifiers {
    enemy_health: Milli,
    enemy_speed: Milli,
    think_frequency: Milli,
    player_damage: Milli,
    health_recharging: bool,
    initial_delay: Milli,
    auto_aim: Milli,
}

/// The numbers of one `CDifficultyParams` as read from data: floats and a flag as an integer, unchecked. See [`Modifiers::from_raw`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawModifiers {
    /// `dif_fEnemyHealthMultiplier`.
    pub enemy_health: f64,
    /// `dif_fEnemySpeedMultiplier`.
    pub enemy_speed: f64,
    /// `dif_fThinkFrequencyMultiplier`.
    pub think_frequency: f64,
    /// `dif_fPlayerDamageMultiplier`.
    pub player_damage: f64,
    /// `dif_bPlayerHealthRecharging` (0 or 1).
    pub health_recharging: i64,
    /// `dif_fInitialDelayMultiplier`.
    pub initial_delay: f64,
    /// `dif_fAutoAimingFactor`.
    pub auto_aim: f64,
}

/// The member and the problem of a value that was refused; the public errors add the level where it is known.
type Rejected = (Field, ValueProblem);

fn bad((field, problem): Rejected, level: Option<Difficulty>) -> DifficultyError {
    DifficultyError::BadValue {
        level,
        field,
        problem,
    }
}

fn checked(field: Field, value: Milli, may_be_zero: bool) -> Result<Milli, Rejected> {
    if value.0 > Milli::MAX_TABLE.0 {
        Err((field, ValueProblem::TooLarge))
    } else if value.0 == 0 && !may_be_zero {
        Err((field, ValueProblem::Zero))
    } else {
        Ok(value)
    }
}

fn read(field: Field, value: f64, may_be_zero: bool) -> Result<Milli, Rejected> {
    let m = Milli::from_f64(value).map_err(|p| (field, p))?;
    checked(field, m, may_be_zero)
}

impl Modifiers {
    /// Build from thousandths.
    ///
    /// # Errors
    /// A value above [`Milli::MAX_TABLE`], or a zero health, speed or think frequency.
    pub fn new(
        enemy_health: Milli,
        enemy_speed: Milli,
        think_frequency: Milli,
        player_damage: Milli,
        health_recharging: bool,
        initial_delay: Milli,
        auto_aim: Milli,
    ) -> Result<Self, DifficultyError> {
        Self::checked_new(
            enemy_health,
            enemy_speed,
            think_frequency,
            player_damage,
            health_recharging,
            initial_delay,
            auto_aim,
        )
        .map_err(|r| bad(r, None))
    }

    fn checked_new(
        enemy_health: Milli,
        enemy_speed: Milli,
        think_frequency: Milli,
        player_damage: Milli,
        health_recharging: bool,
        initial_delay: Milli,
        auto_aim: Milli,
    ) -> Result<Self, Rejected> {
        Ok(Self {
            enemy_health: checked(Field::EnemyHealth, enemy_health, false)?,
            enemy_speed: checked(Field::EnemySpeed, enemy_speed, false)?,
            think_frequency: checked(Field::ThinkFrequency, think_frequency, false)?,
            player_damage: checked(Field::PlayerDamage, player_damage, true)?,
            health_recharging,
            initial_delay: checked(Field::InitialDelay, initial_delay, true)?,
            auto_aim: checked(Field::AutoAim, auto_aim, true)?,
        })
    }

    /// Build from numbers read from data or a mod, rounding each to the nearest thousandth.
    ///
    /// # Errors
    /// NaN, infinite, negative, too large, a zero health, speed or think frequency, or a recharging flag other than 0 or 1.
    pub fn from_raw(raw: RawModifiers) -> Result<Self, DifficultyError> {
        Self::parse(raw).map_err(|r| bad(r, None))
    }

    fn parse(raw: RawModifiers) -> Result<Self, Rejected> {
        let health_recharging = match raw.health_recharging {
            0 => false,
            1 => true,
            _ => return Err((Field::HealthRecharging, ValueProblem::NotAFlag)),
        };
        Ok(Self {
            enemy_health: read(Field::EnemyHealth, raw.enemy_health, false)?,
            enemy_speed: read(Field::EnemySpeed, raw.enemy_speed, false)?,
            think_frequency: read(Field::ThinkFrequency, raw.think_frequency, false)?,
            player_damage: read(Field::PlayerDamage, raw.player_damage, true)?,
            health_recharging,
            initial_delay: read(Field::InitialDelay, raw.initial_delay, true)?,
            auto_aim: read(Field::AutoAim, raw.auto_aim, true)?,
        })
    }

    /// A table entry written out in the source: the values are checked by the tests of this module, not at run time.
    const fn spec(
        enemy_health: u32,
        enemy_speed: u32,
        think_frequency: u32,
        player_damage: u32,
        health_recharging: bool,
        initial_delay: u32,
        auto_aim: u32,
    ) -> Self {
        Self {
            enemy_health: Milli(enemy_health),
            enemy_speed: Milli(enemy_speed),
            think_frequency: Milli(think_frequency),
            player_damage: Milli(player_damage),
            health_recharging,
            initial_delay: Milli(initial_delay),
            auto_aim: Milli(auto_aim),
        }
    }

    /// `dif_fEnemyHealthMultiplier`.
    #[must_use]
    pub const fn enemy_health(&self) -> Milli {
        self.enemy_health
    }
    /// `dif_fEnemySpeedMultiplier`.
    #[must_use]
    pub const fn enemy_speed(&self) -> Milli {
        self.enemy_speed
    }
    /// `dif_fThinkFrequencyMultiplier`: how much more often an enemy's brain thinks.
    #[must_use]
    pub const fn think_frequency(&self) -> Milli {
        self.think_frequency
    }
    /// `dif_fPlayerDamageMultiplier`: read as the damage the **player takes** (the spec's inference).
    #[must_use]
    pub const fn player_damage(&self) -> Milli {
        self.player_damage
    }
    /// `dif_bPlayerHealthRecharging`: the player's health recharges (Tourist only in the original's table). The rate and the delay are
    /// not in the data and are not here.
    #[must_use]
    pub const fn health_recharging(&self) -> bool {
        self.health_recharging
    }
    /// `dif_fInitialDelayMultiplier`: scales the delay before an enemy reacts.
    #[must_use]
    pub const fn initial_delay(&self) -> Milli {
        self.initial_delay
    }
    /// `dif_fAutoAimingFactor`: scales the auto-aim (how it enters the original's cone is not in the data).
    #[must_use]
    pub const fn auto_aim(&self) -> Milli {
        self.auto_aim
    }

    /// An enemy's health: `base * enemy_health`, rounded to nearest, but never below 1 for a positive `base` (a living enemy does not
    /// start dead). **Chosen** rounding and floor; the multiplier is Spec.
    #[must_use]
    pub fn scale_enemy_health(&self, base: u32) -> u32 {
        scale_positive_at_least_one(self.enemy_health, base)
    }

    /// The damage the player takes from one hit: `base * player_damage`, rounded to nearest, so a hit worth less than half a point on
    /// Tourist (x0.2: a base of 2 or less) does nothing. **Chosen** rounding.
    #[must_use]
    pub fn scale_damage_to_player(&self, base: u32) -> u32 {
        self.player_damage.apply_u32(base, Rounding::Nearest)
    }

    /// An enemy's speed as a raw fixed-point number (the point does not move), rounded to nearest, saturating. **Chosen** rounding.
    #[must_use]
    pub fn scale_enemy_speed(&self, raw: i64) -> i64 {
        self.enemy_speed.apply_i64(raw, Rounding::Nearest)
    }

    /// The ticks between two thoughts of an enemy's brain: `base / think_frequency`, rounded to nearest, never below 1 tick for a
    /// positive `base`. **Chosen** rounding and floor.
    #[must_use]
    pub fn scale_think_period(&self, base_ticks: u32) -> u32 {
        let v = self
            .think_frequency
            .divide_u32(base_ticks, Rounding::Nearest);
        if base_ticks > 0 { v.max(1) } else { 0 }
    }

    /// The ticks before an enemy first reacts: `base * initial_delay`, rounded to nearest (0 stays possible: Serious halves and
    /// quarters it). **Chosen** rounding.
    #[must_use]
    pub fn scale_initial_delay(&self, base_ticks: u32) -> u32 {
        self.initial_delay.apply_u32(base_ticks, Rounding::Nearest)
    }

    /// An auto-aim quantity (an angle or a strength as a raw fixed-point number), rounded to nearest, saturating. **Chosen** rounding.
    #[must_use]
    pub fn scale_auto_aim(&self, raw: i64) -> i64 {
        self.auto_aim.apply_i64(raw, Rounding::Nearest)
    }
}

fn scale_positive_at_least_one(m: Milli, base: u32) -> u32 {
    let v = m.apply_u32(base, Rounding::Nearest);
    if base > 0 { v.max(1) } else { 0 }
}

/// The five levels' modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DifficultyTable {
    entries: [Modifiers; 5],
}

impl DifficultyTable {
    /// **Spec**: the PC table, `ggp_gdGameDifficulties` of `GlobalGameParameters.rsc` (`player-rules.md`). Player damage is the damage
    /// the player takes (inferred role). Checked against the real file by the data-gated test.
    pub const PC: Self = Self {
        entries: [
            Modifiers::spec(500, 750, 750, 200, true, 1500, 1250),
            Modifiers::spec(1000, 750, 750, 500, false, 1250, 1150),
            Modifiers::spec(1000, 1000, 1000, 1000, false, 1000, 1000),
            Modifiers::spec(1000, 1125, 1125, 1500, false, 500, 900),
            Modifiers::spec(1000, 1250, 1750, 2000, false, 250, 800),
        ],
    };

    /// **Spec**, reference only: `ggp_gdGameDifficultiesXBox`, the gentler console table. The PC game uses [`Self::PC`]; this one is
    /// here so a mod or a "console" rule set can choose it. Checked against the real file by the data-gated test.
    pub const XBOX: Self = Self {
        entries: [
            Modifiers::spec(500, 750, 750, 200, true, 1500, 1200),
            Modifiers::spec(500, 750, 750, 500, false, 1500, 1100),
            Modifiers::spec(750, 900, 750, 750, false, 1250, 1000),
            Modifiers::spec(1000, 1000, 1000, 1500, false, 1000, 900),
            Modifiers::spec(1000, 1125, 1125, 2000, false, 500, 800),
        ],
    };

    /// A table from five valid entries, easiest first. No other rule is imposed: the Xbox table is not monotone in every column
    /// either.
    #[must_use]
    pub const fn new(entries: [Modifiers; 5]) -> Self {
        Self { entries }
    }

    /// A table from numbers read from data or a mod, easiest level first. The error names the level.
    ///
    /// # Errors
    /// [`DifficultyError::BadValue`] for the first entry (in level order) and member (in [`Field::ALL`] order) that is not acceptable.
    pub fn from_raw(rows: [RawModifiers; 5]) -> Result<Self, DifficultyError> {
        let mut entries = [DifficultyTable::PC.entries[0]; 5];
        for (i, row) in rows.into_iter().enumerate() {
            entries[i] =
                Modifiers::parse(row).map_err(|r| bad(r, Difficulty::from_index(i as u8)))?;
        }
        Ok(Self { entries })
    }

    /// The modifiers of `level`.
    #[must_use]
    pub const fn get(&self, level: Difficulty) -> &Modifiers {
        &self.entries[level.index() as usize]
    }
}

impl Default for DifficultyTable {
    /// The PC table.
    fn default() -> Self {
        Self::PC
    }
}

// ---- co-op --------------------------------------------------------------------------------------------------

/// How the number of players changes the numbers (**Chosen**: the original's data has no co-op scaling, and `game-modes.md` says the
/// difficulty tables apply to single player and co-op unchanged).
///
/// The rule, a pure function: with `n` players (1 to [`MAX_PLAYERS`]) the factor is `1 + step * (n - 1)`: linear in the extra players,
/// exactly 1 for one player. There are two independent steps, one for the **health** of each enemy and one for the **number** of
/// enemies a spawner makes. The level's modifiers ([`Modifiers`]) are *not* changed by the player count: damage taken, speed, think rate,
/// delay and auto-aim stay as the level says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CoopScaling {
    health_step: Milli,
    count_step: Milli,
}

impl CoopScaling {
    /// No scaling: the same numbers for any number of players. The default, and what the original's data says.
    pub const NONE: Self = Self {
        health_step: Milli::ZERO,
        count_step: Milli::ZERO,
    };

    /// A scaling with a health step and a spawn-count step per extra player (a step of 500 is +50% per extra player).
    ///
    /// # Errors
    /// [`DifficultyError::BadScaling`] when a step is above [`Milli::MAX_TABLE`].
    pub fn new(health_step: Milli, count_step: Milli) -> Result<Self, DifficultyError> {
        if health_step > Milli::MAX_TABLE || count_step > Milli::MAX_TABLE {
            return Err(DifficultyError::BadScaling);
        }
        Ok(Self {
            health_step,
            count_step,
        })
    }

    /// The health step per extra player.
    #[must_use]
    pub const fn health_step(&self) -> Milli {
        self.health_step
    }

    /// The spawn-count step per extra player.
    #[must_use]
    pub const fn count_step(&self) -> Milli {
        self.count_step
    }

    fn factor(step: Milli, players: u8) -> Result<Milli, DifficultyError> {
        match players {
            0 => Err(DifficultyError::NoPlayers),
            p if p > MAX_PLAYERS => Err(DifficultyError::TooManyPlayers { players }),
            p => {
                // step <= 100_000 and extra <= 63: well inside u32 after adding 1000 (6_300_000 + 1000).
                let extra = u32::from(p) - 1;
                Ok(Milli(Milli::ONE.0 + step.0 * extra))
            }
        }
    }

    /// The enemy health factor for `players`.
    ///
    /// # Errors
    /// No players, or more than [`MAX_PLAYERS`].
    pub fn health_factor(&self, players: u8) -> Result<Milli, DifficultyError> {
        Self::factor(self.health_step, players)
    }

    /// The spawn-count factor for `players`.
    ///
    /// # Errors
    /// No players, or more than [`MAX_PLAYERS`].
    pub fn count_factor(&self, players: u8) -> Result<Milli, DifficultyError> {
        Self::factor(self.count_step, players)
    }
}

impl Default for CoopScaling {
    fn default() -> Self {
        Self::NONE
    }
}

/// A level's modifiers combined with the player count: what the game actually applies. Built by [`Effective::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Effective {
    modifiers: Modifiers,
    players: u8,
    health_factor: Milli,
    count_factor: Milli,
}

impl Effective {
    /// Combine `modifiers` with `coop` for `players`.
    ///
    /// # Errors
    /// No players, or more than [`MAX_PLAYERS`].
    pub fn new(
        modifiers: &Modifiers,
        coop: &CoopScaling,
        players: u8,
    ) -> Result<Self, DifficultyError> {
        Ok(Self {
            modifiers: *modifiers,
            players,
            health_factor: coop.health_factor(players)?,
            count_factor: coop.count_factor(players)?,
        })
    }

    /// The level's modifiers (unchanged by the player count).
    #[must_use]
    pub const fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    /// The number of players this was built for.
    #[must_use]
    pub const fn players(&self) -> u8 {
        self.players
    }

    /// An enemy's health: `base * level multiplier * co-op factor`, **one** rounding at the end (to nearest), never below 1 for a
    /// positive `base`, saturating at `u32::MAX`.
    #[must_use]
    pub fn enemy_health(&self, base: u32) -> u32 {
        let v = scale(
            i128::from(base),
            u128::from(self.modifiers.enemy_health.0) * u128::from(self.health_factor.0),
            1_000_000,
            Rounding::Nearest,
        );
        let v = saturate_u32(v);
        if base > 0 { v.max(1) } else { 0 }
    }

    /// How many enemies a spawner makes for a base count of `base`: `base * co-op factor`, rounded to nearest, never below 1 for a
    /// positive `base`, saturating. The level's difficulty does not change the count (the data has no such member).
    #[must_use]
    pub fn spawn_count(&self, base: u32) -> u32 {
        scale_positive_at_least_one(self.count_factor, base)
    }
}

// ---- carry-over and locking -----------------------------------------------------------------------------------

/// When the campaign's difficulty may be changed. **Chosen**: the original lets the player pick the level when starting a campaign
/// (`SMSSelectDifficulty`) and says nothing about changing it later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ChangePolicy {
    /// Never: the level chosen at the start stays for the whole campaign.
    Locked,
    /// Only between levels (not while a level is being played), up or down. The default: it lets a player who is stuck choose an
    /// easier level for the next one, and cannot be used to dodge a fight in the middle of a level.
    #[default]
    BetweenLevels,
    /// At any time.
    Anytime,
}

impl ChangePolicy {
    const fn index(self) -> u8 {
        match self {
            Self::Locked => 0,
            Self::BetweenLevels => 1,
            Self::Anytime => 2,
        }
    }

    const fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Self::Locked),
            1 => Some(Self::BetweenLevels),
            2 => Some(Self::Anytime),
            _ => None,
        }
    }
}

/// Why a change of difficulty was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeRefusal {
    /// The policy is [`ChangePolicy::Locked`].
    Locked,
    /// The policy is [`ChangePolicy::BetweenLevels`] and a level is being played.
    LevelInProgress,
}

impl std::fmt::Display for ChangeRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Locked => "it is locked for this campaign",
            Self::LevelInProgress => "a level is being played",
        })
    }
}

/// What a campaign remembers about its difficulty: the level, the policy for changing it, and whether a level is being played. The
/// difficulty **carries over** from level to level unchanged unless a permitted change is made; it is part of a save (use
/// [`Self::to_bytes`] or the note [`Self::note`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CampaignDifficulty {
    level: Difficulty,
    policy: ChangePolicy,
    in_level: bool,
}

/// The size of [`CampaignDifficulty::to_bytes`].
pub const CAMPAIGN_BYTES: usize = 4;
const BYTES_VERSION: u8 = 1;

impl CampaignDifficulty {
    /// A campaign started on `level` with `policy`, between levels.
    #[must_use]
    pub const fn new(level: Difficulty, policy: ChangePolicy) -> Self {
        Self {
            level,
            policy,
            in_level: false,
        }
    }

    /// The level.
    #[must_use]
    pub const fn level(&self) -> Difficulty {
        self.level
    }

    /// The policy.
    #[must_use]
    pub const fn policy(&self) -> ChangePolicy {
        self.policy
    }

    /// True while a level is being played.
    #[must_use]
    pub const fn in_level(&self) -> bool {
        self.in_level
    }

    /// A level starts. Doing it twice changes nothing more.
    pub fn enter_level(&mut self) {
        self.in_level = true;
    }

    /// A level ends (finished, or left for the menu). The difficulty carries over to the next one.
    pub fn leave_level(&mut self) {
        self.in_level = false;
    }

    /// Whether [`Self::request_change`] would accept a *different* level now.
    ///
    /// # Errors
    /// The refusal the policy gives now.
    pub const fn can_change(&self) -> Result<(), ChangeRefusal> {
        match self.policy {
            ChangePolicy::Locked => Err(ChangeRefusal::Locked),
            ChangePolicy::BetweenLevels if self.in_level => Err(ChangeRefusal::LevelInProgress),
            ChangePolicy::BetweenLevels | ChangePolicy::Anytime => Ok(()),
        }
    }

    /// Ask for another level. Returns `Ok(true)` when the level changed and `Ok(false)` when `level` is the one already in force (always
    /// accepted, even when locked).
    ///
    /// # Errors
    /// [`DifficultyError::ChangeRefused`] when `level` is different and the policy does not allow it now.
    pub fn request_change(&mut self, level: Difficulty) -> Result<bool, DifficultyError> {
        if level == self.level {
            return Ok(false);
        }
        self.can_change().map_err(DifficultyError::ChangeRefused)?;
        self.level = level;
        Ok(true)
    }

    /// The `(key, value)` of the save note that holds the level: [`NOTE_KEY`] and [`Difficulty::name`].
    #[must_use]
    pub const fn note(&self) -> (&'static str, &'static str) {
        (NOTE_KEY, self.level.name())
    }

    /// The level from a save note's value, with the policy and state of a campaign that was just loaded between levels.
    ///
    /// # Errors
    /// [`DifficultyError::BadBytes`] for a name that is not a level.
    pub fn from_note(value: &str, policy: ChangePolicy) -> Result<Self, DifficultyError> {
        Difficulty::from_name(value)
            .map(|level| Self::new(level, policy))
            .ok_or(DifficultyError::BadBytes("unknown difficulty name"))
    }

    /// The state as four bytes: format version, level, policy, in-level flag.
    #[must_use]
    pub const fn to_bytes(&self) -> [u8; CAMPAIGN_BYTES] {
        [
            BYTES_VERSION,
            self.level.index(),
            self.policy.index(),
            self.in_level as u8,
        ]
    }

    /// Read what [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    /// [`DifficultyError::BadBytes`] for a wrong length, version, level, policy or flag.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DifficultyError> {
        let [version, level, policy, in_level] = *bytes else {
            return Err(DifficultyError::BadBytes("wrong length"));
        };
        if version != BYTES_VERSION {
            return Err(DifficultyError::BadBytes("unknown version"));
        }
        let level = Difficulty::from_index(level)
            .ok_or(DifficultyError::BadBytes("unknown difficulty level"))?;
        let policy = ChangePolicy::from_index(policy)
            .ok_or(DifficultyError::BadBytes("unknown change policy"))?;
        let in_level = match in_level {
            0 => false,
            1 => true,
            _ => return Err(DifficultyError::BadBytes("bad in-level flag")),
        };
        Ok(Self {
            level,
            policy,
            in_level,
        })
    }
}

#[cfg(test)]
#[path = "difficulty_tests.rs"]
mod tests;
