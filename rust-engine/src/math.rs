use ruint::Uint;
use std::cmp::{min, max};

pub type U256 = Uint<256, 4>;
pub type U160 = Uint<160, 3>;

pub fn q96() -> U256 { Uint::from_limbs([0x1000000000000000000000000, 0, 0, 0]) }
pub fn q128() -> U256 { Uint::from_limbs([0x100000000000000000000000000000000, 0, 0, 0]) }
pub fn min_sqrt_ratio() -> U160 { Uint::from_limbs([4295128740, 0, 0]) }
pub fn max_sqrt_ratio() -> U160 { Uint::from_limbs([0xfff, 0, 0]) }

pub const TICK_BASE: f64 = 1.0001f64;

#[derive(Clone, Debug)]
pub struct TickMath;

impl TickMath {
    pub fn get_sqrt_ratio_at_tick(tick: i32) -> U160 {
        let abs_tick = if tick < 0 { -tick as i64 } else { tick as i64 };
        let mut ratio: U256 = if (abs_tick & 0x1) != 0 {
            Uint::from(0xfffcb933bd6fad37aa2d162d1a594001)
        } else {
            Uint::from(0x100000000000000000000000000000000)
        };

        if (abs_tick & 0x2) != 0 { ratio = (ratio * Uint::from(0xfff97272373d413259a46990580e213a)) >> 128; }
        if (abs_tick & 0x4) != 0 { ratio = (ratio * Uint::from(0xfff2e50f5f656932ef12357cf3c214fd)) >> 128; }
        if (abs_tick & 0x8) != 0 { ratio = (ratio * Uint::from(0xffe5caca7e10e4e61c3624eaa0941cd0)) >> 128; }
        if (abs_tick & 0x10) != 0 { ratio = (ratio * Uint::from(0xffcb9843d60f6159c9db58835c926644)) >> 128; }
        if (abs_tick & 0x20) != 0 { ratio = (ratio * Uint::from(0xff973b41fa98c081472e6896dfb254c0)) >> 128; }
        if (abs_tick & 0x40) != 0 { ratio = (ratio * Uint::from(0xff2ea16466c96a3843ec78b326b52861)) >> 128; }
        if (abs_tick & 0x80) != 0 { ratio = (ratio * Uint::from(0xfe5dee046a99a2a811c461f1969c3053)) >> 128; }
        if (abs_tick & 0x100) != 0 { ratio = (ratio * Uint::from(0xfcbe86c7900a88aedcffc83b479aa3a4)) >> 128; }
        if (abs_tick & 0x200) != 0 { ratio = (ratio * Uint::from(0xf987a7253ac413176f2b074cf7815e54)) >> 128; }
        if (abs_tick & 0x400) != 0 { ratio = (ratio * Uint::from(0xf3392b0822b70005940c7a398e4b70f3)) >> 128; }
        if (abs_tick & 0x800) != 0 { ratio = (ratio * Uint::from(0xe7159475a2c29b7443b29c7fa6e889d9)) >> 128; }
        if (abs_tick & 0x1000) != 0 { ratio = (ratio * Uint::from(0xd097f3bdfd2022b8845ad8f792aa5825)) >> 128; }
        if (abs_tick & 0x2000) != 0 { ratio = (ratio * Uint::from(0xa9f746462d870fdf8a65dc1f90e061e5)) >> 128; }
        if (abs_tick & 0x4000) != 0 { ratio = (ratio * Uint::from(0x70d869a156d2a1b890bb3df62baf32f7)) >> 128; }
        if (abs_tick & 0x8000) != 0 { ratio = (ratio * Uint::from(0x4be2d5f997265465c6097823b51b70)) >> 128; }
        if (abs_tick & 0x10000) != 0 { ratio = (ratio * Uint::from(0x3186727a04ed1b3eb0a43d74be14b)) >> 128; }
        if (abs_tick & 0x20000) != 0 { ratio = (ratio * Uint::from(0x1405134c1e0ebb346d7c8f4b7cd6)) >> 128; }

        if tick >= 0 {
            ratio = ratio.wrapping_add(q128() - Uint::from(1u64)) / q128();
        } else {
            ratio = q128() / ratio.wrapping_add(Uint::from(1u64));
        }

        U160::try_from(ratio).unwrap_or_else(|_| U160::ZERO)
    }

