// x/contracts/examples/actus/tests/integration.rs

use std::{path::PathBuf, process::Command};
use wasmlanche::{
    simulator::{Error as SimError, SimpleState, Simulator},
    Address,
};
use token::Units;

/// If your contract references these from your “mod types” or “type.rs”
use crate::core::{ContractState, ContractTerms, ContractType, EventType};
// Or adjust to your actual code location, e.g. "use actus::{ContractState, ...};"

/// If you have environment variables for the compiled WASM:
const CONTRACT_PATH: &str = env!("CONTRACT_PATH");  // or define a static path if you prefer

#[test]
fn test_pam_integration() -> Result<(), SimError> {
    // 1. Create an in-memory chain state and simulator
    let mut state = SimpleState::new();
    let simulator = Simulator::new(&mut state);

    // 2. Deploy your ACTUS contract
    //    Make sure CONTRACT_PATH points to the compiled WASM artifact
    let deployed_contract = simulator.create_contract(CONTRACT_PATH)?;
    let contract_address = deployed_contract.address;

    // 3. Optionally deploy or reference a token contract for settlement, if needed
    //    Here, we create a simple test token
    let token_contract = simulator.create_contract("PATH_TO_TOKEN_WASM")?; 
    let token_address = token_contract.address;

    // 4. Initialize the token (this depends on your token’s “init” signature)
    simulator.call_contract::<(), _>(
        token_address,
        "init",
        ("TestToken", "TT"),
        10_000_000
    )?;

    // 5. Optionally mint tokens to a user for testing
    let alice = Address::new([1; 33]);
    simulator.set_actor(alice);
    simulator.call_contract::<(), _>(
        token_address,
        "mint",
        (alice, 1_000_000u64),
        10_000_000
    )?;

    // 6. Build or Borsh-serialize some minimal `ContractTerms`
    let terms_bytes = create_pam_terms(token_address); // see below

    // 7. Initialize the ACTUS contract
    //    “init” matches your contract’s `init(context, contract_type, contract_role, currency, terms)`
    simulator.call_contract::<(), _>(
        contract_address,
        "init",
        (
            ContractType::PAM as u8, // or your numeric code for PAM
            0u8,                     // contract_role if needed
            token_address,           // currency
            terms_bytes,             // Borsh-serialized ContractTerms
        ),
        10_000_000
    )?;

    // 8. Helper function to process events
    fn process_event(
        sim: &Simulator<SimpleState>,
        contract_addr: Address,
        evt_type: EventType,
        timestamp: u64,
    ) -> Result<Option<Units>, SimError> {
        // This calls “process_event(u8, u64)” with the event type + timestamp
        sim.call_contract::<Option<Units>, _>(
            contract_addr,
            "process_event",
            (evt_type as u8, timestamp),
            10_000_000
        )
    }

    // 9. Now we can trigger events (IED at t=1000, IP at t=1100, etc.)
    //    This is an example—adapt to your actual logic
    let ied_result = process_event(&simulator, contract_address, EventType::IED, 1000)?;
    println!("IED result: {:?}", ied_result);

    let ip_result = process_event(&simulator, contract_address, EventType::IP, 1100)?;
    println!("IP result: {:?}", ip_result);

    let pr_result = process_event(&simulator, contract_address, EventType::PR, 1200)?;
    println!("PR result: {:?}", pr_result);

    let md_result = process_event(&simulator, contract_address, EventType::MD, 1300)?;
    println!("MD result: {:?}", md_result);

    // 10. Query final state to check principal=0, interest=0, etc.
    let final_state: ContractState = simulator.call_contract(
        contract_address,
        "get_state",
        (),
        10_000_000
    )?;
    println!("Final contract state: {:?}", final_state);

    // 11. Asserts
    assert_eq!(final_state.notional_principal, 0);
    assert_eq!(final_state.accrued_interest, 0);

    Ok(())
}

/// Example function to create minimal “PAM” terms and Borsh-serialize them
fn create_pam_terms(settlement_currency: Address) -> Vec<u8> {
    use borsh::BorshSerialize;

    let terms = ContractTerms {
        // Fill in the fields your “init” logic or “transitions” code expects
        contract_id: "pam-contract".to_string(),
        contract_type: ContractType::PAM,
        contract_role: ContractRole::CR_RPA, 
        settlement_currency: Some(settlement_currency.as_ref().to_vec()),

        // e.g. a simple scenario
        initial_exchange_date: Some(1000),
        notional_principal: Some(500_000),
        nominal_interest_rate: Some(50_000), // 5% in basis points
        maturity_date: Some(1300),

        // fill other fields as needed or default them
        status_date: 1000, // or context.timestamp at init
        schedule_config: ScheduleConfig {
            calendar: None,
            end_of_month_convention: None,
            business_day_convention: None,
        },
        ..Default::default()
    };

    terms.try_to_vec().unwrap()
}
