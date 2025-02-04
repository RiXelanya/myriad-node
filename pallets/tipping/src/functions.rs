use crate::*;

use frame_support::{
	dispatch::DispatchError,
	sp_runtime::traits::{AccountIdConversion, SaturatedConversion, Saturating, Zero},
	traits::{fungibles, Currency, ExistenceRequirement, Get},
	PalletId,
};
use sp_std::vec::*;

const PALLET_ID: PalletId = PalletId(*b"Tipping!");

impl<T: Config> Pallet<T> {
	/// The account ID that holds tipping's funds
	pub fn tipping_account_id() -> T::AccountId {
		PALLET_ID.into_account_truncating()
	}

	pub fn can_update_balance(key: &TipsBalanceKeyOf<T>) -> bool {
		TipsBalanceByReference::<T>::contains_key(key)
	}

	/*pub fn can_pay_content(
			ft_identifier: &[u8],
			sender: &T::AccountId,
			amount: &BalanceOf<T>,
		) -> Result<FeeDetail<BalanceOf<T>>, Error<T>> {
			let tx_fee_denom = 100u8
				.checked_div(T::TransactionFee::get())
				.filter(|value| value <= &100u8)
				.ok_or(Error::<T>::InsufficientFee)?;

			let fee: BalanceOf<T> = *amount / tx_fee_denom.saturated_into();
			let total_transfer = *amount + fee;

			let transferable_balance: BalanceOf<T> = if ft_identifier == b"native" {
				let minimum_balance = CurrencyOf::<T>::minimum_balance();
				let account_balance = CurrencyOf::<T>::free_balance(sender);

				if account_balance >= minimum_balance {
					account_balance - minimum_balance
				} else {
					Zero::zero()
				}
			} else {
				let asset_id = Self::asset_id(ft_identifier)?;
				let asset_minimum_balance =
					<T::Assets as fungibles::Inspect<T::AccountId>>::minimum_balance(asset_id);
				let asset_account_balance =
					<T::Assets as fungibles::Inspect<T::AccountId>>::balance(asset_id, sender);
				let asset_transferable_balance = if asset_account_balance >= asset_minimum_balance {
					asset_account_balance - asset_minimum_balance
				} else {
					0u128
				};

				asset_transferable_balance.saturated_into()
			};

			if total_transfer > transferable_balance {
				return Err(Error::<T>::InsufficientBalance)
			}

			let admin_fee_denom = 100u8
				.checked_div(T::AdminFee::get())
				.filter(|value| value <= &100u8)
				.ok_or(Error::<T>::InsufficientFee)?;

			let admin_fee = fee / admin_fee_denom.saturated_into();
			let server_fee = fee - admin_fee;
			let fee_detail = FeeDetail::new(admin_fee, server_fee, fee);

			Ok(fee_detail)
		}

	*/
	pub fn can_pay_content(
		ft_identifier: &[u8],
		sender: &T::AccountId,
		amount: &BalanceOf<T>,
	) -> Result<FeeDetail<BalanceOf<T>>, Error<T>> {
		let transferable_balance: BalanceOf<T> = if ft_identifier == b"native" {
			let minimum_balance = CurrencyOf::<T>::minimum_balance();
			let account_balance = CurrencyOf::<T>::free_balance(sender);

			if account_balance >= minimum_balance {
				account_balance - minimum_balance
			} else {
				Zero::zero()
			}
		} else {
			let asset_id = Self::asset_id(ft_identifier)?;
			let asset_minimum_balance =
				<T::Assets as fungibles::Inspect<T::AccountId>>::minimum_balance(asset_id);
			let asset_account_balance =
				<T::Assets as fungibles::Inspect<T::AccountId>>::balance(asset_id, sender);
			let asset_transferable_balance = if asset_account_balance >= asset_minimum_balance {
				asset_account_balance - asset_minimum_balance
			} else {
				0u128
			};

			asset_transferable_balance.saturated_into()
		};

		if *amount > transferable_balance {
			return Err(Error::<T>::InsufficientBalance)
		}

		let tx_fee_denom = 100u8
			.checked_div(T::TransactionFee::get())
			.filter(|value| value <= &100u8)
			.ok_or(Error::<T>::InsufficientFee)?;

		let fee: BalanceOf<T> = *amount / tx_fee_denom.saturated_into();
		if fee > *amount {
			return Err(Error::<T>::InsufficientBalance)
		}
		let admin_fee_denom = 100u8
			.checked_div(T::AdminFee::get())
			.filter(|value| value <= &100u8)
			.ok_or(Error::<T>::InsufficientFee)?;

		let admin_fee = fee / admin_fee_denom.saturated_into();
		let server_fee = fee - admin_fee;
		let fee_detail = FeeDetail::new(admin_fee, server_fee, fee);

		Ok(fee_detail)
	}

	pub fn can_pay_fee(key: &TipsBalanceKeyOf<T>, tx_fee: &BalanceOf<T>) -> Result<(), Error<T>> {
		if tx_fee == &Zero::zero() {
			return Err(Error::<T>::InsufficientBalance)
		}

		let tips_balance = Self::tips_balance_by_reference(key).ok_or(Error::<T>::NotExists)?;
		let amount = tips_balance.get_amount();

		if amount == &Zero::zero() {
			return Err(Error::<T>::InsufficientBalance)
		}

		if amount < tx_fee {
			return Err(Error::<T>::InsufficientBalance)
		}

		Ok(())
	}

