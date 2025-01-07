// src/type.rs

use borsh::{BorshDeserialize, BorshSerialize};

////////////////////////////////////////////////////////////////////////////////
// 1. Core "Timestamp" Type
////////////////////////////////////////////////////////////////////////////////

/// If you prefer a simple alias for Unix timestamp:
pub type Timestamp = u64;

////////////////////////////////////////////////////////////////////////////////
// 2. EVENT TYPES
////////////////////////////////////////////////////////////////////////////////

/// All ACTUS event types, matching "BusinessEvents.hs"
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum EventType {
    IED  = 0,   // Initial Exchange
    FP   = 1,   // Fee Payment
    PR   = 2,   // Principal Redemption
    PD   = 3,   // Principal Drawing
    PY   = 4,   // Penalty Payment
    PP   = 5,   // Principal Prepayment
    IP   = 6,   // Interest Payment
    IPFX = 7,   // Interest Payment Fixed Leg
    IPFL = 8,   // Interest Payment Floating Leg
    IPCI = 9,   // Interest Capitalization
    CE   = 10,  // Credit Event
    RRF  = 11,  // Rate Reset Fixing with Known Rate
    RR   = 12,  // Rate Reset Fixing with Unknown Rate
    PRF  = 13,  // Principal Payment Amount Fixing
    DV   = 14,  // Dividend Payment
    PRD  = 15,  // Purchase
    MR   = 16,  // Margin Call
    TD   = 17,  // Termination
    SC   = 18,  // Scaling Index Fixing
    IPCB = 19,  // Interest Calculation Base Fixing
    MD   = 20,  // Maturity
    XD   = 21,  // Exercise
    STD  = 22,  // Settlement
    PI   = 23,  // Principal Increase
    AD   = 24,  // Monitoring
}

////////////////////////////////////////////////////////////////////////////////
// 3. CONTRACT STATE
////////////////////////////////////////////////////////////////////////////////

/// This mirrors "ContractState.hs".
/// We've replaced `LocalTime` with `Timestamp` (u64).
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ContractState {
    // tmd  :: Maybe LocalTime
    pub time_of_maturity: Option<Timestamp>,

    // nt   :: a
    pub notional_principal: u64,

    // ipnr :: a
    pub nominal_interest_rate: u64,

    // ipac :: a
    pub accrued_interest: u64,

    // ipac1 :: Maybe a
    pub accrued_interest_first_leg: Option<u64>,

    // ipac2 :: Maybe a
    pub accrued_interest_second_leg: Option<u64>,

    // ipla :: Maybe a
    pub last_interest_period: Option<u64>,

    // feac :: a
    pub fee_accrued: u64,

    // nsc :: a
    pub notional_scaling_multiplier: u64,

    // isc :: a
    pub interest_scaling_multiplier: u64,

    // prf :: PRF
    pub contract_performance: ContractPerformance,

    // sd   :: LocalTime
    pub status_date: Timestamp,

    // prnxt :: a
    pub next_principal_redemption_payment: u64,

    // ipcb :: a
    pub interest_calculation_base: u64,

    // xd :: Maybe LocalTime
    pub exercise_date: Option<Timestamp>,

    // xa :: Maybe a
    pub exercise_amount: Option<u64>,
}

/// Contract performance states, from `PRF_PF, PRF_DL, PRF_DQ, PRF_DF`.
#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum ContractPerformance {
    PF = 0, // Performant
    DL = 1, // Delayed
    DQ = 2, // Delinquent
    DF = 3, // Default
}

////////////////////////////////////////////////////////////////////////////////
// 4. CONTRACT TERMS
////////////////////////////////////////////////////////////////////////////////

