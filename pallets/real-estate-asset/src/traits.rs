use frame_support::pallet_prelude::*;

use super::*;

pub trait PropertyTokenTrait<T: Config> {
    fn create_property_token(
        funding_account: &AccountIdOf<T>,
        region: RegionId,
        location: LocationId<T>,
        token_amount: u32,
        property_price: <T as pallet::Config>::Balance,
        data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
    ) -> Result<(<T as pallet::Config>::NftId, u32), DispatchError>;

    fn burn_property_token(asset_id: u32) -> DispatchResult;

    fn transfer_property_token(
        asset_id: u32,
        sender: &AccountIdOf<T>,
        funds_source: &AccountIdOf<T>,
        receiver: &AccountIdOf<T>,
        token_amount: u32,
    ) -> DispatchResult;

    fn distribute_property_token_to_owner(
        asset_id: u32,
        investor: &AccountIdOf<T>,
        token_amount: u32,
    ) -> DispatchResult;

    fn take_property_token(asset_id: u32, owner: &AccountIdOf<T>) -> u32;

    fn remove_token_ownership(asset_id: u32, account: &AccountIdOf<T>) -> DispatchResult;

    fn remove_token_owner_list(asset_id: u32) -> DispatchResult;

    fn register_spv(asset_id: u32) -> DispatchResult;

    fn get_property_asset_info(
        asset_id: u32,
    ) -> Option<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
    >;

    fn get_property_owner(asset_id: u32) -> BoundedVec<AccountIdOf<T>, T::MaxPropertyToken>;

    fn get_token_balance(asset_id: u32, owner: &AccountIdOf<T>) -> u32;
}

impl<T: Config> PropertyTokenTrait<T> for Pallet<T> {
    fn create_property_token(
        funding_account: &AccountIdOf<T>,
        region: RegionId,
        location: LocationId<T>,
        token_amount: u32,
        property_price: <T as pallet::Config>::Balance,
        data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
    ) -> Result<(<T as pallet::Config>::NftId, u32), DispatchError> {
        Self::do_create_property_token(
            funding_account,
            region,
            location,
            token_amount,
            property_price,
            data,
        )
    }

    fn burn_property_token(asset_id: u32) -> DispatchResult {
        Self::burn_property_token(asset_id)
    }

    fn transfer_property_token(
        asset_id: u32,
        sender: &AccountIdOf<T>,
        funds_source: &AccountIdOf<T>,
        receiver: &AccountIdOf<T>,
        token_amount: u32,
    ) -> DispatchResult {
        Self::transfer_property_token(asset_id, sender, funds_source, receiver, token_amount)
    }

    fn distribute_property_token_to_owner(
        asset_id: u32,
        investor: &AccountIdOf<T>,
        token_amount: u32,
    ) -> DispatchResult {
        Self::distribute_property_token_to_owner(asset_id, investor, token_amount)
    }

    fn take_property_token(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
        Self::take_property_token(asset_id, owner)
    }

    fn remove_token_ownership(asset_id: u32, account: &AccountIdOf<T>) -> DispatchResult {
        Self::remove_token_ownership(asset_id, account)
    }

    fn remove_token_owner_list(asset_id: u32) -> DispatchResult {
        Self::remove_token_owner_list(asset_id)
    }

    fn register_spv(asset_id: u32) -> DispatchResult {
        Self::register_spv(asset_id)
    }

    fn get_property_asset_info(
        asset_id: u32,
    ) -> Option<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
    > {
        Self::get_property_asset_info(asset_id)
    }

    fn get_property_owner(asset_id: u32) -> BoundedVec<AccountIdOf<T>, T::MaxPropertyToken> {
        Self::get_property_owner(asset_id)
    }

    fn get_token_balance(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
        Self::get_token_balance(asset_id, owner)
    }
}