	pub fn can_claim_tip(
		key: &TipsBalanceKeyOf<T>,
		receiver: &AccountIdOf<T>,
	) -> Option<TipsBalanceOf<T>> {
		if let Some(tips_balance) = Self::tips_balance_by_reference(key) {
			if tips_balance.get_amount() == &Zero::zero() {
				return None
			}

			if tips_balance.get_account_id().is_none() {
				return None
			}

			if tips_balance.get_account_id().as_ref().unwrap() != receiver {
				return None
			}

			return Some(tips_balance)
		}

		None
	}

	pub fn do_update_withdrawal_balance(ft_identifier: &[u8], balance: BalanceOf<T>) {
		WithdrawalBalance::<T>::mutate(ft_identifier, |value| {
			*value += balance;
		});
	}

	pub fn do_update_reward_balance(
		instance_id: u64,
		tips_balance_info: &TipsBalanceInfoOf<T>,
		balance: BalanceOf<T>,
	) {
		let server_id = tips_balance_info.get_server_id();
		let ft_identifier = tips_balance_info.get_ft_identifier();
		RewardBalance::<T>::mutate((server_id, instance_id, ft_identifier), |value| {
			*value += balance;
		});
	}

	pub fn do_store_message(
		msg : &mut T::Hash,
		tips_balance: &TipsBalanceOf<T>,
	) {
		let key = tips_balance.key();
		if MessagedTips::<T>::contains_key(&key) {
			MessagedTips::<T>::set(key, Some(*msg))
		}
		else {
			MessagedTips::<T>::insert(key, msg)
		}
	}

	pub fn do_store_tips_balance(
		tips_balance: &TipsBalanceOf<T>,
		set_empty: bool,
		tx_fee: Option<BalanceOf<T>>,
	) -> BalanceOf<T> {
		let key = tips_balance.key();
		let amount = *tips_balance.get_amount();
		let account_id = tips_balance.get_account_id();
		let ft_identifier = tips_balance.get_ft_identifier();

		//  Total tip that has been send and claim
		let mut total_tip: BalanceOf<T> = amount;

		if Self::can_update_balance(&key) {
			TipsBalanceByReference::<T>::mutate(key, |tips_balance| match tips_balance {
				Some(tips_balance) => {
					if set_empty {
						tips_balance.set_amount(Zero::zero()); // Set balance to zero
					} else if tx_fee.is_some() && ft_identifier == b"native" {
						// Reduce user balance by the tx fee
						// As user ask admin server to claim references
						let current_balance = *tips_balance.get_amount();
						let final_balance =
							current_balance.saturating_sub(tx_fee.unwrap()).saturating_add(amount);
						tips_balance.set_amount(final_balance);
						total_tip = final_balance;
					} else {
						// There is an increase in balance
						tips_balance.add_amount(amount);
					}

					// Claim tips balance by account_id
					if account_id.is_some() {
						tips_balance.set_account_id(account_id.as_ref().unwrap());
					}
				},
				None => (),
			});
		} else {
			TipsBalanceByReference::<T>::insert(key, tips_balance);
		}

		total_tip
	}


	pub fn do_transfer(
		ft_identifier: &[u8],
		sender: &AccountIdOf<T>,
		receiver: &AccountIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if ft_identifier == b"native" {
			CurrencyOf::<T>::transfer(sender, receiver, amount, ExistenceRequirement::KeepAlive)?;
		} else {
			let asset_id = Self::asset_id(ft_identifier)?;
			let _ = <T::Assets as fungibles::Transfer<T::AccountId>>::transfer(
				asset_id,
				sender,
				receiver,
				amount.saturated_into(),
				true,
			)?;
		}

		Ok(())
	}

	pub fn do_store_tips_balances(
		server_id: &AccountIdOf<T>,
		references: &References,
		account_references: &References,
		ft_identifiers: &[FtIdentifier],
		account_id: &AccountIdOf<T>,
		tx_fee: &BalanceOf<T>,
	) -> Vec<TipsBalanceOf<T>> {
		let mut account_tips_balances = Vec::<TipsBalanceOf<T>>::new();

		let account_reference_type = account_references.get_reference_type();
		let account_reference_id = &account_references.get_reference_ids()[0];

		for ft_identifier in ft_identifiers.iter() {
			let mut tip: BalanceOf<T> = Zero::zero();

			let reference_type = references.get_reference_type();
			let reference_ids = references.get_reference_ids();

			// Get balance for references
			// Store the balance to account reference balance
			for reference_id in reference_ids {
				let server_id = server_id.clone();
				let key = (server_id, reference_type, reference_id, ft_identifier);
				let tips_balance = TipsBalanceByReference::<T>::take(&key);

				if let Some(tips_balance) = tips_balance {
					let amount = tips_balance.get_amount();
					if *amount > Zero::zero() {
						tip = tip.saturating_add(*amount);
					}
				}
			}

			let account_tips_balance_info = TipsBalanceInfo::new(
				server_id,
				account_reference_type,
				account_reference_id,
				ft_identifier,
			);

			let mut account_tips_balance = TipsBalance::new(&account_tips_balance_info, &tip);

			account_tips_balance.set_account_id(account_id);

			let tips = Self::do_store_tips_balance(&account_tips_balance, false, Some(*tx_fee));

			account_tips_balance.set_amount(tips);
			account_tips_balances.push(account_tips_balance);
		}

		account_tips_balances
	}

	pub fn asset_id(ft_identifier: &[u8]) -> Result<u32, Error<T>> {
		let str_num =
			String::from_utf8(ft_identifier.to_vec()).map_err(|_| Error::<T>::WrongFormat)?;

		str_num.parse::<u32>().map_err(|_| Error::<T>::WrongFormat)
	}
}
