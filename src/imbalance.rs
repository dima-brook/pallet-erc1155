// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    Config, Saturating,
};
use sp_std::{mem, result};
use sp_runtime::{RuntimeDebug, traits::Zero};
use frame_support::{traits::{SameOrOther, Imbalance, TryDrop, Get}, pallet_prelude::PhantomData};

/// Opaque, move-only struct with private fields that serves as a token denoting that
/// funds have been created without any equal and opposite accounting.
#[must_use]
#[derive(RuntimeDebug, PartialEq, Eq)]
pub struct PositiveImbalance<T: Config, Token: Get<T::TokenId>>(T::Balance, PhantomData<Token>);

impl<T: Config, Token: Get<T::TokenId>> PositiveImbalance<T, Token> {
    /// Create a new positive imbalance from a balance.
    pub fn new(amount: T::Balance) -> Self {
        PositiveImbalance(amount, PhantomData)
    }
}

/// Opaque, move-only struct with private fields that serves as a token denoting that
/// funds have been destroyed without any equal and opposite accounting.
#[must_use]
#[derive(RuntimeDebug, PartialEq, Eq)]
pub struct NegativeImbalance<T: Config, Token: Get<T::TokenId>>(T::Balance, PhantomData<Token>);

impl<T: Config, Token: Get<T::TokenId>> NegativeImbalance<T, Token> {
    /// Create a new negative imbalance from a balance.
    pub fn new(amount: T::Balance) -> Self {
        NegativeImbalance(amount, PhantomData)
    }
}

impl<T: Config, Token: Get<T::TokenId>> TryDrop for PositiveImbalance<T, Token> {
    fn try_drop(self) -> result::Result<(), Self> {
        self.drop_zero()
    }
}

impl<T: Config, Token: Get<T::TokenId>> Default for PositiveImbalance<T, Token> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<T: Config, Token: Get<T::TokenId>> Imbalance<T::Balance> for PositiveImbalance<T, Token> {
    type Opposite = NegativeImbalance<T, Token>;

    fn zero() -> Self {
        Self(Zero::zero(), PhantomData)
    }
    fn drop_zero(self) -> result::Result<(), Self> {
        if self.0.is_zero() {
            Ok(())
        } else {
            Err(self)
        }
    }
    fn split(self, amount: T::Balance) -> (Self, Self) {
        let first = self.0.min(amount);
        let second = self.0 - first;

        mem::forget(self);
        (Self::new(first), Self::new(second))
    }
    fn merge(mut self, other: Self) -> Self {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);

        self
    }
    fn subsume(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
    }
    fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
        let (a, b) = (self.0, other.0);
        mem::forget((self, other));

        if a > b {
            SameOrOther::Same(Self::new(a - b))
        } else if b > a {
            SameOrOther::Other(NegativeImbalance::new(b - a))
        } else {
            SameOrOther::None
        }
    }
    fn peek(&self) -> T::Balance {
        self.0.clone()
    }
}

impl<T: Config, Token: Get<T::TokenId>> TryDrop for NegativeImbalance<T, Token> {
    fn try_drop(self) -> result::Result<(), Self> {
        self.drop_zero()
    }
}

impl<T: Config, Token: Get<T::TokenId>> Default for NegativeImbalance<T, Token> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<T: Config, Token: Get<T::TokenId>> Imbalance<T::Balance> for NegativeImbalance<T, Token> {
    type Opposite = PositiveImbalance<T, Token>;

    fn zero() -> Self {
        Self::new(Zero::zero())
    }
    fn drop_zero(self) -> result::Result<(), Self> {
        if self.0.is_zero() {
            Ok(())
        } else {
            Err(self)
        }
    }
    fn split(self, amount: T::Balance) -> (Self, Self) {
        let first = self.0.min(amount);
        let second = self.0 - first;

        mem::forget(self);
        (Self::new(first), Self::new(second))
    }
    fn merge(mut self, other: Self) -> Self {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);

        self
    }
    fn subsume(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
        mem::forget(other);
    }
    fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
        let (a, b) = (self.0, other.0);
        mem::forget((self, other));

        if a > b {
            SameOrOther::Same(Self::new(a - b))
        } else if b > a {
            SameOrOther::Other(Self::Opposite::new(b - a))
        } else {
            SameOrOther::None
        }
    }
    fn peek(&self) -> T::Balance {
        self.0.clone()
    }
}

impl<T: Config, Token: Get<T::TokenId>> Drop for PositiveImbalance<T, Token> {
    /// Basic drop handler will just square up the total issuance.
    fn drop(&mut self) {
        <super::Issuance<T>>::mutate(
            Token::get(),
            |v| *v = Some(v.unwrap_or(0u32.into()).saturating_add(self.0))
        );
    }
}

impl<T: Config, Token: Get<T::TokenId>> Drop for NegativeImbalance<T, Token> {
    /// Basic drop handler will just square up the total issuance.
    fn drop(&mut self) {
        <super::Issuance<T>>::mutate(
            Token::get(),
            |v| *v = Some(v.unwrap_or(0u32.into()).saturating_sub(self.0))
        );
    }
}
