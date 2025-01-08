use borsh::{BorshDeserialize, BorshSerialize};

use super::core::{
    BusinessDayConvention, Calendar, Cycle, EndOfMonthConvention, Period,
    ScheduleConfig, Timestamp,
};

const SECONDS_PER_DAY: u64 = 86400;
const DAYS_PER_MONTH: u64 = 30;  // Using 30/360 convention
const MONTHS_PER_YEAR: u64 = 12;

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ShiftedDay {
    pub payment_day: Timestamp,
    pub calculation_day: Timestamp,
}

impl ShiftedDay {
    pub fn new(payment_day: Timestamp, calculation_day: Timestamp) -> Self {
        Self {
            payment_day,
            calculation_day,
        }
    }

    pub fn from_single(day: Timestamp) -> Self {
        Self {
            payment_day: day,
            calculation_day: day,
        }
    }
}

pub struct ScheduleGenerator;

impl ScheduleGenerator {
    /// Generates a schedule of events based on the given parameters
    pub fn generate_schedule(
        anchor_date: Timestamp,
        cycle: &Cycle,
        end_date: Timestamp,
        config: &ScheduleConfig,
    ) -> Vec<ShiftedDay> {
        if cycle.n <= 0 || anchor_date >= end_date {
            return vec![];
        }

        let mut dates = Vec::new();
        let mut current = anchor_date;

        while current <= end_date {
            // Apply business day convention if configured
            let shifted_day = if let Some(bdc) = config.business_day_convention {
                Self::apply_business_day_convention(
                    current,
                    bdc,
                    config.calendar.unwrap_or(Calendar::NC),
                )
            } else {
                ShiftedDay::from_single(current)
            };

            dates.push(shifted_day);
            current = Self::shift_date(current, cycle);

            // Apply end of month convention if configured
            if let Some(eomc) = config.end_of_month_convention {
                current = Self::apply_end_of_month_convention(current, eomc);
            }
        }

        // Handle stub periods
        if cycle.stub {
            Self::adjust_for_short_stub(&mut dates, end_date);
        } else {
            Self::adjust_for_long_stub(&mut dates, end_date);
        }

        dates
    }

    /// Shifts a date according to a cycle
    fn shift_date(date: Timestamp, cycle: &Cycle) -> Timestamp {
        let seconds_to_add = match cycle.p {
            Period::D => cycle.n as u64 * SECONDS_PER_DAY,
            Period::W => cycle.n as u64 * 7 * SECONDS_PER_DAY,
            Period::M => cycle.n as u64 * DAYS_PER_MONTH * SECONDS_PER_DAY,
            Period::Q => cycle.n as u64 * 3 * DAYS_PER_MONTH * SECONDS_PER_DAY,
            Period::H => cycle.n as u64 * 6 * DAYS_PER_MONTH * SECONDS_PER_DAY,
            Period::Y => cycle.n as u64 * MONTHS_PER_YEAR * DAYS_PER_MONTH * SECONDS_PER_DAY,
        };

        date + seconds_to_add
    }

    /// Applies business day convention to a date
    fn apply_business_day_convention(
        date: Timestamp,
        convention: BusinessDayConvention,
        calendar: Calendar,
    ) -> ShiftedDay {
        match (convention, calendar) {
            (BusinessDayConvention::NULL, _) => ShiftedDay::from_single(date),
            
            (BusinessDayConvention::SCF, Calendar::MF) => {
                let calc_day = date;
                let pay_day = Self::next_business_day(date);
                ShiftedDay::new(pay_day, calc_day)
            }

            (BusinessDayConvention::SCMF, Calendar::MF) => {
                let calc_day = date;
                let mut pay_day = Self::next_business_day(date);
                
                // If shifted to next month, use preceding instead
                if Self::get_month(pay_day) != Self::get_month(date) {
                    pay_day = Self::previous_business_day(date);
                }
                
                ShiftedDay::new(pay_day, calc_day)
            }

            (BusinessDayConvention::CSF, Calendar::MF) => {
                let calc_day = date;
                let pay_day = Self::next_business_day(date);
                ShiftedDay::new(pay_day, calc_day)
            }

            (BusinessDayConvention::SCP, Calendar::MF) => {
                let calc_day = date;
                let pay_day = Self::previous_business_day(date);
                ShiftedDay::new(pay_day, calc_day)
            }

            // If no calendar or other conventions, return unchanged
            _ => ShiftedDay::from_single(date),
        }
    }

