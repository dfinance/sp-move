use core::convert::TryFrom;
use core::convert::TryInto;
use move_vm::io::traits::{Balance as VmBalance, BalanceAccess};

use crate::addr::address_to_account;
use frame_support::traits::Currency;
use frame_support::traits::Imbalance;
use frame_support::traits::WithdrawReasons;
use frame_support::traits::ExistenceRequirement;

type BalanceOf<T> = <T as balances::Config>::Balance;

pub const PONT: Ticker = Ticker::new("PONT");
/// Suppoted tickers.
pub static TICKERS: &[Ticker] = &[PONT];

pub fn is_ticker_supported(ticker: Ticker) -> bool {
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

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Ticker<'a>(&'a [u8]);

impl<'a> std::fmt::Display for Ticker<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::str::from_utf8(self.0).expect("Could not read as utf-8"))
            .expect("Could not write into formatter");
        Ok(())
    }
}

impl<'a> Into<&'a [u8]> for Ticker<'a> {
    fn into(self) -> &'a [u8] {
        self.0
    }
}

impl<'a> From<&'a str> for Ticker<'a> {
    fn from(f: &str) -> Ticker<'_> {
        Ticker(f.as_bytes())
    }
}

impl Ticker<'_> {
    pub const fn new(ticker: &str) -> Ticker {
        Ticker(ticker.as_bytes())
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
        ticker: &[u8],
    ) -> Option<VmBalance> {
        let ticker = Ticker(ticker);

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
                <balances::Module<T> as Currency<T::AccountId>>::free_balance(&address)
                    .try_into()
                    .map_err(|_err| error!("Convert native balance to VM balance type."))
            })
            .ok()
    }

    fn add(
        &self,
        address: &move_core_types::account_address::AccountAddress,
        ticker: &[u8],
        amount: VmBalance,
    ) {
        let ticker = Ticker(ticker);

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
                        <balances::Module<T> as Currency<T::AccountId>>::withdraw(
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

    fn sub(
        &self,
        address: &move_core_types::account_address::AccountAddress,
        ticker: &[u8],
        amount: VmBalance,
    ) {
        let ticker = Ticker(ticker);

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
                        <balances::Module<T> as Currency<T::AccountId>>::deposit_creating(
                            &address, amount,
                        )
                    })
            })
            .map(|imbalance| imbalance.peek())
            // TODO: return result
            .ok();
        trace!("native balance deposit imbalance: {:?}", imbalance);
    }

    fn get_currency_info(
        &self,
        path: &move_vm::io::traits::CurrencyAccessPath,
    ) -> Option<move_vm::io::balance::CurrencyInfo> {
        todo!()
    }
}

#[cfg(not(feature = "no-vm-static"))]
pub mod boxed {
    use move_vm::io::{
        balance::CurrencyInfo,
        traits::{Balance as VmBalance, BalanceAccess},
    };
    use sp_std::prelude::*;
    use move_core_types::account_address::AccountAddress;

    pub type BalancesAdapter = BalancesBoxedAdapter;

    /// Vm storage boxed adapter for native storage
    pub struct BalancesBoxedAdapter {
        f_get: Box<dyn Fn(&AccountAddress, &[u8]) -> Option<VmBalance>>,
        f_deposit: Box<dyn Fn(&AccountAddress, &[u8], VmBalance)>,
        f_withdraw: Box<dyn Fn(&AccountAddress, &[u8], VmBalance)>,
    }

    impl<T: balances::Config> From<super::BalancesAdapter<T>> for BalancesBoxedAdapter {
        fn from(adapter: super::BalancesAdapter<T>) -> Self {
            Self {
                f_get: Box::new(move |address, ticker| adapter.get_balance(address, ticker)),
                f_deposit: Box::new(|address, ticker, amount| {
                    let adapter = super::BalancesAdapter::<T>::new();
                    adapter.add(address, ticker, amount)
                }),
                f_withdraw: Box::new(|address, ticker, amount| {
                    let adapter = super::BalancesAdapter::<T>::new();
                    adapter.sub(address, ticker, amount)
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
                f_deposit: Box::new(move |addr, id, val| balances.add(addr, id, val)),
                f_withdraw: Box::new(move |addr, id, val| balances.sub(addr, id, val)),
            }
        }
    }

    impl BalanceAccess for BalancesBoxedAdapter {
        fn get_balance(&self, address: &AccountAddress, ticker: &[u8]) -> Option<VmBalance> {
            (self.f_get)(address, ticker)
        }

        fn add(&self, address: &AccountAddress, ticker: &[u8], amount: VmBalance) {
            (self.f_deposit)(address, ticker, amount)
        }

        fn sub(&self, address: &AccountAddress, ticker: &[u8], amount: VmBalance) {
            (self.f_withdraw)(address, ticker, amount)
        }

        fn get_currency_info(
            &self,
            path: &move_vm::io::traits::CurrencyAccessPath,
        ) -> Option<move_vm::io::balance::CurrencyInfo> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PONT;

    #[test]
    fn is_ticker_supported() {
        assert!(!super::is_ticker_supported(super::Ticker::new(
            "NOT_SUPPORTED"
        )));
        assert!(super::is_ticker_supported(PONT));
        assert!(super::is_ticker_supported(super::Ticker::new("PONT")));
        assert!(super::is_ticker_supported("PONT".into()));
    }
}
