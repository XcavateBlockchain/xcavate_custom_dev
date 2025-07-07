#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod traits;

use frame_support::pallet_prelude::*;

use frame_support::sp_runtime::traits::{AccountIdConversion, StaticLookup};
use frame_support::{
    traits::{
        fungible::Mutate,
        fungibles::Inspect as FungiblesInspect,
        fungibles::Mutate as FungiblesMutate,
        nonfungibles_v2::Mutate as NonfungiblesMutate,
        nonfungibles_v2::Transfer,
        tokens::{fungible, fungibles, nonfungibles_v2, Balance, Preservation},
    },
    PalletId,
};

use frame_system::RawOrigin;

use pallet_nfts::{CollectionConfig, ItemConfig, ItemSettings};

use pallet_regions::LocationId;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type LocalAssetIdOf<T> = <<T as Config>::LocalCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{CheckedAdd, One};

    /// Infos regarding the property asset.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct PropertyAssetDetails<NftId, NftCollectionId, T: Config> {
        pub collection_id: NftCollectionId,
        pub item_id: NftId,
        pub region: RegionId,
        pub location: LocationId<T>,
        pub price: <T as pallet::Config>::Balance,
        pub token_amount: u32,
        pub spv_created: bool,
    }

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_nfts::Config
        + pallet_xcavate_whitelist::Config
        + pallet_regions::Config
        + pallet_nft_fractionalization::Config
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type Balance: Balance + TypeInfo + From<u128> + Default;

        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// The type used to identify an NFT within a collection.
        type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

        type Nfts: nonfungibles_v2::Inspect<
                Self::AccountId,
                ItemId = <Self as pallet::Config>::NftId,
                CollectionId = <Self as pallet_regions::Config>::NftCollectionId,
            > + Transfer<Self::AccountId>
            + nonfungibles_v2::Mutate<Self::AccountId, ItemConfig>
            + nonfungibles_v2::Create<
                Self::AccountId,
                CollectionConfig<
                    <Self as pallet_regions::Config>::Balance,
                    BlockNumberFor<Self>,
                    <Self as pallet_nfts::Config>::CollectionId,
                >,
            >;

        /// The marketplace's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        type LocalCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// Collection id type from pallet nft fractionalization.
        type FractionalizeCollectionId: IsType<<Self as pallet_nft_fractionalization::Config>::NftCollectionId>
            + Parameter
            + From<<Self as pallet_regions::Config>::NftCollectionId>
            + Ord
            + Copy
            + MaxEncodedLen
            + Encode;

        /// Item id type from pallet nft fractionalization.
        type FractionalizeItemId: IsType<<Self as pallet_nft_fractionalization::Config>::NftId>
            + Parameter
            + From<<Self as pallet::Config>::NftId>
            + Ord
            + Copy
            + MaxEncodedLen
            + Encode;

        /// Asset id type from pallet nft fractionalization.
        type AssetId: IsType<<Self as pallet_nft_fractionalization::Config>::AssetId>
            + Parameter
            + From<u32>
            + Ord
            + Copy;

        /// Amount to fund a property account.
        #[pallet::constant]
        type PropertyAccountFundingAmount: Get<<Self as pallet::Config>::Balance>;

        /// The maximum amount of token of a property.
        #[pallet::constant]
        type MaxPropertyToken: Get<u32>;
    }

    pub type FractionalizedAssetId<T> = <T as Config>::AssetId;
    pub type FractionalizeCollectionId<T> = <T as Config>::FractionalizeCollectionId;
    pub type FractionalizeItemId<T> = <T as Config>::FractionalizeItemId;

    pub type RegionId = u16;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Id for the next nft in a collection.
    #[pallet::storage]
    pub type NextNftId<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        <T as pallet_regions::Config>::NftCollectionId,
        <T as pallet::Config>::NftId,
        ValueQuery,
    >;

    /// Id of the possible next asset that would be used for
    /// Nft fractionalization.
    #[pallet::storage]
    pub type NextAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Mapping of the assetid to the property details.
    #[pallet::storage]
    pub type PropertyAssetInfo<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        PropertyAssetDetails<
            <T as pallet::Config>::NftId,
            <T as pallet_regions::Config>::NftCollectionId,
            T,
        >,
        OptionQuery,
    >;

    /// Mapping of the assetid to the vector of token holder.
    #[pallet::storage]
    pub type PropertyOwner<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        BoundedVec<AccountIdOf<T>, T::MaxPropertyToken>,
        ValueQuery,
    >;

    /// Mapping of assetid and accountid to the amount of token an account is holding of the asset.
    #[pallet::storage]
    pub type PropertyOwnerToken<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,
        Blake2_128Concat,
        AccountIdOf<T>,
        u32,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Test
        PropertyTokenCreated { asset_id: u32 },
        /// The property nft got burned.
        PropertyNftBurned {
            collection_id: <T as pallet_regions::Config>::NftCollectionId,
            item_id: <T as pallet::Config>::NftId,
            asset_id: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// This Region is not known.
        RegionUnknown,
        ArithmeticOverflow,
        /// The account doesn't have enough funds.
        NotEnoughFunds,
        /// The property asset could not be found.
        PropertyAssetNotRegistered,
        /// The sender has not enough token.
        NotEnoughToken,
        /// This index is not taken.
        InvalidIndex,
        /// There are already too many token buyer.
        TooManyTokenBuyer,
    }

    impl<T: Config> Pallet<T> {
        /// Get the account id of the pallet
        pub fn account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::PalletId::get().into_account_truncating()
        }

        pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
            <T as pallet::Config>::PalletId::get().into_sub_account_truncating(("pr", asset_id))
        }

        pub(crate) fn do_create_property_token(
            funding_account: &AccountIdOf<T>,
            region: RegionId,
            location: LocationId<T>,
            token_amount: u32,
            property_price: <T as pallet::Config>::Balance,
            data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
        ) -> Result<(<T as pallet::Config>::NftId, u32), DispatchError> {
            let region_info =
                pallet_regions::RegionDetails::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
            let item_id = NextNftId::<T>::get(region_info.collection_id);
            let mut asset_number: u32 = NextAssetId::<T>::get();
            let mut asset_id: LocalAssetIdOf<T> = asset_number;
            while !T::LocalCurrency::total_issuance(asset_id).is_zero() {
                asset_number = asset_number
                    .checked_add(1)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                asset_id = asset_number;
            }
            let asset_id: FractionalizedAssetId<T> = asset_number.into();
            let pallet_account = Self::account_id();
            let property_account = Self::property_account_id(asset_number);

            <T as pallet::Config>::NativeCurrency::transfer(
                funding_account,
                &property_account,
                T::PropertyAccountFundingAmount::get(),
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughFunds)?;

            <T as pallet::Config>::Nfts::mint_into(
                &region_info.collection_id,
                &item_id,
                &property_account.clone(),
                &Self::default_item_config(),
                true,
            )?;
            <T as pallet::Config>::Nfts::set_item_metadata(
                Some(&pallet_account),
                &region_info.collection_id,
                &item_id,
                &data,
            )?;

            // Fractionalize NFT
            let property_origin: OriginFor<T> = RawOrigin::Signed(property_account.clone()).into();
            let user_lookup = <T::Lookup as StaticLookup>::unlookup(property_account.clone());
            let fractionalize_collection_id =
                FractionalizeCollectionId::<T>::from(region_info.collection_id);
            let fractionalize_item_id = FractionalizeItemId::<T>::from(item_id);

            pallet_nft_fractionalization::Pallet::<T>::fractionalize(
                property_origin.clone(),
                fractionalize_collection_id.into(),
                fractionalize_item_id.into(),
                asset_id.into(),
                user_lookup,
                token_amount.into(),
            )?;

            // Store asset details
            PropertyAssetInfo::<T>::insert(
                asset_number,
                PropertyAssetDetails {
                    collection_id: region_info.collection_id,
                    item_id,
                    region,
                    location,
                    price: property_price,
                    token_amount,
                    spv_created: false,
                },
            );

            let next_item_id = item_id
                .checked_add(&One::one())
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let next_asset_number = asset_number
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            NextNftId::<T>::insert(region_info.collection_id, next_item_id);
            NextAssetId::<T>::put(next_asset_number);
            Self::deposit_event(Event::<T>::PropertyTokenCreated {
                asset_id: asset_number,
            });
            Ok((item_id, asset_number))
        }

        pub(crate) fn burn_property_token(asset_id: u32) -> DispatchResult {
            let asset_details = PropertyAssetInfo::<T>::take(asset_id)
                .ok_or(Error::<T>::PropertyAssetNotRegistered)?;
            let pallet_account = Self::property_account_id(asset_id);
            let pallet_origin: OriginFor<T> = RawOrigin::Signed(pallet_account.clone()).into();
            let user_lookup = <T::Lookup as StaticLookup>::unlookup(pallet_account);
            let fractionalize_collection_id =
                FractionalizeCollectionId::<T>::from(asset_details.collection_id);
            let fractionalize_item_id = FractionalizeItemId::<T>::from(asset_details.item_id);
            let fractionalize_asset_id = FractionalizedAssetId::<T>::from(asset_id);
            pallet_nft_fractionalization::Pallet::<T>::unify(
                pallet_origin.clone(),
                fractionalize_collection_id.into(),
                fractionalize_item_id.into(),
                fractionalize_asset_id.into(),
                user_lookup,
            )?;
            <T as pallet::Config>::Nfts::burn(
                &asset_details.collection_id,
                &asset_details.item_id,
                None,
            )?;
            Self::deposit_event(Event::<T>::PropertyNftBurned {
                collection_id: asset_details.collection_id,
                item_id: asset_details.item_id,
                asset_id,
            });
            Ok(())
        }

        pub(crate) fn transfer_property_token(
            asset_id: u32,
            sender: &AccountIdOf<T>,
            funds_source: &AccountIdOf<T>,
            receiver: &AccountIdOf<T>,
            token_amount: u32,
        ) -> DispatchResult {
            let sender_token_amount = PropertyOwnerToken::<T>::take(asset_id, sender);
            let new_sender_token_amount = sender_token_amount
                .checked_sub(token_amount)
                .ok_or(Error::<T>::NotEnoughToken)?;
            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                funds_source,
                receiver,
                token_amount.into(),
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughToken)?;
            if new_sender_token_amount == 0 {
                let mut owner_list = PropertyOwner::<T>::take(asset_id);
                let index = owner_list
                    .iter()
                    .position(|x| x == sender)
                    .ok_or(Error::<T>::InvalidIndex)?;
                owner_list.remove(index);
                PropertyOwner::<T>::insert(asset_id, owner_list);
            } else {
                PropertyOwnerToken::<T>::insert(asset_id, sender, new_sender_token_amount);
            }
            if PropertyOwner::<T>::get(asset_id).contains(receiver) {
                PropertyOwnerToken::<T>::try_mutate(asset_id, receiver, |receiver_balance| {
                    *receiver_balance = receiver_balance
                        .checked_add(token_amount)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    Ok::<(), DispatchError>(())
                })?;
            } else {
                PropertyOwner::<T>::try_mutate(asset_id, |owner_list| {
                    owner_list
                        .try_push(receiver.clone())
                        .map_err(|_| Error::<T>::TooManyTokenBuyer)?;
                    Ok::<(), DispatchError>(())
                })?;
                PropertyOwnerToken::<T>::insert(asset_id, receiver, token_amount);
            }
            Ok(())
        }

        pub(crate) fn distribute_property_token_to_owner(
            asset_id: u32,
            investor: &AccountIdOf<T>,
            token_amount: u32,
        ) -> DispatchResult {
            let property_account = Self::property_account_id(asset_id);

            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                &property_account,
                investor,
                token_amount.into(),
                Preservation::Expendable,
            )?;
            PropertyOwner::<T>::try_mutate(asset_id, |keys| {
                keys.try_push(investor.clone())
                    .map_err(|_| Error::<T>::TooManyTokenBuyer)?;
                Ok::<(), DispatchError>(())
            })?;
            PropertyOwnerToken::<T>::insert(asset_id, investor, token_amount);

            Ok(())
        }

        pub(crate) fn take_property_token(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
            PropertyOwnerToken::<T>::take(asset_id, owner)
        }

        pub(crate) fn remove_token_ownership(
            asset_id: u32,
            account: &AccountIdOf<T>,
        ) -> DispatchResult {
            PropertyOwnerToken::<T>::take(asset_id, account);
            Ok(())
        }

        pub(crate) fn remove_token_owner_list(asset_id: u32) -> DispatchResult {
            PropertyOwner::<T>::take(asset_id);
            Ok(())
        }

        pub(crate) fn register_spv(asset_id: u32) -> DispatchResult {
            PropertyAssetInfo::<T>::try_mutate(asset_id, |maybe_asset_details| {
                let asset_details = maybe_asset_details
                    .as_mut()
                    .ok_or(Error::<T>::PropertyAssetNotRegistered)?;
                asset_details.spv_created = true;
                Ok::<(), DispatchError>(())
            })
        }

        /// Set the default item configuration for minting a nft.
        fn default_item_config() -> ItemConfig {
            ItemConfig {
                settings: ItemSettings::all_enabled(),
            }
        }

        pub(crate) fn get_property_asset_info(
            asset_id: u32,
        ) -> Option<
            PropertyAssetDetails<
                <T as pallet::Config>::NftId,
                <T as pallet_regions::Config>::NftCollectionId,
                T,
            >,
        > {
            PropertyAssetInfo::<T>::get(asset_id)
        }

        pub(crate) fn get_property_owner(
            asset_id: u32,
        ) -> BoundedVec<AccountIdOf<T>, T::MaxPropertyToken> {
            PropertyOwner::<T>::get(asset_id)
        }

        pub(crate) fn get_token_balance(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
            PropertyOwnerToken::<T>::get(asset_id, owner)
        }
    }
}
