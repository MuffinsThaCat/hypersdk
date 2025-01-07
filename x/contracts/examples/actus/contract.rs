use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    public, state_schema, Address, Context, ContractId, 
    ExternalCallArgs, ExternalCallContext, Gas, Units,
};

use crate::core::{
    ContractState, ContractTerms, EventType,
    TransitionEngine, Result, Error,
};

const MAX_GAS: Gas = 10_000_000;
const ZERO: u64 = 0;

// Define core state storage schema WITHOUT schedules
state_schema! {
    // Contract Configuration
    ContractType => u8,    // Type of ACTUS contract (PAM, LAM, etc.)
    ContractRole => u8,    // Role in the contract
    Currency => Address,   // Settlement currency token address

    // Contract State
    State => Vec<u8>,      // Serialized ContractState
    Terms => Vec<u8>,      // Serialized ContractTerms

    // (Removed schedules and maturity date fields)
    // MaturityDate => u64,
    // PrincipalSchedule => Vec<u8>,
    // InterestSchedule => Vec<u8>,
    // FeeSchedule => Vec<u8>,
}

/// A helper for building external call args for token transfers
fn call_args_from_address(address: Address) -> ExternalCallArgs {
    ExternalCallArgs {
        contract_address: address,
        max_units: MAX_GAS,
        value: ZERO,
    }
}

/// Initializes the ACTUS contract (no schedule generation)
#[public]
pub fn init(
    context: &mut Context,
    contract_type: u8,
    contract_role: u8,
    currency: Address,
    terms_bytes: Vec<u8>,  // Serialized ContractTerms
) -> Result<()> {
    // 1. Deserialize and validate terms
    let contract_terms: ContractTerms = ContractTerms::try_from_slice(&terms_bytes)
        .map_err(|_| Error::ValidationError("Failed to deserialize terms".into()))?;

    // 2. Initialize empty or partially set up state
    //    If you have a custom "new" function, adapt accordingly.
    //    Otherwise, do something like:
    let mut initial_state = ContractState::new(
        context.timestamp(),
        &contract_terms,
    )?;

    // If you have removed "ContractState::new", you could do:
    // let mut initial_state = ContractState::default();
    // initial_state.status_date = context.timestamp();
    // ... do any needed field setup ...

    // 3. Store configuration and state
    //    (removed schedules => no references to generate_schedules)
    context.store((
        (ContractType, contract_type),
        (ContractRole, contract_role),
        (Currency, currency),
        (Terms, terms_bytes),
        (State, initial_state.try_to_vec()?),
        // We no longer store MaturityDate or Schedules
    ))
    .map_err(|_| Error::StorageError("Failed to set state".into()))?;

    Ok(())
}

/// Process an ACTUS event
#[public]
pub fn process_event(
    context: &mut Context,
    event_type: u8,
    timestamp: u64,
) -> Result<Option<Units>> {
    // 1. Load the current state bytes
    let state_bytes = context.get(State)
        .map_err(|_| Error::StorageError("Failed to load state".into()))?
        .ok_or_else(|| Error::StateError("State not initialized".into()))?;

    let terms_bytes = context.get(Terms)
        .map_err(|_| Error::StorageError("Failed to load terms".into()))?
        .ok_or_else(|| Error::StateError("Terms not initialized".into()))?;

    // 2. Deserialize
    let mut state: ContractState = ContractState::try_from_slice(&state_bytes)
        .map_err(|_| Error::StateError("Failed to deserialize state".into()))?;
    
    let terms: ContractTerms = ContractTerms::try_from_slice(&terms_bytes)
        .map_err(|_| Error::StateError("Failed to deserialize terms".into()))?;

    // 3. Process the event
    let result = TransitionEngine::process_event(
        event_type.into(),
        timestamp,
        &mut state,
        &terms
    )?;

    // 4. If the event triggers a payment, do a token transfer
    if let Some(amount) = result {
        process_payment(context, amount, &terms)?;
    }

    // 5. Store the updated state
    context.store((
        (State, state.try_to_vec()?),
    ))
    .map_err(|_| Error::StorageError("Failed to update state".into()))?;

    Ok(result)
}

/// Retrieve the current contract state
#[public]
pub fn get_state(context: &mut Context) -> Result<ContractState> {
    let state_bytes = context.get(State)
        .map_err(|_| Error::StorageError("Failed to load state".into()))?
        .ok_or_else(|| Error::StateError("State not initialized".into()))?;

    ContractState::try_from_slice(&state_bytes)
        .map_err(|_| Error::StateError("Failed to deserialize state".into()))
}

/// (Optional) Payment logic if your Terms define 'party'/'counterparty'
fn process_payment(
    context: &mut Context,
    amount: Units,
    terms: &ContractTerms,
) -> Result<()> {
    // 1. Load the token currency
    let currency = context.get(Currency)
        .map_err(|_| Error::StorageError("Failed to load currency".into()))?
        .ok_or_else(|| Error::StateError("Currency not set".into()))?;

    let args = call_args_from_address(currency);

    // 2. Transfer from terms.party -> terms.counterparty
    //    If you removed 'party'/'counterparty' from ContractTerms, adapt accordingly.
    token::transfer_from(
        context.to_extern(args),
        terms.party,
        terms.counterparty,
        amount,
    ).map_err(|_| Error::PaymentError("Transfer failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        // TODO: Test contract initialization
    }

    #[test]
    fn test_process_event() {
        // TODO: Test event processing
    }
}
