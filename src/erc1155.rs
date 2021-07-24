use codec::FullCodec;
use frame_support::{traits::Imbalance, dispatch::{DispatchError, DispatchResult}};
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize};
use sp_std::{fmt::Debug, vec::Vec};


pub trait ERC1155<AccountId> {
    type TokenId: AtLeast32BitUnsigned + FullCodec + Default + Copy + MaybeSerializeDeserialize + Debug;
    type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    type PositiveImbalance: Imbalance<Self::Balance>;
    type NegativeImbalance: Imbalance<Self::Balance>;

    fn safe_transfer_from(from: &AccountId, to: &AccountId, id: &Self::TokenId, value: Self::Balance, calldata: Option<Vec<u8>>) -> DispatchResult;

    fn safe_batch_transfer_from(
        from: &AccountId, to: &AccountId,
        id_values: impl Iterator<Item = impl AsRef<(Self::TokenId, Self::Balance)>>,
        calldata: Option<Vec<u8>>
    ) -> DispatchResult {
        for v in id_values {
            let (id, value) = v.as_ref();
            Self::safe_transfer_from(from ,to, id, *value, calldata.clone())?;
        }

        Ok(())
    }

    fn balance_of(owner: &AccountId, id: &Self::TokenId) -> Self::Balance;

    fn balance_of_batch(
        owner_ids: impl Iterator<Item = impl AsRef<(AccountId, Self::TokenId)>>,
    ) -> Vec<Self::Balance> { // TODO: impl Iterator<Item = Balance>
        owner_ids.map(|v| {
            let (owner, id) = v.as_ref();
            Self::balance_of(owner, id)
        })
        .collect()
    }

    fn set_approval_for_all(owner: &AccountId, approved: bool);

    fn is_approved_for_all(owner: &AccountId, operator: &AccountId) -> bool;
}

pub trait ERC1155MetadataURI<AccountId>: ERC1155<AccountId> {
    type TokenInfo: Debug + FullCodec + MaybeSerializeDeserialize + Default + Clone + PartialEq;

    fn uri(id: &Self::TokenId) -> Self::TokenInfo;
}

pub trait ERC1155MetadataURIExt<AccountId>: ERC1155MetadataURI<AccountId> {
    fn set_uri(uri: &Self::TokenInfo);
}

pub trait ERC1155Mintable<AccountId>: ERC1155<AccountId> {
    fn mint(account: &AccountId, id: &Self::TokenId, amount: Self::Balance, calldata: Vec<u8>) -> Result<Self::PositiveImbalance, DispatchError>;

    fn mint_batch(
        account: &AccountId,
        id_amounts: impl Iterator<Item = impl AsRef<(Self::TokenId, Self::Balance)>>,
        calldata: Vec<u8>
    ) -> DispatchResult {
        for v in id_amounts {
            let (id, amount) = v.as_ref();
            Self::mint(account, id, *amount, calldata.clone())?;
        }

        Ok(())
    }
}

pub trait ERC1155Burnable<AccountId>: ERC1155<AccountId> {
    fn burn(account: &AccountId, id: &Self::TokenId, amount: Self::Balance) -> Result<Self::NegativeImbalance, DispatchError>;

    fn burn_batch(
        account: &AccountId,
        id_amounts: impl Iterator<Item = impl AsRef<(Self::TokenId, Self::Balance)>>
    ) -> DispatchResult {
        for v in id_amounts {
            let (id, amount) = v.as_ref();
            Self::burn(account, id, *amount)?;
        }

        Ok(())
    }
}

pub trait ERC1155Reservable<AccountId>: ERC1155<AccountId> {
    fn lock(owner: &AccountId, id: &Self::TokenId, amount: Self::Balance) -> DispatchResult;
    fn unlock(owner: &AccountId, id: &Self::TokenId, amount: Self::Balance) -> DispatchResult;
}
