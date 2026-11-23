//! ss2-sim — entity model, fixed-step simulation and replication hooks (track T08)

pub mod bus;
pub mod schedule;
pub mod tick;

pub use bus::{Audience, BusConfig, BusError, EventBus, Outbox, Replicated};
pub use schedule::{Phase, Schedule, ScheduleError, Simulation, TickContext};
pub use tick::{FrameTicks, TickConfig, TickConfigError, TickLoop};
