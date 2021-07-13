use core::convert::TryFrom;
use core::convert::TryInto;
use move_vm::data::BalanceAccess;
use move_vm_types::natives::balance::Balance as VmBalance;

use crate::addr::address_to_account;
use frame_support::traits::Currency;
use frame_support::traits::Imbalance;
use frame_support::traits::WithdrawReasons;
use frame_support::traits::ExistenceRequirement;

type BalanceOf<T> = <T as balances::Config>::Balance;

pub const PONT: &str = "PONT";
/// Suppoted tickers.
pub static TICKERS: &[&str] = &[PONT];

pub fn is_ticker_supported(ticker: &str) -> bool {
    TICKERS.contains(&ticker)
}

pub struct BalancesAdapter<T>(core::marker::PhantomData<T>);

impl<T: balances::Config> Default for BalancesAdapter<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T: balances::Config> BalancesAdapter<T> {
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T: balances::Config> BalanceAccess for BalancesAdapter<T>
where
    <T as balances::Config>::Balance: TryFrom<VmBalance>,
    <T as balances::Config>::Balance: TryInto<VmBalance>,
{
    fn get_balance(
        &self,
        address: &move_core_types::account_address::AccountAddress,
        ticker: &str,
    ) -> Option<VmBalance> {
        if !is_ticker_supported(ticker) {
            trace!("native balance ticker '{}' not supported", ticker);
            return None;
        }

        trace!(
            "native balance requested for address: {} (ticker: {})",
            address,
            ticker
        );
        address_to_account::<T::AccountId>(&address)
            .map_err(|_| error!("Can't convert address from Move to Substrate."))
            .and_then(|address| {
                <balances::Pallet<T> as Currency<T::AccountId>>::free_balance(&address)
                    .try_into()
                    .map_err(|_err| error!("Convert native balance to VM balance type."))
            })
            .ok()
    }

    fn deposit(
        &self,
        address: &move_core_types::account_address::AccountAddress,
        ticker: &str,
        amount: VmBalance,
    ) {
        if !is_ticker_supported(ticker) {
            return trace!("native balance ticker '{}' not supported", ticker);
        }

        trace!("deposit resource {} requested, amount: {}", ticker, amount);
        let imbalance = address_to_account::<T::AccountId>(&address)
            .map_err(|_| error!("Can't convert address from Move to Substrate."))
            .and_then(|address| {
                amount
                    .try_into()
                    .map_err(|_err| error!("Can't convert VM balance to native balance type."))
                    .and_then(|amount: BalanceOf<T>| {
                        <balances::Pallet<T> as Currency<T::AccountId>>::withdraw(
                            &address,
                            amount,
                            WithdrawReasons::RESERVE,
                            ExistenceRequirement::AllowDeath,
                        )
                        .map_err(|_err| error!("Can't withdraw native balance."))
                    })
                    .map(|imbalance| imbalance.peek())
            })
            .ok();
        trace!("native balance withdraw imbalance: {:?}", imbalance);
    }

    fn withdraw(
        &self,
        address: &move_core_types::account_address::AccountAddress,
        ticker: &str,
        amount: VmBalance,
    ) {
        if !is_ticker_supported(ticker) {
            return trace!("native balance ticker '{}' not supported", ticker);
        }

        trace!("withdraw resource {} requested, amount: {}", ticker, amount);
        let imbalance = address_to_account::<T::AccountId>(&address)
            .map_err(|_err| error!("Can't convert address from Move to Substrate."))
            .and_then(|address| {
                amount
                    .try_into()
                    .map_err(|_err| error!("Can't convert VM balance to native balance type."))
                    .map(|amount: BalanceOf<T>| {
                        <balances::Pallet<T> as Currency<T::AccountId>>::deposit_creating(
                            &address, amount,
                        )
                    })
            })
            .map(|imbalance| imbalance.peek())
            // TODO: return result
            .ok();
        trace!("native balance deposit imbalance: {:?}", imbalance);
    }
}

#[cfg(not(feature = "no-vm-static"))]
pub mod boxed {
    use sp_std::prelude::*;
    use move_core_types::account_address::AccountAddress;
    use move_vm_types::natives::balance::Balance as VmBalance;
    use move_vm::data::BalanceAccess;

    pub type BalancesAdapter = BalancesBoxedAdapter;

    /// Vm storage boxed adapter for native storage
    pub struct BalancesBoxedAdapter {
        f_get: Box<dyn Fn(&AccountAddress, &str) -> Option<VmBalance>>,
        f_deposit: Box<dyn Fn(&AccountAddress, &str, VmBalance)>,
        f_withdraw: Box<dyn Fn(&AccountAddress, &str, VmBalance)>,
    }

    impl<T: balances::Config> From<super::BalancesAdapter<T>> for BalancesBoxedAdapter {
        fn from(adapter: super::BalancesAdapter<T>) -> Self {
            Self {
                f_get: Box::new(move |address, ticker| adapter.get_balance(address, ticker)),
                f_deposit: Box::new(|address, ticker, amount| {
                    let adapter = super::BalancesAdapter::<T>::new();
                    adapter.deposit(address, ticker, amount)
                }),
                f_withdraw: Box::new(|address, ticker, amount| {
                    let adapter = super::BalancesAdapter::<T>::new();
                    adapter.withdraw(address, ticker, amount)
                }),
            }
        }
    }

    impl<T: balances::Config + 'static> From<&'static super::BalancesAdapter<T>>
        for BalancesBoxedAdapter
    {
        fn from(balances: &'static super::BalancesAdapter<T>) -> Self {
            Self {
                f_get: Box::new(move |addr, id| balances.get_balance(addr, id)),
                f_deposit: Box::new(move |addr, id, val| balances.deposit(addr, id, val)),
                f_withdraw: Box::new(move |addr, id, val| balances.withdraw(addr, id, val)),
            }
        }
    }

    impl BalanceAccess for BalancesBoxedAdapter {
        fn get_balance(&self, address: &AccountAddress, ticker: &str) -> Option<VmBalance> {
            (self.f_get)(address, ticker)
        }

        fn deposit(&self, address: &AccountAddress, ticker: &str, amount: VmBalance) {
            (self.f_deposit)(address, ticker, amount)
        }

        fn withdraw(&self, address: &AccountAddress, ticker: &str, amount: VmBalance) {
            (self.f_withdraw)(address, ticker, amount)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PONT;

    #[test]
    fn is_ticker_supported() {
        assert!(!super::is_ticker_supported(&"NOT_SUPPORTED"));
        assert!(super::is_ticker_supported(PONT));
    }
}