    pub fn get_tick_at_sqrt_ratio(sqrt_price_x96: U160) -> i32 {
        let sqrt_price = U256::from(sqrt_price_x96);
        let ratio = sqrt_price * sqrt_price;
        let mut msb = 0;
        let mut shifted = ratio;

        for i in 0..256 {
            if shifted.bit(255 - i) {
                msb = 255 - i;
                break;
            }
        }

        let log2 = Uint::<256, 4>::from(msb) << 64;
        let mut r = if shifted >> msb != Uint::ZERO {
            let r = shifted >> msb;
            r
        } else {
            Uint::from(0u64)
        };

        let mut log2_value = log2;
        macro_rules! iter_log2 {
            ($bit:literal, $val:expr) => {
                if r >= $val {
                    r = (r * Uint::from(10u64.pow($bit))) >> ($bit + 1);
                    log2_value = log2_value | (Uint::from(1u64) << ($bit + 64));
                }
            };
        }

        iter_log2!(63, Uint::from(0x8000000000000000u64));
        // Simplified approximation
        let tick = ((log2_value.as_limbs()[0] as f64) * (100.0f64.ln() / 2.0f64.ln()) * TICK_BASE) as i32;
        tick
    }
}

pub struct LiquidityMath;

impl LiquidityMath {
    pub fn get_amount0_delta(
        sqrt_ratio_ax96: U160,
        sqrt_ratio_bx96: U160,
        liquidity: u128,
    ) -> u128 {
        let (lower, upper) = if sqrt_ratio_ax96 > sqrt_ratio_bx96 {
            (sqrt_ratio_bx96, sqrt_ratio_ax96)
        } else {
            (sqrt_ratio_ax96, sqrt_ratio_bx96)
        };

        let numerator = U256::from(liquidity) * U256::from(upper - lower);
        let denominator = U256::from(upper) * U256::from(lower);

        if denominator == U256::ZERO {
            return u128::MAX;
        }

        let result = numerator / (denominator >> 96);
        u128::try_from(result).unwrap_or(u128::MAX)
    }

    pub fn get_amount1_delta(
        sqrt_ratio_ax96: U160,
        sqrt_ratio_bx96: U160,
        liquidity: u128,
    ) -> u128 {
        let (lower, upper) = if sqrt_ratio_ax96 > sqrt_ratio_bx96 {
            (sqrt_ratio_bx96, sqrt_ratio_ax96)
        } else {
            (sqrt_ratio_ax96, sqrt_ratio_bx96)
        };

        let numerator = U256::from(liquidity) * U256::from(upper - lower);
        let result = numerator >> 96;
        u128::try_from(result).unwrap_or(u128::MAX)
    }
}

pub fn calculate_sqrt_price_x96(token0_decimals: u8, token1_decimals: u8, price: f64) -> U160 {
    let adjusted = price * 10f64.powi(token1_decimals as i32 - token0_decimals as i32);
    let sqrt = adjusted.sqrt();
    let q96: f64 = 2u128.pow(96) as f64;
    U160::try_from((sqrt * q96) as u128).unwrap_or(U160::ZERO)
}

pub fn calculate_price_from_sqrt(sqrt_price_x96: U160, token0_decimals: u8, token1_decimals: u8) -> f64 {
    let price = (sqrt_price_x96.as_limbs()[0] as f64 / (2u128.pow(96) as f64)).powi(2);
    price * 10f64.powi(token0_decimals as i32 - token1_decimals as i32)
}

pub fn compute_swap_amount_out(
    sqrt_price_current: U160,
    sqrt_price_target: U160,
    liquidity: u128,
    zero_for_one: bool,
) -> u128 {
    if zero_for_one {
        LiquidityMath::get_amount1_delta(sqrt_price_target, sqrt_price_current, liquidity)
    } else {
        LiquidityMath::get_amount0_delta(sqrt_price_current, sqrt_price_target, liquidity)
    }
}

pub fn compute_profit(
    amount_in: u128,
    intermediate_amount: u128,
    final_amount: u128,
) -> i128 {
    let net = final_amount as i128 - amount_in as i128;
    net
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_to_sqrt_price() {
        let sqrt = TickMath::get_sqrt_ratio_at_tick(0);
        assert!(sqrt > U160::ZERO);
    }

    #[test]
    fn test_liquidity_math() {
        let sqrt_a = TickMath::get_sqrt_ratio_at_tick(-100);
        let sqrt_b = TickMath::get_sqrt_ratio_at_tick(100);
        let amount0 = LiquidityMath::get_amount0_delta(sqrt_a, sqrt_b, 1000000);
        let amount1 = LiquidityMath::get_amount1_delta(sqrt_a, sqrt_b, 1000000);
        assert!(amount0 > 0 || amount1 > 0);
    }
}
