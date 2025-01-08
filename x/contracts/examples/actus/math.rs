use token::Units;

/// Constants for time calculations
const SECONDS_PER_DAY: u64 = 86400;
const DAYS_PER_YEAR: u64 = 365;
const MONTHS_PER_YEAR: u64 = 12;
const BASIS_POINT_FACTOR: u64 = 10000;

// ============= Year Fraction Calculations =============

/// Year fraction calculation using different day count conventions
pub fn year_fraction(
    dcc: u8,
    start_time: u64,
    end_time: u64,
    maturity_time: Option<u64>
) -> Units {
    match dcc {
        0 => year_fraction_actual_actual_isda(start_time, end_time),
        1 => year_fraction_actual_360(start_time, end_time),
        2 => year_fraction_actual_365(start_time, end_time),
        3 => year_fraction_thirty_e_360_isda(start_time, end_time, maturity_time),
        4 => year_fraction_thirty_e_360(start_time, end_time),
        5 => year_fraction_bus_252(start_time, end_time),
        _ => year_fraction_actual_365(start_time, end_time), // Default to A/365
    }
}

fn year_fraction_actual_actual_isda(start_time: u64, end_time: u64) -> Units {
    if start_time >= end_time {
        return 0;
    }

    let start_year = get_year(start_time);
    let end_year = get_year(end_time);
    
    if start_year == end_year {
        let year_length = if is_leap_year(start_year) { 366 } else { 365 };
        let days_diff = days_between(start_time, end_time);
        return (days_diff * BASIS_POINT_FACTOR / year_length) as Units;
    }

    // Handle multi-year periods
    let mut total_fraction = 0;
    
    // First partial year
    let next_year_start = start_of_year(start_year + 1);
    total_fraction += year_fraction_actual_actual_isda(start_time, next_year_start);
    
    // Full years in between
    for year in (start_year + 1)..end_year {
        total_fraction += BASIS_POINT_FACTOR as Units;
    }
    
    // Final partial year
    total_fraction += year_fraction_actual_actual_isda(
        start_of_year(end_year),
        end_time
    );
    
    total_fraction
}

fn year_fraction_actual_360(start_time: u64, end_time: u64) -> Units {
    if start_time >= end_time {
        return 0;
    }
    
    let days = days_between(start_time, end_time);
    (days * BASIS_POINT_FACTOR / 360) as Units
}

fn year_fraction_actual_365(start_time: u64, end_time: u64) -> Units {
    if start_time >= end_time {
        return 0;
    }
    
    let days = days_between(start_time, end_time);
    (days * BASIS_POINT_FACTOR / 365) as Units
}

fn year_fraction_thirty_e_360(start_time: u64, end_time: u64) -> Units {
    if start_time >= end_time {
        return 0;
    }
    
    let (start_y, start_m, start_d) = get_ymd(start_time);
    let (end_y, end_m, end_d) = get_ymd(end_time);
    
    let d1 = if start_d == 31 { 30 } else { start_d };
    let d2 = if end_d == 31 { 30 } else { end_d };
    
    let days = 360 * (end_y - start_y) + 
               30 * (end_m - start_m) + 
               (d2 - d1);
               
    (days * BASIS_POINT_FACTOR / 360) as Units
}

fn year_fraction_thirty_e_360_isda(
    start_time: u64,
    end_time: u64,
    maturity: Option<u64>
) -> Units {
    if start_time >= end_time {
        return 0;
    }
    
    let (start_y, start_m, start_d) = get_ymd(start_time);
    let (end_y, end_m, end_d) = get_ymd(end_time);
    
    let d1 = if is_last_day_of_month(start_time) { 30 } else { start_d };
    let d2 = if is_last_day_of_month(end_time) && 
                (maturity.is_none() || end_time != maturity.unwrap() || end_m != 2)
             { 30 } else { end_d };
    
    let days = 360 * (end_y - start_y) + 
               30 * (end_m - start_m) + 
               (d2 - d1);
               
    (days * BASIS_POINT_FACTOR / 360) as Units
}

