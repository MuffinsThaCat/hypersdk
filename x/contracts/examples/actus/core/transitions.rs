use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::Units;

use super::types::*;
use super::Result;
use crate::math;

pub struct TransitionEngine;

impl TransitionEngine {
    /// Main entry point for processing an ACTUS event
    pub fn process_event(
        event: EventType,
        timestamp: u64,
        state: &mut ContractState,
        terms: &ContractTerms
    ) -> Result<Option<Units>> {
        // Validate event timing
        if timestamp < state.status_date {
            return Err(Error::TransitionError("Event timestamp before status date".into()));
        }

        // Update accrued interest if time has passed
        if timestamp > state.status_date {
            Self::update_accrued_interest(state, terms, timestamp)?;
        }

        // Dispatch based on contract type (PAM, LAM, NAM, ANN, etc.)
        let result = match terms.contract_type {
            ContractType::PAM => Self::process_pam_event(event, timestamp, state, terms),
            ContractType::LAM => Self::process_lam_event(event, timestamp, state, terms),
            ContractType::NAM => Self::process_nam_event(event, timestamp, state, terms),
            ContractType::ANN => Self::process_ann_event(event, timestamp, state, terms),
            // If you want to handle e.g. SWPPV, STK, CLM, etc., add them here:
            // ContractType::STK => Self::process_stk_event(event, timestamp, state, terms),
            // ...
            _ => Err(Error::TransitionError("Unsupported contract type".into())),
        }?;

        // Update status date
        state.status_date = timestamp;

        Ok(result)
    }

    // =======================
    //        PAM Logic
    // =======================
    fn process_pam_event(
        event: EventType,
        timestamp: u64,
        state: &mut ContractState,
        terms: &ContractTerms
    ) -> Result<Option<Units>> {
        match event {
            // Initial Exchange
            EventType::IED => {
                if let Some(ied) = terms.initial_exchange_date {
                    if timestamp == ied {
                        if let Some(notional) = terms.notional_principal {
                            // Set up initial principal and interest
                            state.notional_principal = notional;
                            state.nominal_interest_rate = terms.nominal_interest_rate.unwrap_or(0);
                            state.accrued_interest = 0;
                            return Ok(Some(notional));
                        }
                    }
                }
                Ok(None)
            },

            // Interest Payment
            EventType::IP => {
                if state.accrued_interest > 0 {
                    let payment = state.accrued_interest;
                    state.accrued_interest = 0;
                    Ok(Some(payment))
                } else {
                    Ok(None)
                }
            },

            // Principal Redemption at Maturity
            EventType::MD => {
                if let Some(md) = terms.maturity_date {
                    if timestamp == md {
                        let principal = state.notional_principal;
                        let interest = state.accrued_interest;
                        let total_payment = principal + interest;

                        // Zero out principal/interest
                        state.notional_principal = 0;
                        state.accrued_interest = 0;
                        state.nominal_interest_rate = 0;

                        return Ok(Some(total_payment));
                    }
                }
                Ok(None)
            },

            // Regular Monitoring/Accrual
            EventType::AD => {
                Self::update_accrued_interest(state, terms, timestamp)?;
                Ok(None)
            },

            // Rate Reset
            EventType::RR => {
                // If your contract logic supports `next_reset_rate`
                if let Some(rate) = terms.next_reset_rate {
                    state.nominal_interest_rate = rate;
                }
                Ok(None)
            },

            // Fee Payment
            EventType::FP => {
                // If your contract has a fee_accrued or fee fields, pay them here
                // e.g., let paid_fee = state.fee_accrued; state.fee_accrued = 0; Ok(Some(paid_fee))
                Ok(None)
            },

            // Principal Prepayment
            EventType::PP => {
                if let Some(pp_effect) = terms.prepayment_effect {
                    match pp_effect {
                        PrepaymentEffect::PPEF_N => Ok(None), // No prepayment
                        PrepaymentEffect::PPEF_A => {
                            // Reduce next principal redemption, if any
                            if state.next_principal_redemption_payment > 0 {
                                let amount = state.next_principal_redemption_payment;
                                state.next_principal_redemption_payment = 0;
                                Ok(Some(amount))
                            } else {
                                Ok(None)
                            }
                        },
                        PrepaymentEffect::PPEF_M => {
                            // Wipe out entire principal
                            let amount = state.notional_principal;
                            state.notional_principal = 0;
                            Ok(Some(amount))
                        }
                    }
                } else {
                    Ok(None)
                }
            },

            // Additional event examples:
            // EventType::PD => { /* Principal Drawing logic, if any */ Ok(None) },
            // EventType::PY => { /* Penalty Payment */ Ok(None) },
            // EventType::PRD => { /* Purchase event, if used */ Ok(None) },

            _ => Ok(None)
        }
    }

