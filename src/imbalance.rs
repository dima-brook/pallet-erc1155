// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::{
    Config, Saturating,
};
use sp_std::{mem, result};
use sp_runtime::{RuntimeDebug, traits::Zero};
use frame_support::traits::{SameOrOther, Imbalance, TryDrop};

/// Opaque, move-only struct with private fields that serves as a token denoting that
/// funds have been created without any equal and opposite accounting.
#[must_use]
#[derive(RuntimeDebug, PartialEq, Eq)]
pub struct PositiveImbalance<T: Config>(T::Balance, T::TokenId);

impl<T: Config> PositiveImbalance<T> {
    /// Create a new positive imbalance from a balance.
    pub fn new(amount: T::Balance, token: T::TokenId) -> Self {
        PositiveImbalance(amount, token)
    }
}

/// Opaque, move-only struct with private fields that serves as a token denoting that
/// funds have been destroyed without any equal and opposite accounting.
#[must_use]
#[derive(RuntimeDebug, PartialEq, Eq)]
pub struct NegativeImbalance<T: Config>(T::Balance, T::TokenId);

impl<T: Config> NegativeImbalance<T> {
    /// Create a new negative imbalance from a balance.
    pub fn new(amount: T::Balance, token: T::TokenId) -> Self {
        NegativeImbalance(amount, token)
    }
}

impl<T: Config> TryDrop for PositiveImbalance<T> {
    fn try_drop(self) -> result::Result<(), Self> {
        self.drop_zero()
    }
}

impl<T: Config> Default for PositiveImbalance<T> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<T: Config> Imbalance<T::Balance> for PositiveImbalance<T> {
    type Opposite = NegativeImbalance<T>;

    fn zero() -> Self {
        panic!("zero is not supposed to be used with erc1155 imbalance!")
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

        let token = self.1.clone();
        mem::forget(self);
        (Self::new(first, token.clone()), Self::new(second, token))
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
        let token = self.1.clone();
        mem::forget((self, other));

        if a > b {
            SameOrOther::Same(Self::new(a - b, token))
        } else if b > a {
            SameOrOther::Other(NegativeImbalance::new(b - a, token))
        } else {
            SameOrOther::None
        }
    }
    fn peek(&self) -> T::Balance {
        self.0.clone()
    }
}

impl<T: Config> TryDrop for NegativeImbalance<T> {
    fn try_drop(self) -> result::Result<(), Self> {
        self.drop_zero()
    }
}

impl<T: Config> Default for NegativeImbalance<T> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<T: Config> Imbalance<T::Balance> for NegativeImbalance<T> {
    type Opposite = PositiveImbalance<T>;

    fn zero() -> Self {
        panic!("zero shouldn't be used with erc1155 imbalance!")
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

        let token = self.1.clone();
        mem::forget(self);
        (Self::new(first, token.clone()), Self::new(second, token))
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
        let token = self.1.clone();
        mem::forget((self, other));

        if a > b {
            SameOrOther::Same(Self::new(a - b, token))
        } else if b > a {
            SameOrOther::Other(Self::Opposite::new(b - a, token))
        } else {
            SameOrOther::None
        }
    }
    fn peek(&self) -> T::Balance {
        self.0.clone()
    }
}

impl<T: Config> Drop for PositiveImbalance<T> {
    /// Basic drop handler will just square up the total issuance.
    fn drop(&mut self) {
        <super::Issuance<T>>::mutate(
            self.1,
            |v| *v = Some(v.unwrap_or(0u32.into()).saturating_add(self.0))
        );
    }
}

impl<T: Config> Drop for NegativeImbalance<T> {
    /// Basic drop handler will just square up the total issuance.
    fn drop(&mut self) {
        <super::Issuance<T>>::mutate(
            self.1,
            |v| *v = Some(v.unwrap_or(0u32.into()).saturating_sub(self.0))
        );
    }
}