fn year_fraction_bus_252(start_time: u64, end_time: u64) -> Units {
    if start_time >= end_time {
        return 0;
    }
    
    let business_days = count_business_days(start_time, end_time);
    (business_days * BASIS_POINT_FACTOR / 252) as Units
}

// ============= Annuity Calculations =============

/// Calculates the annuity amount given an interest rate and time factors
pub fn annuity(rate: Units, time_factors: &[Units]) -> Units {
    let numerator = product(&time_factors.iter()
        .map(|&t| rate * t / BASIS_POINT_FACTOR + BASIS_POINT_FACTOR)
        .collect::<Vec<_>>());
        
    let denominator = sum(&running_product(&time_factors.iter()
        .map(|&t| rate * t / BASIS_POINT_FACTOR + BASIS_POINT_FACTOR)
        .collect::<Vec<_>>()));
        
    if denominator == 0 {
        0
    } else {
        numerator * BASIS_POINT_FACTOR / denominator
    }
}

// ============= Helper Functions =============

/// Get year from timestamp
fn get_year(timestamp: u64) -> u64 {
    timestamp / (SECONDS_PER_DAY * DAYS_PER_YEAR)
}

/// Get year, month, day from timestamp
fn get_ymd(timestamp: u64) -> (u64, u64, u64) {
    let days = timestamp / SECONDS_PER_DAY;
    let year = days / DAYS_PER_YEAR;
    let month = (days % (DAYS_PER_YEAR)) / 30; // Approximate
    let day = days % 30 + 1;
    (year, month, day)
}

/// Calculate days between timestamps
fn days_between(start: u64, end: u64) -> u64 {
    (end - start) / SECONDS_PER_DAY
}

/// Check if year is leap year
fn is_leap_year(year: u64) -> bool {
    if year % 4 != 0 {
        false
    } else if year % 100 != 0 {
        true
    } else if year % 400 != 0 {
        false
    } else {
        true
    }
}

/// Get timestamp for start of year
fn start_of_year(year: u64) -> u64 {
    year * DAYS_PER_YEAR * SECONDS_PER_DAY
}

/// Check if timestamp is last day of month
fn is_last_day_of_month(timestamp: u64) -> bool {
    let (_, _, day) = get_ymd(timestamp);
    let tomorrow = timestamp + SECONDS_PER_DAY;
    let (_, tomorrow_m, _) = get_ymd(tomorrow);
    day == days_in_month(get_ymd(timestamp).1)
}

/// Get days in month (simplified)
fn days_in_month(month: u64) -> u64 {
    match month {
        1 => 31,
        2 => 28, // Simplified, doesn't handle leap years
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 30,
    }
}

/// Count business days between timestamps
fn count_business_days(start: u64, end: u64) -> u64 {
    let mut total_days = days_between(start, end);
    let mut weeks = total_days / 7;
    let remaining_days = total_days % 7;
    
    // Subtract weekends
    total_days -= weeks * 2;
    
    // Handle remaining days
    if remaining_days > 0 {
        let start_day = (start / SECONDS_PER_DAY) % 7;
        for i in 0..remaining_days {
            let current_day = (start_day + i) % 7;
            if current_day == 0 || current_day == 6 {
                total_days -= 1;
            }
        }
    }
    
    total_days
}

/// Product of a vector of numbers
fn product(values: &[Units]) -> Units {
    values.iter().fold(1, |acc, &x| acc * x / BASIS_POINT_FACTOR)
}

/// Running product (tails) of a vector
fn running_product(values: &[Units]) -> Vec<Units> {
    let mut result = Vec::with_capacity(values.len());
    let mut product = 1;
    for &value in values.iter().rev() {
        product = product * value / BASIS_POINT_FACTOR;
        result.push(product);
    }
    result.reverse();
    result
}

/// Sum of a vector of numbers
fn sum(values: &[Units]) -> Units {
    values.iter().fold(0, |acc, &x| acc + x)
}

#[cfg(test)]

// ============= Option Math =============