    /// Applies end of month convention
    fn apply_end_of_month_convention(date: Timestamp, convention: EndOfMonthConvention) -> Timestamp {
        match convention {
            EndOfMonthConvention::EOM => {
                if Self::is_end_of_month(date) {
                    let next_month = Self::add_months(date, 1);
                    Self::last_day_of_month(next_month)
                } else {
                    date
                }
            }
            EndOfMonthConvention::SD => date,
        }
    }

    /// Adjusts schedule for short stub period
    fn adjust_for_short_stub(dates: &mut Vec<ShiftedDay>, end_date: Timestamp) {
        if let Some(last) = dates.last() {
            if last.payment_day < end_date {
                dates.push(ShiftedDay::from_single(end_date));
            }
        }
    }

    /// Adjusts schedule for long stub period
    fn adjust_for_long_stub(dates: &mut Vec<ShiftedDay>, end_date: Timestamp) {
        if let Some(last) = dates.last() {
            if last.payment_day > end_date {
                dates.pop();
                dates.push(ShiftedDay::from_single(end_date));
            }
        }
    }

    // Helper functions
    fn next_business_day(date: Timestamp) -> Timestamp {
        let weekday = Self::get_weekday(date);
        match weekday {
            5 => date + 3 * SECONDS_PER_DAY, // Friday -> Monday
            6 => date + 2 * SECONDS_PER_DAY, // Saturday -> Monday
            _ => if weekday < 5 { date + SECONDS_PER_DAY } else { date },
        }
    }

    fn previous_business_day(date: Timestamp) -> Timestamp {
        let weekday = Self::get_weekday(date);
        match weekday {
            0 => date - 2 * SECONDS_PER_DAY, // Sunday -> Friday
            6 => date - 1 * SECONDS_PER_DAY, // Saturday -> Friday
            _ => if weekday > 1 { date - SECONDS_PER_DAY } else { date },
        }
    }

    fn get_weekday(timestamp: Timestamp) -> u64 {
        // Unix timestamp starts on Thursday (4)
        // So we add 4 before taking modulo 7
        ((timestamp / SECONDS_PER_DAY) + 4) % 7
    }

    fn get_month(timestamp: Timestamp) -> u64 {
        // Approximate month calculation
        (timestamp / (SECONDS_PER_DAY * DAYS_PER_MONTH)) % 12
    }

    fn is_end_of_month(timestamp: Timestamp) -> bool {
        let next_day = timestamp + SECONDS_PER_DAY;
        Self::get_month(next_day) != Self::get_month(timestamp)
    }

    fn last_day_of_month(timestamp: Timestamp) -> Timestamp {
        let current_month = Self::get_month(timestamp);
        let mut day = timestamp;
        
        while Self::get_month(day + SECONDS_PER_DAY) == current_month {
            day += SECONDS_PER_DAY;
        }
        
        day
    }

    fn add_months(timestamp: Timestamp, months: u64) -> Timestamp {
        timestamp + (months * DAYS_PER_MONTH * SECONDS_PER_DAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_monthly_schedule() {
        let anchor_date = 1640995200; // 2022-01-01
        let cycle = Cycle {
            n: 1,
            p: Period::M,
            stub: true,
            include_end_day: true,
        };
        let end_date = anchor_date + (3 * DAYS_PER_MONTH * SECONDS_PER_DAY);
        let config = ScheduleConfig {
            calendar: None,
            end_of_month_convention: None,
            business_day_convention: None,
        };

        let schedule = ScheduleGenerator::generate_schedule(
            anchor_date,
            &cycle,
            end_date,
            &config,
        );

        assert_eq!(schedule.len(), 4);
        assert_eq!(schedule[0].payment_day, anchor_date);
        assert_eq!(
            schedule[1].payment_day,
            anchor_date + (DAYS_PER_MONTH * SECONDS_PER_DAY)
        );
    }

    #[test]
    fn test_business_day_convention() {
        let friday = 1640995200; // Assuming this is a Friday
        let shifted = ScheduleGenerator::apply_business_day_convention(
            friday,
            BusinessDayConvention::SCF,
            Calendar::MF,
        );
        
        // Should shift to Monday
        assert_eq!(
            shifted.payment_day,
            friday + (3 * SECONDS_PER_DAY)
        );
    }

    #[test]
    fn test_end_of_month_convention() {
        let end_of_jan = 1643673600; // 2022-01-31
        let shifted = ScheduleGenerator::apply_end_of_month_convention(
            end_of_jan,
            EndOfMonthConvention::EOM,
        );
        
        // Should be last day of next month
        assert!(ScheduleGenerator::is_end_of_month(shifted));
    }
}