/// Matches "ContractTerms.hs" but uses simple `Timestamp` for date/time fields.
/// For numeric fields (like interest rates), we use `u64`. You can adjust as needed.
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ContractTerms {
    // General
    pub contract_id: String, 
    pub contract_type: ContractType,
    pub contract_role: ContractRole,
    pub settlement_currency: Option<String>,

    // Calendar
    pub initial_exchange_date: Option<Timestamp>,
    pub day_count_convention: Option<DayCountConvention>,
    pub schedule_config: ScheduleConfig,

    // Identification
    pub status_date: Timestamp,
    pub market_object_code: Option<String>,

    // Performance / credit
    pub contract_performance: Option<ContractPerformance>,
    // If you want coverageOfCreditEnhancement or creditEventTypeCovered, add them here:
    // pub credit_event_type_covered: Option<CreditEventTypeCovered>,
    // pub coverage_of_credit_enhancement: Option<u64>,
    // etc. – remove if not needed.

    // Fees
    pub cycle_of_fee: Option<Cycle>,
    pub cycle_anchor_date_of_fee: Option<Timestamp>,
    pub fee_accrued: Option<u64>,
    pub fee_basis: Option<FeeBasis>,
    pub fee_rate: Option<u64>,

    // Interest
    pub cycle_anchor_date_of_interest_payment: Option<Timestamp>,
    pub cycle_of_interest_payment: Option<Cycle>,
    pub accrued_interest: Option<u64>,
    pub capitalization_end_date: Option<Timestamp>,
    pub cycle_anchor_date_of_interest_calculation_base: Option<Timestamp>,
    pub cycle_of_interest_calculation_base: Option<Cycle>,
    pub interest_calculation_base: Option<IPCB>,
    pub interest_calculation_base_amount: Option<u64>,
    pub nominal_interest_rate: Option<u64>,
    pub nominal_interest_rate2: Option<u64>,
    pub interest_scaling_multiplier: Option<u64>,

    // Dates
    pub maturity_date: Option<Timestamp>,
    pub amortization_date: Option<Timestamp>,
    pub exercise_date: Option<Timestamp>,

    // Notional
    pub notional_principal: Option<u64>,
    pub premium_discount_at_ied: Option<u64>,
    pub cycle_anchor_date_of_principal_redemption: Option<Timestamp>,
    pub cycle_of_principal_redemption: Option<Cycle>,
    pub next_principal_redemption_payment: Option<u64>,
    pub purchase_date: Option<Timestamp>,
    pub price_at_purchase_date: Option<u64>,
    pub termination_date: Option<Timestamp>,
    pub price_at_termination_date: Option<u64>,
    pub quantity: Option<u64>,
    pub currency: Option<String>,
    pub currency2: Option<String>,

    // Scaling
    pub scaling_effect: Option<ScalingEffect>,
    pub scaling_index_at_status_date: Option<u64>,
    pub cycle_anchor_date_of_scaling_index: Option<Timestamp>,
    pub cycle_of_scaling_index: Option<Cycle>,
    pub scaling_index_at_contract_deal_date: Option<u64>,
    pub market_object_code_of_scaling_index: Option<String>,
    pub notional_scaling_multiplier: Option<u64>,

    // Rate Reset Fields
    // pub cycle_anchor_date_of_rate_reset: Option<Timestamp>,
    // pub cycle_of_rate_reset: Option<Cycle>,
    // pub next_reset_rate: Option<u64>,
    // pub rate_spread: Option<u64>,
    // pub rate_multiplier: Option<u64>,
    // pub period_floor: Option<u64>,
    // pub period_cap: Option<u64>,
    // pub life_cap: Option<u64>,
    // pub life_floor: Option<u64>,
    // pub market_object_code_of_rate_reset: Option<String>,

    // Penalty
    pub penalty_rate: Option<u64>,
    pub penalty_type: Option<PenaltyType>,
    pub prepayment_effect: Option<PrepaymentEffect>,
}

/// ContractType from `CT = PAM, LAM, NAM, ANN, STK, ...`
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum ContractType {
    PAM   = 0, // Principal at maturity
    LAM   = 1, // Linear amortizer
    NAM   = 2, // Negative amortizer
    ANN   = 3, // Annuity
    STK   = 4, // Stock
    OPTNS = 5, // Option
    FUTUR = 6, // Future
    COM   = 7, // Commodity
    CSH   = 8, // Cash
    CLM   = 9, // Call Money
    SWPPV = 10,// Plain Vanilla Swap
    SWAPS = 11,// Swap
    CEG   = 12,// Guarantee
    CEC   = 13 // Collateral
}