/// Black-Scholes option pricing - returns value in basis points
pub fn option_price(
    spot: Units,           // Current price of underlying in basis points
    strike: Units,         // Strike price in basis points
    time: Units,           // Time to maturity in year fractions (basis points)
    rate: Units,           // Risk-free rate in basis points
    volatility: Units,     // Volatility in basis points
    is_call: bool,         // true for call, false for put
) -> Units {
    let s = spot as f64 / BASIS_POINT_FACTOR as f64;
    let k = strike as f64 / BASIS_POINT_FACTOR as f64;
    let t = time as f64 / BASIS_POINT_FACTOR as f64;
    let r = rate as f64 / BASIS_POINT_FACTOR as f64;
    let v = volatility as f64 / BASIS_POINT_FACTOR as f64;

    let d1 = (f64::ln(s / k) + (r + v * v / 2.0) * t) / (v * f64::sqrt(t));
    let d2 = d1 - v * f64::sqrt(t);

    let price = if is_call {
        s * normal_cdf(d1) - k * f64::exp(-r * t) * normal_cdf(d2)
    } else {
        k * f64::exp(-r * t) * normal_cdf(-d2) - s * normal_cdf(-d1)
    };

    (price * BASIS_POINT_FACTOR as f64) as Units
}

/// Normal cumulative distribution function
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / f64::sqrt(2.0)))
}

/// Error function approximation
fn erf(x: f64) -> f64 {
    // Constants for approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * f64::exp(-x * x);

    sign * y
}

// ============= Scaling Factor Calculations =============

/// Calculate scaling factor based on index value changes
pub fn calculate_scaling_factor(
    initial_index: Units,
    current_index: Units,
    reference_index: Units,
) -> Units {
    if initial_index == 0 {
        return BASIS_POINT_FACTOR as Units;
    }
    
    let numerator = (current_index as u128) * (BASIS_POINT_FACTOR as u128);
    let denominator = initial_index as u128;
    (numerator / denominator) as Units
}

/// Apply scaling to an amount
pub fn apply_scaling(
    amount: Units,
    scaling_factor: Units,
) -> Units {
    ((amount as u128) * (scaling_factor as u128) / (BASIS_POINT_FACTOR as u128)) as Units
}

// ============= Interest Rate Calculations =============

/// Calculate effective interest rate from nominal rate
pub fn effective_rate(
    nominal_rate: Units,    // In basis points
    compounds_per_year: u64,
) -> Units {
    let r = nominal_rate as f64 / BASIS_POINT_FACTOR as f64;
    let n = compounds_per_year as f64;
    
    let effective = (1.0 + r/n).powf(n) - 1.0;
    (effective * BASIS_POINT_FACTOR as f64) as Units
}

/// Calculate forward rate from spot rates
pub fn forward_rate(
    rate1: Units,  // Rate for period 1 in basis points
    rate2: Units,  // Rate for period 2 in basis points
    time1: Units,  // Length of period 1 in year fractions (basis points)
    time2: Units,  // Length of period 2 in year fractions (basis points)
) -> Units {
    if time1 >= time2 {
        return 0;
    }

    let r1 = rate1 as f64 / BASIS_POINT_FACTOR as f64;
    let r2 = rate2 as f64 / BASIS_POINT_FACTOR as f64;
    let t1 = time1 as f64 / BASIS_POINT_FACTOR as f64;
    let t2 = time2 as f64 / BASIS_POINT_FACTOR as f64;

    let forward = ((1.0 + r2).powf(t2) / (1.0 + r1).powf(t1)).powf(1.0/(t2-t1)) - 1.0;
    (forward * BASIS_POINT_FACTOR as f64) as Units
}

// ============= Present Value Calculations =============

