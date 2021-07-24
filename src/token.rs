use crate::{Config, Balances, Event, Issuance, Error, imbalance::{PositiveImbalance, NegativeImbalance}, Pallet};
use sp_runtime::{traits::{Zero, CheckedAdd, CheckedSub, Saturating, Bounded}};
use frame_support::{traits::{Currency, Get, WithdrawReasons, ExistenceRequirement, SignedImbalance}, pallet_prelude::PhantomData, dispatch::{DispatchResult, DispatchError}};

pub struct Erc1155Token<T: Config, Inner: Get<T::TokenId>>(PhantomData<T>, PhantomData<Inner>);

// Does this have overhead?
impl<T: Config, Inner: Get<T::TokenId>> Erc1155Token<T, Inner> {
    pub fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<T: Config, Inner: Get<T::TokenId>> Get<T::TokenId> for Erc1155Token<T, Inner> {
    fn get() -> T::TokenId {
        return Inner::get()
    }
}

impl<T, I> Currency<T::AccountId> for Erc1155Token<T, I>
where
    T: Config,
    I: Get<T::TokenId>
{
    type Balance = T::Balance;
    type PositiveImbalance = PositiveImbalance<T>;
    type NegativeImbalance = NegativeImbalance<T>;

    fn total_balance(who: &T::AccountId) -> Self::Balance {
        <Balances<T>>::get(who, Self::get()).clone().unwrap_or(T::Balance::zero())
    }

    fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
        value >= <Balances<T>>::get(who, Self::get()).unwrap_or(T::Balance::zero())
    }

    fn total_issuance() -> Self::Balance {
        <Issuance<T>>::get(Self::get()).clone().unwrap_or(T::Balance::zero())
    }

    fn minimum_balance() -> Self::Balance {
        return 0u32.into();
    }

    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        if amount.is_zero() {
            return Self::PositiveImbalance::new(0u32.into(), Self::get());
        }

        let mut res = amount;
        <Issuance<T>>::mutate(Self::get(), |supply| {
            let sup = supply.unwrap_or(T::Balance::zero());
            *supply = Some(sup
                .checked_sub(&res)
                .unwrap_or_else(|| {
                    res = sup;
                    T::Balance::zero()
                }));
        });

        Self::PositiveImbalance::new(amount, Self::get())
    }

    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        if amount.is_zero() {
            return Self::NegativeImbalance::new(0u32.into(), Self::get());
        }

        let mut res = amount;
        <Issuance<T>>::mutate(Self::get(), |supply| {
            let sup = supply.unwrap_or(T::Balance::zero());
            *supply = Some(sup.checked_add(&res)
                .unwrap_or_else(|| {
                    let max = Self::Balance::max_value();
                    res = max - sup;
                    max
                }));
        });

        Self::NegativeImbalance::new(amount, Self::get())
    }

    fn free_balance(who: &T::AccountId) -> Self::Balance {
        Self::total_balance(who)
    }

    fn ensure_can_withdraw(
        who: &T::AccountId,
        _value: T::Balance,
        _: WithdrawReasons,
        new_balance: T::Balance
    ) -> DispatchResult {
        if Self::total_balance(who) < new_balance {
            return Err(Error::<T>::OutOfFunds.into());
        }

        Ok(())
    }

    fn transfer(
        from: &T::AccountId,
        to: &T::AccountId,
        value: Self::Balance,
        _existence_requirement: ExistenceRequirement
    ) -> DispatchResult {
        if value.is_zero() || from == to {
            return Ok(())
        }

        <Balances<T>>::try_mutate(from, Self::get(), |balance| -> Result<(), Error<T>> {
            *balance = Some(balance.map(|b| b.checked_sub(&value))
                .flatten()
                .ok_or(Error::<T>::OutOfFunds)?);
            <Balances<T>>::mutate(to, Self::get(), |balance_target| {
                // Should we consider checked add?
                *balance_target = Some(balance.unwrap().saturating_add(value));
            });

            Ok(())
        })?;

        <Pallet<T>>::deposit_event(Event::TransferSingle(Some(from.clone()), Some(to.clone()), Self::get(), value));
 
        Ok(())
    }

    fn slash(
        who: &T::AccountId,
        value: Self::Balance
    ) -> (Self::NegativeImbalance, Self::Balance) {
        let ret = |slashed, remaining| {
            <Pallet<T>>::deposit_event(Event::TransferSingle(Some(who.clone()), None, Self::get(), slashed));
            
            (NegativeImbalance::new(slashed, Self::get()), remaining)
        };

        if value.is_zero() {
            return ret(T::Balance::zero(), Self::Balance::zero());
        }

        if Self::total_balance(who).is_zero() {
            return ret(T::Balance::zero(), value);
        }

        <Balances<T>>::mutate(who, Self::get(), |balance| {
            // Unwrap safety: balance is only None when Self::total_balance == 0
            let balance: &mut Self::Balance = balance.as_mut().unwrap();
            let slashed: Self::Balance;
            let mut remaining = Self::Balance::zero();
            if *balance < value {
                slashed = *balance;
                *balance = Self::Balance::zero();
                remaining = value - *balance;
            } else {
                *balance = *balance - value;
                slashed = value;
            }

            ret(slashed, remaining)
        })
    }

    fn deposit_into_existing(
        who: &T::AccountId,
        value: Self::Balance
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        if value.is_zero() { return Ok(PositiveImbalance::new(0u32.into(), Self::get())) }

        <Balances<T>>::try_mutate(who, Self::get(), |balance| {
            // checked add?
            *balance = Some(balance.ok_or(Error::<T>::AccountNotFound)?.saturating_add(value));
            Ok(PositiveImbalance::new(value, Self::get()))
        })
    }

    fn deposit_creating(
        who: &T::AccountId,
        value: Self::Balance
    ) -> Self::PositiveImbalance {
        if value.is_zero() { return PositiveImbalance::new(0u32.into(), Self::get()) }

        <Balances<T>>::mutate(who, Self::get(), |balance| {
            // checked add?
            *balance = Some(balance.unwrap_or(Self::Balance::zero()).saturating_add(value));
            PositiveImbalance::new(value, Self::get())
        })
    }

    fn withdraw(
        who: &T::AccountId,
        value: Self::Balance,
        _: WithdrawReasons,
        _: ExistenceRequirement
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        <Balances<T>>::try_mutate(who, Self::get(), |balance| {
            *balance = Some(balance
                .map(|b| b.checked_sub(&value))
                .flatten()
                .ok_or(Error::<T>::OutOfFunds)?);

            Ok(Self::NegativeImbalance::new(value, Self::get()))
        })
    }

    fn make_free_balance_be(who: &T::AccountId, value: Self::Balance) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        <Balances<T>>::mutate(who, Self::get(), |balance| {
            let bal = balance.unwrap_or(T::Balance::zero());
            let im = if value > bal {
                SignedImbalance::Negative(NegativeImbalance::new(value - bal, Self::get()))
            } else {
                SignedImbalance::Positive(PositiveImbalance::new(bal - value, Self::get()))
            };
            *balance = Some(value);

            im
        })
    }
}
