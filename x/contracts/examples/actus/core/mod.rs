// src/core/mod.rs

mod types;
mod transitions;

// If you removed all schedule logic, you can comment out or remove "mod schedule;"
// mod schedule;

// We only publicly use types and transitions now
pub use types::*;
pub use transitions::*;
// If you removed "schedule", also remove "pub use schedule::*;"

// Common error handling
#[derive(Debug)]
pub enum Error {
    ValidationError(String),
    TransitionError(String),
    MathError(String),
    // If you no longer use ScheduleError, remove it
    // ScheduleError(String),
}

// We keep the same Result type alias
pub type Result<T> = std::result::Result<T, Error>;

// If you removed the entire scheduling approach, you can remove or comment out the GenerateSchedule trait
// pub trait GenerateSchedule {
//     fn generate_schedule(&self, terms: &ContractTerms) -> Result<Vec<ShiftedDay>>;
// }

// If you still want to keep a StateTransition trait, you can keep it, or remove if unused
pub trait StateTransition {
    fn transition(
        &self,
        event: EventType,
        timestamp: u64,
        state: &mut ContractState,
        terms: &ContractTerms,
    ) -> Result<Option<Units>>;
}

// Now export only the main types needed by contract.rs
pub use types::{
    ContractState,
    ContractTerms,
    EventType,
    ContractType,
    ContractRole,
    // If you still want ShiftedDay in the code, keep it
    ShiftedDay,
};
