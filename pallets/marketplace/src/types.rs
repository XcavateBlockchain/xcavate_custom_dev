use crate::*;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::sp_runtime::Permill;
use frame_support::{sp_runtime::RuntimeDebug, DefaultNoBound};
use scale_info::TypeInfo;

/// Infos regarding the listing of a real estate object.
#[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct RegionInfo<T: Config> {
    pub collection_id: <T as pallet_regions::Config>::NftCollectionId,
    pub listing_duration: BlockNumberFor<T>,
    pub owner: AccountIdOf<T>,
    pub tax: Permill,
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
pub struct PropertyListingDetails<NftId, NftCollectionId, T: Config> {
    pub real_estate_developer: AccountIdOf<T>,
    pub token_price: <T as pallet::Config>::Balance,
    pub collected_funds: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
    pub collected_tax: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
    pub collected_fees: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
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
    pub token_price: <T as pallet::Config>::Balance,
    pub asset_id: u32,
    pub item_id: NftId,
    pub collection_id: NftCollectionId,
    pub amount: u32,
}

/// Infos regarding an offer.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct OfferDetails<T: Config> {
    pub token_price: <T as pallet::Config>::Balance,
    pub amount: u32,
    pub payment_assets: u32,
}

#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(T))]
pub struct PropertyLawyerDetails<T: Config> {
    pub real_estate_developer_lawyer: Option<AccountIdOf<T>>,
    pub spv_lawyer: Option<AccountIdOf<T>>,
    pub real_estate_developer_status: DocumentStatus,
    pub spv_status: DocumentStatus,
    pub real_estate_developer_lawyer_costs: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
    pub spv_lawyer_costs: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
    pub second_attempt: bool,
}

#[derive(
    Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo, DefaultNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct TokenOwnerDetails<T: Config> {
    pub token_amount: u32,
    pub paid_funds: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
    pub paid_tax: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxPropertyToken,
    >,
}

#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct RefundInfos<T: Config> {
    pub refund_amount: u32,
    pub property_lawyer_details: PropertyLawyerDetails<T>,
}

impl<T: Config> OfferDetails<T>
where
    <T as pallet::Config>::Balance: CheckedMul + TryFrom<u128>,
{
    pub fn get_total_amount(&self) -> Result<<T as pallet::Config>::Balance, Error<T>> {
        let amount_in_balance: <T as pallet::Config>::Balance = (self.amount as u128).into();

        self.token_price
            .checked_mul(&amount_in_balance)
            .ok_or(Error::<T>::MultiplyError)
    }
}

#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProposedDeveloperLawyer<T: Config> {
    pub lawyer: AccountIdOf<T>,
    pub costs: <T as pallet::Config>::Balance,
}

#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProposedSpvLawyer<T: Config> {
    pub lawyer: AccountIdOf<T>,
    pub costs: <T as pallet::Config>::Balance,
    pub expiry_block: BlockNumberFor<T>,
}

/// Voting stats.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct VoteStats {
    pub yes_voting_power: u32,
    pub no_voting_power: u32,
}

/// Takeover enum.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Clone,
    PartialEq,
    Eq,
    MaxEncodedLen,
    RuntimeDebug,
    TypeInfo,
)]
pub enum TakeoverAction {
    Accept,
    Reject,
}

/// Offer enum.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Clone,
    PartialEq,
    Eq,
    MaxEncodedLen,
    RuntimeDebug,
    TypeInfo,
)]
pub enum Offer {
    Accept,
    Reject,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Clone,
    PartialEq,
    Eq,
    MaxEncodedLen,
    RuntimeDebug,
    TypeInfo,
)]
pub enum LegalProperty {
    RealEstateDeveloperSide,
    SpvSide,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Clone,
    PartialEq,
    Eq,
    MaxEncodedLen,
    RuntimeDebug,
    TypeInfo,
)]
pub enum DocumentStatus {
    Pending,
    Approved,
    Rejected,
}

/// Vote enum.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Clone,
    PartialEq,
    Eq,
    MaxEncodedLen,
    RuntimeDebug,
    TypeInfo,
)]
pub enum Vote {
    Yes,
    No,
}

/// AccountId storage.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct PalletIdStorage<T: Config> {
    pallet_id: AccountIdOf<T>,
}
