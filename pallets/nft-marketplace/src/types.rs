use crate::*;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{DefaultNoBound, sp_runtime::RuntimeDebug};
use scale_info::TypeInfo;

/// Infos regarding the listing of a real estate object.
#[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct RegionInfo<T: Config> {
    pub collection_id: <T as pallet::Config>::NftCollectionId,
    pub listing_duration: BlockNumberFor<T>,
	pub owner: AccountIdOf<T>,
	pub tax: Balance,
}

/// Infos regarding a listed nft of a real estate object on the marketplace.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct NftDetails<T: Config> {
    pub spv_created: bool,
    pub asset_id: LocalAssetIdOf<T>,
    pub region: u32,
    pub location: LocationId<T>,
}

/// Infos regarding the listing of a real estate object.
#[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct NftListingDetails<NftId, NftCollectionId, T: Config> {
    pub real_estate_developer: AccountIdOf<T>,
    pub token_price: Balance,
    pub collected_funds: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
    pub collected_tax: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
    pub collected_fees: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
    pub asset_id: u32,
    pub item_id: NftId,
    pub collection_id: NftCollectionId,
    pub token_amount: u32,
    pub tax_paid_by_developer: bool,
    pub listing_expiry: BlockNumberFor<T>,
}

/// Infos regarding the listing of a token.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct TokenListingDetails<NftId, NftCollectionId, T: Config> {
    pub seller: AccountIdOf<T>,
    pub token_price: Balance,
    pub asset_id: u32,
    pub item_id: NftId,
    pub collection_id: NftCollectionId,
    pub amount: u32,
}

/// Infos regarding the asset id.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct AssetDetails<NftId, NftCollectionId, T: Config> {
    pub collection_id: NftCollectionId,
    pub item_id: NftId,
    pub region: u32,
    pub location: LocationId<T>,
    pub price: Balance,
    pub token_amount: u32,
}

/// Infos regarding an offer.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct OfferDetails<Balance, T: Config> {
    pub buyer: AccountIdOf<T>,
    pub token_price: Balance,
    pub amount: u32,
    pub payment_assets: PaymentAssets,
}

#[derive(Encode, Decode, CloneNoBound, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct PropertyLawyerDetails<T: Config> {
    pub real_estate_developer_lawyer: Option<AccountIdOf<T>>,
    pub spv_lawyer: Option<AccountIdOf<T>>,
    pub real_estate_developer_status: DocumentStatus,
    pub spv_status: DocumentStatus,
    pub real_estate_developer_lawyer_costs: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
    pub spv_lawyer_costs: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
    pub second_attempt: bool,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo, DefaultNoBound)]
#[scale_info(skip_type_params(T))]
pub struct TokenOwnerDetails<Balance, T: Config> {
    pub token_amount: u32,
    pub paid_funds: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
    pub paid_tax: BoundedBTreeMap<PaymentAssets, Balance, T::MaxNftToken>,
}

#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct RefundInfos<T: Config> {
    pub refund_amount: u32,
    pub property_lawyer_details: PropertyLawyerDetails<T>,
}

impl<Balance, T: Config> OfferDetails<Balance, T>
where
    Balance: CheckedMul + TryFrom<u128>,
{
    pub fn get_total_amount(&self) -> Result<Balance, Error<T>> {
        let amount_in_balance: Balance = (self.amount as u128)
            .try_into()
            .map_err(|_| Error::<T>::ConversionError)?;

        self.token_price
            .checked_mul(&amount_in_balance)
            .ok_or(Error::<T>::MultiplyError)
    }
}

/// Offer enum.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub enum TakeoverAction {
    Accept,
    Reject,
}

/// Offer enum.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub enum Offer {
    Accept,
    Reject,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub enum LegalProperty {
    RealEstateDeveloperSide,
    SpvSide,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub enum DocumentStatus {
    Pending,
    Approved,
    Rejected,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo, Ord, PartialOrd)]
pub enum PaymentAssets {
    #[codec(index = 0)]
    USDT,
    #[codec(index = 1)]
    USDC,
}

impl PaymentAssets {
    pub const fn id(&self) -> u32 {
        match self {
            PaymentAssets::USDT => 1984,
            PaymentAssets::USDC => 1337,
        }
    }
}

/// AccountId storage.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct PalletIdStorage<T: Config> {
    pallet_id: AccountIdOf<T>,
}