#![cfg_attr(not(feature = "std"), no_std)]

pub(crate) mod imbalance;
pub mod weights;
pub mod token;
pub mod erc1155;

pub use pallet::*;
use erc1155::*;

use codec::{Codec};
use sp_std::fmt::Debug;
use weights::WeightInfo;
use frame_support::{dispatch::{DispatchResult, DispatchError}, ensure};
use frame_system::{pallet_prelude::BlockNumberFor};
use sp_runtime::traits::{Saturating, AtLeast32BitUnsigned, StaticLookup, Zero, CheckedSub};


#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_support::{
        pallet_prelude::*,
    };
    use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
        type Balance: Parameter + Member + AtLeast32BitUnsigned + Codec + Default + Copy
            + MaybeSerializeDeserialize + Debug + MaxEncodedLen;

        /// Token Identifier, it is recommended to use u128 here
        type TokenId: Parameter + Member + AtLeast32BitUnsigned + Codec + Default + Copy
            + MaybeSerializeDeserialize + Debug + MaxEncodedLen;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn issuance)]
    pub type Issuance<T: Config> = StorageMap<_, Twox64Concat, T::TokenId, T::Balance>;

    #[pallet::storage]
    #[pallet::getter(fn balance_of)]
    pub type Balances<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Twox64Concat, T::TokenId, T::Balance>;

    #[pallet::storage]
    pub type LastTokenId<T: Config> = StorageValue<_, T::TokenId>;

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId", T::Balance = "Balance")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// Transfer event
        /// from is None when minting
        /// to is None when burning
        ///
        /// from, to, token_id, value
        TransferSingle(Option<T::AccountId>, Option<T::AccountId>, T::TokenId, T::Balance)
	}

	#[pallet::error]
	pub enum Error<T> {
        TokenNotFound,
        OutOfFunds,
        AccountNotFound
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn safe_transfer(
            from: OriginFor<T>,
            to: <T::Lookup as StaticLookup>::Source,
            token_id: T::TokenId,
            #[pallet::compact] value: T::Balance
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(from)?;
            let recv = T::Lookup::lookup(to)?;

            Self::safe_transfer_from(&sender, &recv, &token_id, value, None)?;
            Ok(().into())
        }
	}

    /// Genesis config
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_token: T::TokenId,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                initial_token: 0u32.into(),
            }
        }
    }

    #[cfg(feature = "std")]
    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
    {
        fn build(&self) {
            <LastTokenId<T>>::put(self.initial_token);
            <Issuance<T>>::insert(self.initial_token, T::Balance::from(0u32))
        }
    }
}

impl<T: Config> pallet::Pallet<T> {
    pub fn create_token(account: T::AccountId, initial_supply: T::Balance) -> T::TokenId {
        let token = Self::token_inc();
        <Balances<T>>::insert(account, token, initial_supply);
        <Issuance<T>>::insert(token, initial_supply);

        return token;
    }

    fn last_token() -> T::TokenId {
        // unwrap safety: initialized at genesis_build
        <LastTokenId<T>>::get().unwrap()
    }

    fn token_inc() -> T::TokenId {
        // unwrap safety: initialized at genesis_build
        let token = Self::last_token();
        <LastTokenId<T>>::put(token.saturating_add(1u32.into()));

        return token;
    }
}

impl<T: Config> ERC1155<T::AccountId> for pallet::Pallet<T> {
    type TokenId = T::TokenId;
    type Balance = T::Balance;
    type PositiveImbalance = imbalance::PositiveImbalance<T>;
    type NegativeImbalance = imbalance::NegativeImbalance<T>;

    fn safe_transfer_from(
        from: &T::AccountId,
        to: &T::AccountId,
        id: &T::TokenId,
        value: T::Balance,
        _calldata: Option<Vec<u8>>
    ) -> DispatchResult {
        ensure!(
            *to != T::AccountId::default(),
            Error::<T>::AccountNotFound
        );

        if value.is_zero() || from == to {
            return Ok(());
        }

        <Balances<T>>::try_mutate(from, *id, |balance| -> Result<(), Error<T>> {
            *balance = Some(balance.map(|b| b.checked_sub(&value))
                .flatten()
                .ok_or(Error::<T>::OutOfFunds)?);
            <Balances<T>>::mutate(to, *id, |balance_target| {
                // Should we consider checked add?
                *balance_target = Some(balance.unwrap().saturating_add(value));
            });

            Ok(())
        })?;

        Self::deposit_event(Event::TransferSingle(Some(from.clone()), Some(to.clone()), *id, value));
        // TODO: Handle ERC1155Receiver

        Ok(())
    }

    fn balance_of(owner: &T::AccountId, id: &Self::TokenId) -> Self::Balance {
        <Balances<T>>::get(owner, id).clone().unwrap_or(T::Balance::zero())
    }

    fn set_approval_for_all(_owner: &T::AccountId, _approved: bool) {
        unimplemented!();
    }

    fn is_approved_for_all(_owner: &T::AccountId, _operator: &T::AccountId) -> bool {
        unimplemented!();
    }
}

impl<T: Config> ERC1155Mintable<T::AccountId> for pallet::Pallet<T> {
    fn mint(
        account: &T::AccountId,
        id: &Self::TokenId,
        amount: Self::Balance,
        _calldata: Vec<u8>
    ) -> Result<Self::PositiveImbalance, DispatchError> {
        ensure!(
            *account != T::AccountId::default(),
            Error::<T>::AccountNotFound
        );

        if amount.is_zero() {
            return Ok(Self::PositiveImbalance::new(0u32.into(), *id))
        }

        let res = <Balances<T>>::mutate(account, id, |balance| {
            // checked add?
            *balance = Some(balance.unwrap_or(Self::Balance::zero()).saturating_add(amount));
            Self::PositiveImbalance::new(amount, *id)
        });

        // TODO: ERC115Receiver
        Ok(res)
    }
}

impl<T: Config> ERC1155Burnable<T::AccountId> for pallet::Pallet<T> {
    fn burn(
        account: &T::AccountId,
        id: &Self::TokenId,
        amount: Self::Balance
    ) -> Result<Self::NegativeImbalance, DispatchError> {
        <Balances<T>>::try_mutate(account, id, |balance| {
            *balance = Some(balance
                .map(|b| b.checked_sub(&amount))
                .flatten()
                .ok_or(Error::<T>::OutOfFunds)?);

            Ok(Self::NegativeImbalance::new(amount, *id))
        })

    }
}
