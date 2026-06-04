#[cfg(test)]
mod convert_assets_to_shares {
    use crate::{MIN_USDC_DEPOSIT, BankErrors, shares_math::convert_assets_to_shares};

    #[test]
    fn should_scale_linearly() {
        let init_total_assets = MIN_USDC_DEPOSIT;
        let init_total_shares = MIN_USDC_DEPOSIT;

        let shares_1 = convert_assets_to_shares(MIN_USDC_DEPOSIT, init_total_shares, init_total_shares);

        let shares_2 = convert_assets_to_shares(MIN_USDC_DEPOSIT * 2, init_total_shares + shares_1, init_total_assets + MIN_USDC_DEPOSIT);

        assert_eq!(shares_2, 2 * shares_1);
    }

    #[test]
    fn should_return_zero_shares_if_zero_assets() {
        let result = convert_assets_to_shares(0, MIN_USDC_DEPOSIT, MIN_USDC_DEPOSIT);

        assert_eq!(result, 0);
    }

     #[test]
    fn should_return_one_share_when_minimal_bank_state() {
        let result = convert_assets_to_shares(1, 1, 1);

        assert_eq!(result, 1);
    }

    #[test]
    fn should_return_assets_when_valid_input() {
        let assets_amount: u64 = 10_000;
        let total_assets_amount: u64 = MIN_USDC_DEPOSIT;
        let total_shares: u64 = MIN_USDC_DEPOSIT;

        let result = convert_assets_to_shares(assets_amount, total_shares, total_assets_amount);

        let expected_result_u128 = (total_shares as u128)
                                            .checked_mul(assets_amount as u128).unwrap()
                                            .checked_div(total_assets_amount as u128).unwrap();
        let expected_result: u64 = expected_result_u128.try_into().map_err(|_| BankErrors::Overflow).unwrap();

        assert_eq!(result, expected_result);
    }
}