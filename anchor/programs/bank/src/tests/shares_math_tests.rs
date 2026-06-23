#[cfg(test)]
mod convert_assets_to_shares {
    use crate::{MIN_USDC_DEPOSIT, BankErrors, shares_math::convert_assets_to_shares};

    #[test]
    fn should_scale_linearly() {
        let init_total_assets = MIN_USDC_DEPOSIT;
        let init_total_shares = MIN_USDC_DEPOSIT;

        let shares_1 = convert_assets_to_shares(MIN_USDC_DEPOSIT, init_total_shares, init_total_shares, false);

        let shares_2 = convert_assets_to_shares(MIN_USDC_DEPOSIT * 2, init_total_shares + shares_1, init_total_assets + MIN_USDC_DEPOSIT, false);

        assert_eq!(shares_2, 2 * shares_1);
    }

    #[test]
    fn should_return_zero_shares_if_zero_assets() {
        let result = convert_assets_to_shares(0, MIN_USDC_DEPOSIT, MIN_USDC_DEPOSIT, false);

        assert_eq!(result, 0);
    }

    #[test]
    fn should_return_shares_amount_equal_to_assets_amount_if_total_assets_is_zero() {
        let result = convert_assets_to_shares(MIN_USDC_DEPOSIT, 0, 0, false);
        assert_eq!(result, MIN_USDC_DEPOSIT);
    }

     #[test]
    fn should_return_one_share_when_minimal_bank_state() {
        let result = convert_assets_to_shares(1, 1, 1, false);

        assert_eq!(result, 1);
    }

    #[test]
    fn should_return_shares_when_valid_input() {
        let assets_amount: u64 = 10_000;
        let total_assets_amount: u64 = MIN_USDC_DEPOSIT;
        let total_shares: u64 = MIN_USDC_DEPOSIT;

        let result = convert_assets_to_shares(assets_amount, total_shares, total_assets_amount, false);

        let expected_result_u128 = (total_shares as u128)
                                            .checked_mul(assets_amount as u128).unwrap()
                                            .checked_div(total_assets_amount as u128).unwrap();
        let expected_result: u64 = expected_result_u128.try_into().map_err(|_| BankErrors::Overflow).unwrap();

        assert_eq!(result, expected_result);
    }
}

#[cfg(test)]
mod convert_shares_to_assets {
    use crate::{BankErrors, MIN_USDC_DEPOSIT, convert_shares_to_assets};

    #[test]
    fn should_return_zero_assets_if_total_shares_is_zero() {
        let result = convert_shares_to_assets(MIN_USDC_DEPOSIT, 0, MIN_USDC_DEPOSIT);
        assert_eq!(result, 0);
    }

    #[test]
    fn should_return_assets_when_valid_input() {
        let user_shares = 10_000;
        let total_shares = MIN_USDC_DEPOSIT;
        let total_assets = MIN_USDC_DEPOSIT;
        let result = convert_shares_to_assets(user_shares, total_shares, total_assets);

        let expected_result_u128 = (total_assets as u128)
                                            .checked_mul(user_shares as u128).unwrap()
                                            .checked_div(total_shares as u128).unwrap();
        let expected_result: u64 = expected_result_u128.try_into().map_err(|_| BankErrors::Overflow).unwrap();
        assert_eq!(result, expected_result);
    }
}