    // =======================
    //        LAM Logic
    // =======================
    fn process_lam_event(
        event: EventType,
        timestamp: u64,
        state: &mut ContractState,
        terms: &ContractTerms
    ) -> Result<Option<Units>> {
        match event {
            // Initial Exchange
            EventType::IED => {
                if let Some(ied) = terms.initial_exchange_date {
                    if timestamp == ied {
                        if let Some(notional) = terms.notional_principal {
                            state.notional_principal = notional;
                            state.nominal_interest_rate = terms.nominal_interest_rate.unwrap_or(0);
                            state.accrued_interest = 0;
                            return Ok(Some(notional));
                        }
                    }
                }
                Ok(None)
            },

            // Interest Payment
            EventType::IP => {
                if state.accrued_interest > 0 {
                    let payment = state.accrued_interest;
                    state.accrued_interest = 0;
                    Ok(Some(payment))
                } else {
                    Ok(None)
                }
            },

            // Principal Redemption
            EventType::PR => {
                // For LAM, typically a linear chunk each time or from terms.next_principal_redemption_payment
                let redemption_amt = terms.next_principal_redemption_payment.unwrap_or(0);
                let redemption = redemption_amt.min(state.notional_principal);
                state.notional_principal = state.notional_principal.saturating_sub(redemption);
                Ok(Some(redemption))
            },

            // Maturity
            EventType::MD => {
                if let Some(md) = terms.maturity_date {
                    if timestamp == md {
                        let principal = state.notional_principal;
                        let interest = state.accrued_interest;
                        let total_payment = principal + interest;

                        // Zero out
                        state.notional_principal = 0;
                        state.accrued_interest = 0;
                        state.nominal_interest_rate = 0;

                        return Ok(Some(total_payment));
                    }
                }
                Ok(None)
            },

            // Additional event examples, if desired:
            // EventType::PD => { /* Principal Drawing for LAM, if used */ Ok(None) },
            // etc.

            _ => Ok(None)
        }
    }

    // =======================
    //        NAM Logic
    // =======================
    fn process_nam_event(
        event: EventType,
        timestamp: u64,
        state: &mut ContractState,
        terms: &ContractTerms
    ) -> Result<Option<Units>> {
        match event {
            // Initial Exchange
            EventType::IED => {
                if let Some(ied) = terms.initial_exchange_date {
                    if timestamp == ied {
                        if let Some(notional) = terms.notional_principal {
                            state.notional_principal = notional;
                            state.nominal_interest_rate = terms.nominal_interest_rate.unwrap_or(0);
                            state.accrued_interest = 0;
                            return Ok(Some(notional));
                        }
                    }
                }
                Ok(None)
            },

            // In NAM, interest might be capitalized rather than paid
            EventType::IP => {
                // If your NAM logic says skip IP, do that here
                Ok(None)
            },

            // Interest Capitalization
            EventType::IPCI => {
                let accrued = state.accrued_interest;
                state.notional_principal = state.notional_principal.saturating_add(accrued);
                state.accrued_interest = 0;
                Ok(Some(accrued)) // or Ok(None) if you don't consider a flow
            },

            // Maturity
            EventType::MD => {
                if let Some(md) = terms.maturity_date {
                    if timestamp == md {
                        let total = state.notional_principal + state.accrued_interest;
                        state.notional_principal = 0;
                        state.accrued_interest = 0;
                        state.nominal_interest_rate = 0;
                        return Ok(Some(total));
                    }
                }
                Ok(None)
            },

            // Additional events, if desired
            // e.g., PP, PD, etc.

            _ => Ok(None)
        }
    }

