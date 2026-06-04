use crate::BankErrors;

pub fn convert_assets_to_shares(assets_amount: u64, total_shares: u64, total_assets_amount: u64) -> u64 {
    let result;
    if total_assets_amount == 0 {
        result = assets_amount;
    } else {
        // result = assets_amount * total_shares/ total_assets_amount
        let assets_amount_u128 = u128::from(assets_amount);
        let total_shares_u128 = u128::from(total_shares);
        let total_assets_amount_u128 = u128::from(total_assets_amount);

        let result_u128 = assets_amount_u128 * total_shares_u128 / total_assets_amount_u128;
        result = result_u128.try_into().map_err(|_| BankErrors::Overflow).unwrap();
    }
    result
}