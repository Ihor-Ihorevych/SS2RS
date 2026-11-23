//! ss2-sim — entity model, fixed-step simulation and replication hooks (track T08)

pub mod bus;
pub mod entity;
pub mod reflect;
pub mod schedule;
pub mod tick;
pub mod world;

pub use bus::{Audience, BusConfig, BusError, EventBus, Outbox, Replicated};
pub use entity::{ClassId, EntityId};
pub use reflect::{
    ClassDesc, ClassRegistry, PropertyDesc, PropertyType, PropertyValue, RegistryError,
    ReplicationAttr, ReplicationMode,
};
pub use schedule::{Phase, Schedule, ScheduleError, Simulation, TickContext};
pub use tick::{FrameTicks, TickConfig, TickConfigError, TickLoop};
pub use world::{Change, LifecycleEvent, SimpleWorld, World, WorldError};
