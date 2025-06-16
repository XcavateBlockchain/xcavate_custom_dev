#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use pallet_nfts::{
	CollectionConfig, CollectionSettings, ItemConfig, MintSettings,
};

use frame_support::{
    pallet_prelude::*, PalletId,
    traits::{
        tokens::{fungible, nonfungibles_v2, Balance, Precision},
		nonfungibles_v2::{Create, Transfer},
        fungible::MutateHold,
    }
};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::{CheckedAdd, One, AccountIdConversion}, Permill};

    /// Infos regarding the listing of a real estate object.
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionInfo<T: Config> {
        pub collection_id: <T as pallet::Config>::NftCollectionId,
        pub listing_duration: BlockNumberFor<T>,
        pub owner: T::AccountId,
        pub tax: Permill,
    }

	/// Takeover enum.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	pub enum TakeoverAction {
		Accept,
		Reject,
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Funds are held for operating a region.
		#[codec(index = 0)]
		RegionDepositReserve,
		#[codec(index = 1)]
		LocationDepositReserve,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcavate_whitelist::Config + pallet_nfts::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Balance: Balance + TypeInfo;

        type NativeCurrency: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::InspectHold<Self::AccountId, Balance = Self::Balance>
			+ fungible::BalancedHold<Self::AccountId, Balance = Self::Balance>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId, Reason = <Self as pallet::Config>::RuntimeHoldReason>; 

        /// The overarching hold reason.
		type RuntimeHoldReason: From<HoldReason>;

        type Nfts: nonfungibles_v2::Inspect<Self::AccountId, ItemId = <Self as pallet::Config>::NftId,
			CollectionId = <Self as pallet::Config>::NftCollectionId>	
			+ Transfer<Self::AccountId>
			+ nonfungibles_v2::Mutate<Self::AccountId, ItemConfig>
			+ nonfungibles_v2::Create<Self::AccountId, CollectionConfig<Self::Balance, 
			BlockNumberFor<Self>, <Self as pallet_nfts::Config>::CollectionId>>;

        /// Identifier for the collection of NFT.
		type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy;

        /// The type used to identify an NFT within a collection.
		type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

        /// A deposit for operating a region.
		#[pallet::constant]
		type RegionDeposit: Get<Self::Balance>;

        /// The marketplace's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

        #[pallet::constant]
		type MaxListingDuration: Get<BlockNumberFor<Self>>;

		/// The maximum length of data stored in for post codes.
		#[pallet::constant]
		type PostcodeLimit: Get<u32>;

		/// A deposit for operating a location.
		#[pallet::constant]
		type LocationDeposit: Get<Self::Balance>;
	}
	pub type LocationId<T> = BoundedVec<u8, <T as Config>::PostcodeLimit>;

    pub type RegionId = u32;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

    /// Mapping of region to the region information.
	#[pallet::storage]
	pub type Regions<T: Config> = 
		StorageMap<_, Blake2_128Concat, RegionId, RegionInfo<T>, OptionQuery>;

    /// Id of the next region.
	#[pallet::storage]
	pub(super) type NextRegionId<T: Config> = StorageValue<_, RegionId, ValueQuery>;

	/// Mapping of region to requests for takeover.
	#[pallet::storage]
	pub type TakeoverRequests<T: Config> =
		StorageMap<_, Blake2_128Concat, RegionId, T::AccountId, OptionQuery>;

	/// True if a location is registered.
	#[pallet::storage]
	pub type LocationRegistration<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		RegionId,
		Blake2_128Concat,
		LocationId<T>,
		bool,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// New region has been created.
		RegionCreated { region_id: u32, collection_id: <T as pallet::Config>::NftCollectionId, owner: T::AccountId, listing_duration: BlockNumberFor<T>, tax: Permill },
		/// Someone proposed to take over a region.
		TakeoverProposed { region: RegionId, proposer: T::AccountId },
		/// A takeover has been accepted from the region owner.
		TakeoverAccepted { region: RegionId, new_owner: T::AccountId },
		/// A takeover has been rejected from the region owner.
		TakeoverRejected { region: RegionId },
		/// A Takeover has been cancelled.
		TakeoverCancelled { region: RegionId, signer:T::AccountId },
		/// Listing duration of a region changed.
		ListingDurationChanged { region: RegionId, listing_duration: BlockNumberFor<T> },
		/// Tax of a region changed.
		RegionTaxChanged { region: RegionId, new_tax: Permill },
		/// New location has been created.
		LocationCreated { region_id: u32, location_id: LocationId<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
        /// User did not pass the kyc.
		UserNotWhitelisted,
        /// The duration of a listing can not be zero.
		ListingDurationCantBeZero,
        /// Listing limit is set too high.
		ListingDurationTooHigh,
        ArithmeticOverflow,
		/// This Region is not known.
		RegionUnknown,
		/// No sufficient permission.
		NoPermission,
		/// The proposer is already owner of this region.
		AlreadyRegionOwner,
		/// There is already a takeover request pending.
		TakeoverAlreadyPending,
		/// There is no pending takeover request.
		NoTakeoverRequest,
		/// The location is already registered.
		LocationRegistered,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
        /// Creates a new region for the marketplace.
		/// This function calls the nfts-pallet to create a new collection.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_duration`: Duration of a listing in this region.
		/// - `tax`: Tax percentage for selling a property in this region.
		///
		/// Emits `RegionCreated` event when succesfful.
        #[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn create_new_region(origin: OriginFor<T>, listing_duration: BlockNumberFor<T>, tax: Permill) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(!listing_duration.is_zero(), Error::<T>::ListingDurationCantBeZero);
			ensure!(listing_duration <= T::MaxListingDuration::get(), Error::<T>::ListingDurationTooHigh);
			T::NativeCurrency::hold(&HoldReason::RegionDepositReserve.into(), &signer, T::RegionDeposit::get())?;
			
			let pallet_id: T::AccountId = Self::account_id();
			let collection_id = <T as pallet::Config>::Nfts::create_collection(
				&pallet_id, 
				&pallet_id, 
				&Self::default_collection_config(),
			)?;

			let current_region_id = NextRegionId::<T>::get();
			let next_region_id = current_region_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
			
			let region_info = RegionInfo {
				collection_id,
				listing_duration,
				owner: signer.clone(),
				tax,
			};
			Regions::<T>::insert(current_region_id, region_info);
			NextRegionId::<T>::put(next_region_id);
			
			Self::deposit_event(Event::<T>::RegionCreated { 
				region_id: current_region_id, 
				collection_id,
				owner: signer,
				listing_duration,
				tax, 
			});
			Ok(())
		}

		/// Region owner can adjust the listing duration.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: Region in where the listing duration should be changed.
		/// - `listing_duration`: New duration of a listing in this region.
		///
		/// Emits `ListingDurationChanged` event when succesfful.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn adjust_listing_duration(origin: OriginFor<T>, region: RegionId, listing_duration: BlockNumberFor<T>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);

			Regions::<T>::try_mutate(region, |maybe_region| {
				let region = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
				ensure!(signer == region.owner, Error::<T>::NoPermission);

				ensure!(!listing_duration.is_zero(), Error::<T>::ListingDurationCantBeZero);
				ensure!(listing_duration <= T::MaxListingDuration::get(), Error::<T>::ListingDurationTooHigh);
			
				region.listing_duration = listing_duration;
				Ok::<(), DispatchError>(())
			})?;

			Self::deposit_event(Event::<T>::ListingDurationChanged { region, listing_duration });
			Ok(())
		}

		/// Region owner can adjust the tax in a region.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: Region in where the tax should be changed.
		/// - `tax`: New tax for a property sell in this region.
		///
		/// Emits `RegionTaxChanged` event when succesfful.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn adjust_region_tax(origin: OriginFor<T>, region: RegionId, new_tax: Permill) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);

			Regions::<T>::try_mutate(region, |maybe_region| {
				let region = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
				ensure!(region.owner == signer, Error::<T>::NoPermission);

				region.tax = new_tax;
				Ok::<(), DispatchError>(())
			})?;

			Self::deposit_event(Event::<T>::RegionTaxChanged { region, new_tax });
			Ok(())
		}

		/// Caller proposes to become new owner of a region.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: Region which the caller wants to own.
		///
		/// Emits `TakeoverProposed` event when succesfful.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn propose_region_takeover(origin: OriginFor<T>, region: RegionId) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let region_info = Regions::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
		
			ensure!(signer != region_info.owner, Error::<T>::AlreadyRegionOwner);
			ensure!(!TakeoverRequests::<T>::contains_key(region), Error::<T>::TakeoverAlreadyPending);
		
			T::NativeCurrency::hold(
				&HoldReason::RegionDepositReserve.into(),
				&signer,
				T::RegionDeposit::get(),
			)?;
		
			TakeoverRequests::<T>::insert(region, signer.clone());
			Self::deposit_event(Event::<T>::TakeoverProposed { region, proposer: signer });
			Ok(())
		}

		/// The region owner can handle the takeover request.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: Region which the caller wants to own.
		/// - `action`: Enum for takeover which is either Accept or Reject.
		///
		/// Emits `TakeoverRejected` event when succesfful.
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn handle_takeover(
			origin: OriginFor<T>,
			region: RegionId,
			action: TakeoverAction,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let mut region_info = Regions::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			ensure!(signer == region_info.owner, Error::<T>::NoPermission);

			let requester = TakeoverRequests::<T>::take(region).ok_or(Error::<T>::NoTakeoverRequest)?;

			match action {
				TakeoverAction::Accept => {	
					T::NativeCurrency::release(
						&HoldReason::RegionDepositReserve.into(),
						&region_info.owner,
						T::RegionDeposit::get(),
						Precision::Exact,
					)?;
	
					region_info.owner = requester.clone();
					Regions::<T>::insert(region, region_info);
	
					Self::deposit_event(Event::<T>::TakeoverAccepted { region, new_owner: requester });
				},
				TakeoverAction::Reject => {
					T::NativeCurrency::release(
						&HoldReason::RegionDepositReserve.into(),
						&requester,
						T::RegionDeposit::get(),
						Precision::Exact,
					)?;
	
					Self::deposit_event(Event::<T>::TakeoverRejected { region });
				},
			}
			Ok(())
		}

		/// The proposer of a takeover can cancel the request.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: Region in which the caller wants to cancel the request.
		///
		/// Emits `TakeoverCancelled` event when succesfful.
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn cancel_region_takeover(origin: OriginFor<T>, region: RegionId) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let requester = TakeoverRequests::<T>::take(region).ok_or(Error::<T>::NoTakeoverRequest)?;
			ensure!(requester == signer, Error::<T>::NoPermission);
		
			T::NativeCurrency::release(
				&HoldReason::RegionDepositReserve.into(),
				&signer,
				T::RegionDeposit::get(),
				Precision::Exact,
			)?;
		
			Self::deposit_event(Event::<T>::TakeoverCancelled { region, signer });
			Ok(())
		}

		/// Creates a new location for a region.
		///
		/// The origin must be the LocationOrigin.
		///
		/// Parameters:
		/// - `region`: The region where the new location should be created.
		/// - `location`: The postcode of the new location.
		///
		/// Emits `LocationCreated` event when succesfful.
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn create_new_location(
			origin: OriginFor<T>,
			region: RegionId,
			location: LocationId<T>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(Regions::<T>::contains_key(region), Error::<T>::RegionUnknown);
			ensure!(
				!LocationRegistration::<T>::contains_key(region, &location),
				Error::<T>::LocationRegistered
			);
			T::NativeCurrency::hold(&HoldReason::LocationDepositReserve.into(), &signer, T::LocationDeposit::get())?;
			LocationRegistration::<T>::insert(region, &location, true);
			Self::deposit_event(Event::<T>::LocationCreated {
				region_id: region,
				location_id: location,
			});
			Ok(())
		}
	}

    impl<T: Config> Pallet<T> {
		/// Get the account id of the pallet
		pub fn account_id() -> T::AccountId {
			<T as pallet::Config>::PalletId::get().into_account_truncating()
		}

		/// Set the default collection configuration for creating a collection.
		fn default_collection_config() -> CollectionConfig<
			T::Balance,
			BlockNumberFor<T>,
			<T as pallet_nfts::Config>::CollectionId,
		> {
			Self::collection_config_with_all_settings_enabled()
		}

		fn collection_config_with_all_settings_enabled() -> CollectionConfig<
			T::Balance,
			BlockNumberFor<T>,
			<T as pallet_nfts::Config>::CollectionId,
		> {
			CollectionConfig {
				settings: CollectionSettings::all_enabled(),
				max_supply: None,
				mint_settings: MintSettings::default(),
			}
		}
    }
}

