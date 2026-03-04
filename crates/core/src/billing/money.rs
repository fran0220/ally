use rust_decimal::{Decimal, prelude::FromPrimitive, prelude::ToPrimitive};

pub const MONEY_SCALE: u32 = 6;

pub fn normalize_money(value: Decimal) -> Decimal {
    value.round_dp(MONEY_SCALE)
}

pub fn decimal_from_f64(value: f64) -> Option<Decimal> {
    if !value.is_finite() {
        return None;
    }
    Decimal::from_f64(value).map(normalize_money)
}

pub fn decimal_to_f64(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}