    // =======================
    //        ANN Logic
    // =======================
    fn process_ann_event(
        event: EventType,
        timestamp: u64,
        state: &mut ContractState,
        terms: &ContractTerms
    ) -> Result<Option<Units>> {
        match event {
            // Initial Exchange
            EventType::IED => {
                if let Some(ied) = terms.initial_exchange_date {
                    if timestamp == ied {
                        if let Some(notional) = terms.notional_principal {
                            state.notional_principal = notional;
                            state.nominal_interest_rate = terms.nominal_interest_rate.unwrap_or(0);
                            state.accrued_interest = 0;
                            return Ok(Some(notional));
                        }
                    }
                }
                Ok(None)
            },

            // Annuity Payment (interest + principal)
            EventType::IP => {
                let outstanding = state.notional_principal;
                let interest_portion = state.accrued_interest;

                // Example stub for computing a fixed installment
                let annuity_payment = Self::compute_annuity_payment(terms, outstanding)?;

                let principal_portion = annuity_payment.saturating_sub(interest_portion);

                // Update principal
                state.notional_principal = outstanding.saturating_sub(principal_portion);
                // Reset accrued interest
                state.accrued_interest = 0;

                Ok(Some(annuity_payment))
            },

            // Maturity
            EventType::MD => {
                if let Some(md) = terms.maturity_date {
                    if timestamp == md {
                        let total_payment = state.notional_principal + state.accrued_interest;
                        state.notional_principal = 0;
                        state.accrued_interest = 0;
                        state.nominal_interest_rate = 0;
                        return Ok(Some(total_payment));
                    }
                }
                Ok(None)
            },

            _ => Ok(None)
        }
    }

    // =======================
    //   Shared Logic
    // =======================
    /// Recalculates accrued interest from `status_date` to `timestamp` using day-count
    fn update_accrued_interest(
        state: &mut ContractState,
        terms: &ContractTerms,
        timestamp: u64,
    ) -> Result<()> {
        // If the contract defines a day count convention, compute interest
        if let Some(dcc) = terms.day_count_convention {
            let time_fraction = math::year_fraction(
                dcc,
                state.status_date,
                timestamp,
                terms.maturity_date
            );

            // e.g. interest = principal * rate * time_fraction
            let accrual = state
                .notional_principal
                .checked_mul(state.nominal_interest_rate)
                .ok_or(Error::MathError("Interest calculation overflow".into()))?
                .checked_mul(time_fraction)
                .ok_or(Error::MathError("Interest calculation overflow".into()))?;

            // Add to accrued_interest
            state.accrued_interest = state
                .accrued_interest
                .checked_add(accrual)
                .ok_or(Error::MathError("Interest accrual overflow".into()))?;
        }

        Ok(())
    }

    /// Example placeholder for computing a fixed annuity payment
    /// In reality, you'd use a more precise formula referencing period length, rate, etc.
    fn compute_annuity_payment(terms: &ContractTerms, outstanding: Units) -> Result<Units> {
        // Could incorporate day-count fraction, rate, schedule, etc.
        // For demonstration, just return 1/10th of the outstanding
        let payment = outstanding / 10; 
        Ok(payment)
    }
}

// =======================
// Unit Tests
// =======================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pam_initial_exchange() {
        let mut state = ContractState {
            status_date: 1000,
            notional_principal: 0,
            nominal_interest_rate: 0,
            accrued_interest: 0,
            contract_performance: ContractPerformance::PF,
            fee_accrued: 0,
            notional_scaling_multiplier: 0,
            interest_scaling_multiplier: 0,
            next_principal_redemption_payment: 0,
            interest_calculation_base: 0,
            exercise_date: None,
            exercise_amount: None,
            // Additional fields from your new struct:
            time_of_maturity: None,
            accrued_interest_first_leg: None,
            accrued_interest_second_leg: None,
            last_interest_period: None,
        };

        let terms = ContractTerms {
            contract_type: ContractType::PAM,
            initial_exchange_date: Some(1000),
            notional_principal: Some(1_000_000),
            nominal_interest_rate: Some(50_000), // 5% in basis points
            day_count_convention: None,
            schedule_config: ScheduleConfig {
                calendar: None,
                end_of_month_convention: None,
                business_day_convention: None,
            },
            status_date: 1000,
            // Fill in any other fields as needed
            ..Default::default()
        };

        let result = TransitionEngine::process_event(
            EventType::IED,
            1000,
            &mut state,
            &terms
        ).unwrap();

        assert_eq!(result, Some(1_000_000));
        assert_eq!(state.notional_principal, 1_000_000);
        assert_eq!(state.nominal_interest_rate, 50_000);
        assert_eq!(state.accrued_interest, 0);
    }

    // Add more tests for LAM, NAM, ANN, etc.
}
