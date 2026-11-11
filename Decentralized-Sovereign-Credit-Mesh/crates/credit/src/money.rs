/// Money validation helper.
///
/// All monetary amounts flowing through the system are `f64` today (see
/// PROD_READINESS_FINDINGS #5); this validator rejects the dangerous values
/// — NaN, infinity and non-positive amounts — at every boundary so the
/// numeric weaknesses are contained until a fixed-point money type lands.
pub fn validate_amount(name: &str, amount: f64) -> Result<f64, String> {
    if !amount.is_finite() {
        return Err(format!("{name} must be a finite number, got {amount}"));
    }
    if amount <= 0.0 {
        return Err(format!("{name} must be positive, got {amount}"));
    }
    Ok(amount)
}
