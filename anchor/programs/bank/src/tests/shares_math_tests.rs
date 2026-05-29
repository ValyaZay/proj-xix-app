#[cfg(test)]
mod convert_assets_to_shares {
    use crate::shares_math::convert_assets_to_shares;

    #[test]
    #[should_panic(expected = "DivisionByZero")]
    fn should_panic_if_total_assets_is_zero() {
        let assets_amount = 10_000;
        let total_shares = 20_000;
        let total_assets_amount = 0;

        convert_assets_to_shares(assets_amount, total_shares, total_assets_amount);
    }
}