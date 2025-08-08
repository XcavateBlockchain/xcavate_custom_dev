use frame_support::pallet_prelude::*;

use super::*;

pub trait PropertyTokenManage<T: Config> {
    fn create_property_token(
        funding_account: &AccountIdOf<T>,
        region: RegionId,
        location: LocationId<T>,
        token_amount: u32,
        property_price: <T as pallet::Config>::Balance,
        data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
    ) -> Result<(<T as pallet::Config>::NftId, u32), DispatchError>;

    fn burn_property_token(asset_id: u32) -> DispatchResult;
}

pub trait PropertyTokenOwnership<T: Config> {
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

    fn remove_property_token_ownership(asset_id: u32, account: &AccountIdOf<T>) -> DispatchResult;

    fn clear_token_owners(asset_id: u32) -> DispatchResult;
}

pub trait PropertyTokenSpvControl<T: Config> {
    fn register_spv(asset_id: u32) -> DispatchResult;

    fn finalize_property(asset_id: u32) -> DispatchResult;

    fn ensure_spv_not_created(asset_id: u32) -> DispatchResult;

    fn ensure_spv_created(asset_id: u32) -> DispatchResult;

    fn get_if_spv_not_created(
        asset_id: u32,
    ) -> Result<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
        DispatchError,
    >;

    fn get_if_property_finalized(
        asset_id: u32,
    ) -> Result<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
        DispatchError,
    >;

    fn ensure_property_finalized(asset_id: u32) -> DispatchResult;
}

pub trait PropertyTokenInspect<T: Config> {
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

impl<T: Config> PropertyTokenManage<T> for Pallet<T> {
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
        Self::do_burn_property_token(asset_id)
    }
}

impl<T: Config> PropertyTokenOwnership<T> for Pallet<T> {
    fn transfer_property_token(
        asset_id: u32,
        sender: &AccountIdOf<T>,
        funds_source: &AccountIdOf<T>,
        receiver: &AccountIdOf<T>,
        token_amount: u32,
    ) -> DispatchResult {
        Self::do_transfer_property_token(asset_id, sender, funds_source, receiver, token_amount)
    }

    fn distribute_property_token_to_owner(
        asset_id: u32,
        investor: &AccountIdOf<T>,
        token_amount: u32,
    ) -> DispatchResult {
        Self::do_distribute_property_token_to_owner(asset_id, investor, token_amount)
    }

    fn take_property_token(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
        Self::do_take_property_token(asset_id, owner)
    }

    fn remove_property_token_ownership(asset_id: u32, account: &AccountIdOf<T>) -> DispatchResult {
        Self::do_remove_property_token_ownership(asset_id, account)
    }

    fn clear_token_owners(asset_id: u32) -> DispatchResult {
        Self::do_clear_token_owners(asset_id)
    }
}

impl<T: Config> PropertyTokenSpvControl<T> for Pallet<T> {
    fn register_spv(asset_id: u32) -> DispatchResult {
        Self::do_register_spv(asset_id)
    }

    fn finalize_property(asset_id: u32) -> DispatchResult {
        Self::do_finalize_property(asset_id)
    }

    fn ensure_spv_not_created(asset_id: u32) -> DispatchResult {
        Self::do_ensure_spv_not_created(asset_id)
    }

    fn ensure_spv_created(asset_id: u32) -> DispatchResult {
        Self::do_ensure_spv_created(asset_id)
    }

    fn get_if_spv_not_created(
        asset_id: u32,
    ) -> Result<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
        DispatchError,
    > {
        Self::do_get_if_spv_not_created(asset_id)
    }

    fn get_if_property_finalized(
        asset_id: u32,
    ) -> Result<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
        DispatchError,
    > {
        Self::do_get_if_property_finalized(asset_id)
    }

    fn ensure_property_finalized(asset_id: u32) -> DispatchResult {
        Self::do_ensure_property_finalized(asset_id)
    }
}

impl<T: Config> PropertyTokenInspect<T> for Pallet<T> {
    fn get_property_asset_info(
        asset_id: u32,
    ) -> Option<
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
    > {
        Self::do_get_property_asset_info(asset_id)
    }

    fn get_property_owner(asset_id: u32) -> BoundedVec<AccountIdOf<T>, T::MaxPropertyToken> {
        Self::get_property_owner(asset_id)
    }

    fn get_token_balance(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
        Self::get_token_balance(asset_id, owner)
    }
}
