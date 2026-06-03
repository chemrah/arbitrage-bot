pub const MIN_SQRT_RATIO: u128 = 4295128740;
pub const MAX_SQRT_RATIO: u128 = 14614467034852101032872738522011788105851972130077903324132311037586334839200u128;
pub const TICK_BASE: f64 = 1.0001f64;

pub fn get_sqrt_price_x96(tick: i32) -> u128 {
    let ratio = TICK_BASE.powi(tick);
    let q96 = 2u128.pow(96) as f64;
    (ratio.sqrt() * q96) as u128
}

pub fn get_tick_at_price(sqrt_price_x96: u128) -> i32 {
    let q96 = 2u128.pow(96) as f64;
    let price = (sqrt_price_x96 as f64 / q96).powi(2);
    (price.log(TICK_BASE).round() as i32)
}

pub fn get_amount_0_delta(sqrt_a: u128, sqrt_b: u128, liquidity: u128) -> u128 {
    let (lower, upper) = if sqrt_a > sqrt_b { (sqrt_b, sqrt_a) } else { (sqrt_a, sqrt_b) };
    let diff = upper - lower;
    // amount0 = liquidity * diff / (upper * lower / 2^96)
    let numerator = (liquidity as u128).checked_mul(diff).unwrap_or(u128::MAX);
    let denominator = ((upper as u128).checked_mul(lower as u128).unwrap_or(1)) >> 96;
    if denominator == 0 { return u128::MAX; }
    numerator / denominator
}

pub fn get_amount_1_delta(sqrt_a: u128, sqrt_b: u128, liquidity: u128) -> u128 {
    let (lower, upper) = if sqrt_a > sqrt_b { (sqrt_b, sqrt_a) } else { (sqrt_a, sqrt_b) };
    let diff = upper - lower;
    ((liquidity as u128).checked_mul(diff).unwrap_or(u128::MAX)) >> 96
}

pub fn compute_swap_amount_out(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    zero_for_one: bool,
) -> u128 {
    if zero_for_one {
        get_amount_1_delta(sqrt_price_target, sqrt_price_current, liquidity)
    } else {
        get_amount_0_delta(sqrt_price_current, sqrt_price_target, liquidity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_price_roundtrip() {
        let tick = 50000;
        let price = get_sqrt_price_x96(tick);
        let back = get_tick_at_price(price);
        assert!((back - tick).abs() < 10, "tick mismatch: {} vs {}", back, tick);
    }
}
