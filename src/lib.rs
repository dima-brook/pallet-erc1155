#![cfg_attr(not(feature = "std"), no_std)]

pub(crate) mod imbalance;
pub mod weights;
pub mod token;

pub use pallet::*;

use codec::{Codec};
use sp_std::fmt::Debug;
use weights::WeightInfo;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::{Saturating, AtLeast32BitUnsigned, StaticLookup};

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
        Transfer(Option<T::AccountId>, Option<T::AccountId>, T::TokenId, T::Balance)
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
        pub fn safe_transfer_from(
            from: OriginFor<T>,
            to: <T::Lookup as StaticLookup>::Source,
            token_id: T::TokenId,
            #[pallet::compact] value: T::Balance
        ) -> DispatchResultWithPostInfo {
            //let sender = ensure_signed(from)?;
            //let recv = T::Lookup::lookup(to)?;
            //<token::Erc1155Token<T, token_id>>::transfer(sender, recv);
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
        let token = <LastTokenId<T>>::get().unwrap();
        <LastTokenId<T>>::put(token.saturating_add(1u32.into()));

        return token;
    }
}