/// ContractRole from `CR_RPA, CR_RPL, CR_CLO, ...`
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum ContractRole {
    CR_RPA = 0,
    CR_RPL = 1,
    CR_CLO = 2,
    CR_CNO = 3,
    CR_COL = 4,
    CR_LG  = 5,
    CR_ST  = 6,
    CR_BUY = 7,
    CR_SEL = 8,
    CR_RFL = 9,
    CR_PFL = 10,
    CR_RF  = 11,
    CR_PF  = 12,
}

/// DayCountConvention from `DCC_A_AISDA, ...`
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum DayCountConvention {
    DCC_A_AISDA     = 0, // "AA"
    DCC_A_360       = 1, // "A360"
    DCC_A_365       = 2, // "A365"
    DCC_E30_360ISDA = 3, // "30E360ISDA"
    DCC_E30_360     = 4, // "30E360"
    DCC_B_252       = 5, // "B252"
}

/// EndOfMonthConvention from `EOMC_EOM, EOMC_SD`
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum EndOfMonthConvention {
    EOMC_EOM = 0, // End of month
    EOMC_SD  = 1  // Same day
}

/// BusinessDayConvention
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum BusinessDayConvention {
    BDC_NULL = 0,
    BDC_SCF  = 1,
    BDC_SCMF = 2,
    BDC_CSF  = 3,
    BDC_CSMF = 4,
    BDC_SCP  = 5,
    BDC_SCMP = 6,
    BDC_CSP  = 7,
    BDC_CSMP = 8,
}

/// Calendar from `CLDR_MF, CLDR_NC`
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum Calendar {
    CLDR_MF = 0, // Monday-Friday
    CLDR_NC = 1, // No calendar
}

/// A small struct to hold schedule config
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ScheduleConfig {
    pub calendar: Option<Calendar>,
    pub end_of_month_convention: Option<EndOfMonthConvention>,
    pub business_day_convention: Option<BusinessDayConvention>,
}

/// PRF = ContractPerformance is above.

/// Additional enumerations if needed for coverageOfCreditEnhancement, creditEventTypeCovered, etc.
/// e.g., GuaranteedExposure, FeeBasis, IPCB, SCEF, etc. 
/// Shown below are some you might need:

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum GuaranteedExposure {
    CEGE_NO = 0,
    CEGE_NI = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum FeeBasis {
    FEB_A = 0,
    FEB_N = 1,
}

/// IPCB = InterestCalculationBase in Haskell
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum IPCB {
    IPCB_NT    = 0,
    IPCB_NTIED = 1,
    IPCB_NTL   = 2,
}

/// SCEF = ScalingEffect in Haskell
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum ScalingEffect {
    SE_OOO = 0,
    SE_IOO = 1,
    SE_ONO = 2,
    SE_OOM = 3,
    SE_INO = 4,
    SE_ONM = 5,
    SE_IOM = 6,
    SE_INM = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum PenaltyType {
    PYTP_A = 0,
    PYTP_N = 1,
    PYTP_I = 2,
    PYTP_O = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum PrepaymentEffect {
    PPEF_N = 0,
    PPEF_A = 1,
    PPEF_M = 2,
}

/// Period / Cycle structs
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum Period {
    P_D = 0,
    P_W = 1,
    P_M = 2,
    P_Q = 3,
    P_H = 4,
    P_Y = 5,
}

/// Stub enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum Stub {
    ShortStub = 0,
    LongStub  = 1,
}

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Cycle {
    pub n: i64,
    pub p: Period,
    pub stub: Stub,
    pub include_end_day: bool,
}

////////////////////////////////////////////////////////////////////////////////
// 5. SHIFTED DAY
////////////////////////////////////////////////////////////////////////////////

/// Equivalent to Haskell's `ShiftedDay { paymentDay, calculationDay }`
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ShiftedDay {
    pub payment_day: Timestamp,
    pub calculation_day: Timestamp,
}

impl ShiftedDay {
    pub fn new(payment_day: Timestamp, calculation_day: Timestamp) -> Self {
        Self { payment_day, calculation_day }
    }
    pub fn from_single(day: Timestamp) -> Self {
        Self { payment_day: day, calculation_day: day }
    }
}