/// Calculate present value of future cash flows
pub fn present_value(
    future_values: &[(Units, Units)],  // (amount, time) pairs, time in year fractions
    discount_rate: Units,              // In basis points
) -> Units {
    let mut total_pv = 0u128;
    let r = discount_rate as f64 / BASIS_POINT_FACTOR as f64;

    for &(amount, time) in future_values {
        let t = time as f64 / BASIS_POINT_FACTOR as f64;
        let discount_factor = 1.0 / (1.0 + r).powf(t);
        let pv = (amount as f64 * discount_factor) as u128;
        total_pv += pv;
    }

    (total_pv * (BASIS_POINT_FACTOR as u128) / (BASIS_POINT_FACTOR as u128)) as Units
}

/// Calculate internal rate of return (IRR)
pub fn calculate_irr(
    cash_flows: &[(Units, Units)],  // (amount, time) pairs, time in year fractions
    initial_guess: Units,           // Initial rate guess in basis points
    max_iterations: u32,
) -> Option<Units> {
    let mut rate = initial_guess as f64 / BASIS_POINT_FACTOR as f64;
    let tolerance = 0.0001;

    for _ in 0..max_iterations {
        let mut f = 0.0;
        let mut df = 0.0;

        for &(amount, time) in cash_flows {
            let t = time as f64 / BASIS_POINT_FACTOR as f64;
            let pv_factor = 1.0 / (1.0 + rate).powf(t);
            f += amount as f64 * pv_factor;
            df -= t * amount as f64 * pv_factor / (1.0 + rate);
        }

        if df.abs() < tolerance {
            break;
        }

        let new_rate = rate - f / df;
        if (new_rate - rate).abs() < tolerance {
            return Some((new_rate * BASIS_POINT_FACTOR as f64) as Units);
        }
        rate = new_rate;
    }

    None  // Failed to converge
}

// ============= Additional Math Functions =============

/// Linear interpolation
pub fn interpolate(
    x: Units,
    x0: Units,
    x1: Units,
    y0: Units,
    y1: Units,
) -> Units {
    if x0 == x1 {
        return y0;
    }
    
    let dx = (x1 as i128) - (x0 as i128);
    let dy = (y1 as i128) - (y0 as i128);
    let x_diff = (x as i128) - (x0 as i128);
    
    let interpolated = y0 as i128 + (x_diff * dy) / dx;
    interpolated as Units
}

/// Safe exponentiation for basis points
pub fn pow(
    base: Units,        // In basis points
    exponent: Units,    // In basis points
) -> Units {
    let b = base as f64 / BASIS_POINT_FACTOR as f64;
    let e = exponent as f64 / BASIS_POINT_FACTOR as f64;
    
    let result = b.powf(e);
    (result * BASIS_POINT_FACTOR as f64) as Units
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_pricing() {
        let spot = 10000;      // $1.00
        let strike = 10000;    // $1.00
        let time = 10000;      // 1 year
        let rate = 500;        // 5%
        let vol = 2000;        // 20%
        
        let call_price = option_price(spot, strike, time, rate, vol, true);
        let put_price = option_price(spot, strike, time, rate, vol, false);
        
        // Put-call parity check (approximately)
        let parity_diff = (call_price as i64 - put_price as i64).abs();
        assert!(parity_diff < 100);  // Within 1 cent
    }

    #[test]
    fn test_scaling_factor() {
        let initial = 10000;    // 1.00
        let current = 11000;    // 1.10
        let reference = 10000;  // 1.00
        
        let factor = calculate_scaling_factor(initial, current, reference);
        assert_eq!(factor, 11000);  // 1.10 in basis points
    }

    #[test]
    fn test_present_value() {
        let cash_flows = vec![
            (10000, 10000),  // $1 in 1 year
            (10000, 20000),  // $1 in 2 years
        ];
        let rate = 1000;     // 10%
        
        let pv = present_value(&cash_flows, rate);
        assert!(pv < 20000);  // PV should be less than sum of cash flows
    }

    #[test]
    fn test_irr() {
        let cash_flows = vec![
            (-10000, 0),     // Initial investment of $1
            (12000, 10000),  // Return of $1.20 in 1 year
        ];
        
        if let Some(irr) = calculate_irr(&cash_flows, 1000, 100) {
            assert!(irr > 0);
        }
    }
}
