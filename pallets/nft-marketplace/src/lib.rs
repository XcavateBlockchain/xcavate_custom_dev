#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::{
	traits::{
		tokens::{fungible, fungibles, nonfungibles_v2, Precision, WithdrawConsequence},
		fungible::{Mutate, MutateHold, Inspect},	
		fungibles::Mutate as FungiblesMutate,
		fungibles::Inspect as FungiblesInspect,
		fungibles::{InspectFreeze, MutateFreeze},
		nonfungibles_v2::Mutate as NonfungiblesMutate,
		nonfungibles_v2::{Create, Transfer},
		tokens::Preservation,
	},
	PalletId,
	storage::bounded_btree_map::BoundedBTreeMap,
};

use frame_support::sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedSub, CheckedDiv, CheckedMul, StaticLookup, Zero, One,
	},
	Saturating,
};

use pallet_nfts::{
	CollectionConfig, CollectionSettings, ItemConfig, ItemSettings, MintSettings,
};

use frame_system::RawOrigin;

use codec::Codec;

use primitives::TestId;

use types::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type Balance = u128;

pub type LocalAssetIdOf<T> =
	<<T as Config>::LocalCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

pub type ForeignAssetIdOf<T> =
	<<T as Config>::ForeignCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

type NativeBalance<T> = <<T as Config>::NativeCurrency as fungible::Inspect<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[cfg(feature = "runtime-benchmarks")]
	pub struct NftHelper;

	#[cfg(feature = "runtime-benchmarks")]
	pub trait BenchmarkHelper<AssetId, T> {
		fn to_asset(i: u32) -> AssetId;
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config>
		BenchmarkHelper<FractionalizedAssetId<T>, T> for NftHelper
	{
		fn to_asset(i: u32) -> FractionalizedAssetId<T> {
			i.into()
		}
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Funds are held for operating a region.
		#[codec(index = 0)]
		RegionDepositReserve,
		#[codec(index = 1)]
		LocationDepositReserve,
		#[codec(index = 2)]
		ListingDepositReserve,
	}

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_nfts::Config
		+ pallet_xcavate_whitelist::Config
		+ pallet_nft_fractionalization::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type representing the weight of this pallet.
		type WeightInfo: WeightInfo;

		type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
			+ fungible::Mutate<AccountIdOf<Self>>
			+ fungible::InspectHold<AccountIdOf<Self>, Balance = Balance>
			+ fungible::BalancedHold<AccountIdOf<Self>, Balance = Balance>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId, Reason = <Self as pallet::Config>::RuntimeHoldReason>;

		/// The overarching hold reason.
		type RuntimeHoldReason: From<HoldReason>;

		type LocalCurrency: fungibles::InspectEnumerable<AccountIdOf<Self>, Balance = Balance, AssetId = u32>
			+ fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = Balance>
			+ fungibles::Inspect<AccountIdOf<Self>, Balance = Balance>;

		type ForeignCurrency: fungibles::InspectEnumerable<AccountIdOf<Self>, Balance = Balance, AssetId = u32>
			+ fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = Balance>
			+ fungibles::Inspect<AccountIdOf<Self>, Balance = Balance>;

		type LocalAssetsFreezer: fungibles::MutateFreeze<AccountIdOf<Self>, AssetId = u32, Balance = Balance, Id = TestId>
			+ fungibles::InspectFreeze<AccountIdOf<Self>, AssetId = u32>;

		type ForeignAssetsFreezer: fungibles::MutateFreeze<AccountIdOf<Self>, AssetId = u32, Balance = Balance, Id = TestId>
			+ fungibles::InspectFreeze<AccountIdOf<Self>, AssetId = u32>;
		
		type Nfts: nonfungibles_v2::Inspect<AccountIdOf<Self>, ItemId = <Self as pallet::Config>::NftId,
			CollectionId = <Self as pallet::Config>::NftCollectionId>	
			+ Transfer<Self::AccountId>
			+ nonfungibles_v2::Mutate<AccountIdOf<Self>, ItemConfig>
			+ nonfungibles_v2::Create<AccountIdOf<Self>, CollectionConfig<NativeBalance<Self>, 
			BlockNumberFor<Self>, <Self as pallet_nfts::Config>::CollectionId>>;

		/// The marketplace's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[cfg(feature = "runtime-benchmarks")]
		type Helper: crate::BenchmarkHelper<
			<Self as pallet_assets::Config<Instance1>>::AssetId,
			Self,
		>;

		/// The minimum amount of token of a nft.
		#[pallet::constant]
		type MinNftToken: Get<u32>;

		/// The maximum amount of token of a nft.
		#[pallet::constant]
		type MaxNftToken: Get<u32>;

		/// Origin who can unlock new locations.
		type LocationOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Identifier for the collection of NFT.
		type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy;

		/// The type used to identify an NFT within a collection.
		type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

		/// Collection id type from pallet nft fractionalization.
		type FractionalizeCollectionId: IsType<<Self as pallet_nft_fractionalization::Config>::NftCollectionId>
			+ Parameter
			+ From<<Self as pallet::Config>::NftCollectionId>
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

		/// The Trasury's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type TreasuryId: Get<PalletId>;

		/// The maximum length of data stored in for post codes.
		#[pallet::constant]
		type PostcodeLimit: Get<u32>;

		/// A deposit for listing a property.
		type ListingDeposit: Get<Balance>;

		/// Amount to fund a property account.
		type PropertyAccountFundingAmount: Get<Balance>;

		/// A deposit for operating a region.
		type RegionDeposit: Get<Balance>;

		/// A deposit for operating a location.
		type LocationDeposit: Get<Balance>;

		/// The fee percentage charged by the marketplace (e.g., 1 for 1%).
		type MarketplaceFeePercentage: Get<Balance>;
		
		type MaxListingDuration: Get<BlockNumberFor<Self>>;
	}

	pub type FractionalizedAssetId<T> = <T as Config>::AssetId;
	pub type FractionalizeCollectionId<T> = <T as Config>::FractionalizeCollectionId;
	pub type FractionalizeItemId<T> = <T as Config>::FractionalizeItemId;
	pub type RegionId = u32;
	pub type ListingId = u32;
	pub type LocationId<T> = BoundedVec<u8, <T as Config>::PostcodeLimit>;

	pub(super) type NftListingDetailsType<T> = NftListingDetails<
		<T as pallet::Config>::NftId,
		<T as pallet::Config>::NftCollectionId,
		T,
	>;

	pub(super) type ListingDetailsType<T> = TokenListingDetails<
		<T as pallet::Config>::NftId,
		<T as pallet::Config>::NftCollectionId,
		T,
	>;

	/// Id for the next nft in a collection.
	#[pallet::storage]
	pub(super) type NextNftId<T: Config> =
		StorageMap<_, Blake2_128Concat, <T as pallet::Config>::NftCollectionId, <T as pallet::Config>::NftId, ValueQuery>;

	/// Id of the possible next asset that would be used for
	/// Nft fractionalization.
	#[pallet::storage]
	pub(super) type NextAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Id of the next region.
	#[pallet::storage]
	pub(super) type NextRegionId<T: Config> = StorageValue<_, RegionId, ValueQuery>;

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

	/// The Id for the next token listing.
	#[pallet::storage]
	pub(super) type NextListingId<T: Config> = StorageValue<_, ListingId, ValueQuery>;

	/// Mapping of region to the region information.
	#[pallet::storage]
	pub type Regions<T: Config> = 
		StorageMap<_, Blake2_128Concat, RegionId, RegionInfo<T>, OptionQuery>;

	/// Mapping of region to the owner.
	#[pallet::storage]
	pub type RegionOwner<T: Config> =
		StorageMap<_, Blake2_128Concat, RegionId, AccountIdOf<T>, OptionQuery>;

	/// Mapping of region to the tax.
	#[pallet::storage]
	pub type RegionTax<T: Config> =
		StorageMap<_, Blake2_128Concat, RegionId, Balance, OptionQuery>;

	/// Mapping of region to requests for takeover.
	#[pallet::storage]
	pub type TakeoverRequests<T: Config> =
		StorageMap<_, Blake2_128Concat, RegionId, AccountIdOf<T>, OptionQuery>;

	/// Mapping from the Nft to the Nft details.
	#[pallet::storage]
	pub(super) type RegisteredNftDetails<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		<T as pallet::Config>::NftCollectionId,
		Blake2_128Concat,
		<T as pallet::Config>::NftId,
		NftDetails<T>,
		OptionQuery,
	>;

	/// Mapping from the nft to the ongoing nft listing details.
	#[pallet::storage]
	pub(super) type OngoingObjectListing<T: Config> =
		StorageMap<_, Blake2_128Concat, ListingId, NftListingDetailsType<T>, OptionQuery>;

	/// Mapping of the nft to the amount of listed token.
	#[pallet::storage]
	pub(super) type ListedToken<T: Config> = StorageMap<_, Blake2_128Concat, ListingId, u32, OptionQuery>;

	/// Mapping of the listing to the buyer of the sold token.
	#[pallet::storage]
	pub(super) type TokenBuyer<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ListingId,
		BoundedVec<AccountIdOf<T>, T::MaxNftToken>,
		ValueQuery,
	>;

	/// Double mapping of the account id of the token owner
	/// and the listing to the amount of token.
	#[pallet::storage]
	pub(super) type TokenOwner<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		ListingId,
		TokenOwnerDetails<Balance, T>,
		ValueQuery,
	>;

	/// Mapping of the listing id to the listing details of a token listing.
	#[pallet::storage]
	pub(super) type TokenListings<T: Config> =
		StorageMap<_, Blake2_128Concat, ListingId, ListingDetailsType<T>, OptionQuery>;

	/// Mapping of the assetid to the vector of token holder.
	#[pallet::storage]
	pub type PropertyOwner<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BoundedVec<AccountIdOf<T>, T::MaxNftToken>,
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

	/// Mapping of the assetid to the collectionid and nftid.
	#[pallet::storage]
	pub type AssetIdDetails<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		AssetDetails<<T as pallet::Config>::NftId, <T as pallet::Config>::NftCollectionId, T>,
		OptionQuery,
	>;

	/// Mapping from listing to offer details.
	#[pallet::storage]
	pub(super) type OngoingOffers<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ListingId,
		Blake2_128Concat,
		AccountIdOf<T>,
		OfferDetails<Balance, T>,
		OptionQuery,
	>;

	/// Stores the lawyer info.
	#[pallet::storage]
	pub(super) type RealEstateLawyer<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		bool,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type PropertyLawyer<T: Config> = StorageMap<
		_, 
		Blake2_128Concat,
		ListingId,
		PropertyLawyerDetails<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type RefundToken<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ListingId,
		RefundInfos<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub type ListingDeposits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ListingId,
		(AccountIdOf<T>, Balance),
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new object has been listed on the marketplace.
		ObjectListed {
			collection_index: <T as pallet::Config>::NftCollectionId,
			item_index: <T as pallet::Config>::NftId,
			price: Balance,
			seller: AccountIdOf<T>,
		},
		/// A token has been bought.
		TokenBought { asset_id: u32, buyer: AccountIdOf<T>, price: Balance },
		/// Token from listed object have been bought.
		TokenBoughtObject { asset_id: u32, buyer: AccountIdOf<T>, amount: u32, price: Balance },
		/// Token have been listed.
		TokenListed { asset_id: u32, price: Balance, seller: AccountIdOf<T> },
		/// The price of the token listing has been updated.
		ListingUpdated { listing_index: ListingId, new_price: Balance },
		/// The nft has been delisted.
		ListingDelisted { listing_index: ListingId },
		/// The price of the listed object has been updated.
		ObjectUpdated { listing_index: ListingId, new_price: Balance },
		/// New region has been created.
		RegionCreated { region_id: u32, collection_id: <T as pallet::Config>::NftCollectionId },
		/// New location has been created.
		LocationCreated { region_id: u32, location_id: LocationId<T> },
		/// A new offer has been made.
		OfferCreated { listing_id: ListingId, price: Balance },
		/// An offer has been cancelled.
		OfferCancelled { listing_id: ListingId, account_id: AccountIdOf<T> },
		/// A lawyer has been registered.
		LawyerRegistered { lawyer: AccountIdOf<T> },
		/// A lawyer claimed a property.
		LawyerClaimedProperty { lawyer: AccountIdOf<T>, listing_id: ListingId, legal_side: LegalProperty},
		/// A lawyer stepped back from a legal case.
		LawyerRemovedFromCase { lawyer: AccountIdOf<T>, listing_id: ListingId },
		/// Documents have been approved or rejected.
		DocumentsConfirmed { signer: AccountIdOf<T>, listing_id: ListingId, approve: bool },
		/// The property nft got burned.
		PropertyNftBurned { collection_id: <T as pallet::Config>::NftCollectionId, item_id: <T as pallet::Config>::NftId, asset_id: u32 },
		/// Property token have been send to the investors.
		PropertyTokenSent { listing_id: ListingId, asset_id: u32 },
		/// The property deal has been successfully sold.
		PropertySuccessfullySold { listing_id: ListingId, item_index: <T as pallet::Config>::NftId, asset_id: u32 },
		/// Funds has been withdrawn.
		FundsWithdrawn { signer: AccountIdOf<T>, listing_id: ListingId },
		/// Funds have been refunded after expired listing.
		FundsRefunded { signer: AccountIdOf<T>, listing_id: ListingId },
		/// An offer has been accepted.
		OfferAccepted { listing_id: ListingId, offeror: AccountIdOf<T>, amount: u32, price: Balance },
		/// An offer has been Rejected.
		OfferRejected { listing_id: ListingId, offeror: AccountIdOf<T>, amount: u32, price: Balance },
		/// A buy has been cancelled.
		BuyCancelled { listing_id: ListingId, buyer: AccountIdOf<T>, amount: u32 },
		/// Property token have been sent to another account.
		PropertyTokenSend { asset_id: u32, sender: AccountIdOf<T>, receiver: AccountIdOf<T>, amount: u32 },
		/// The deposit of the real estate developer has been released.
		ListingDepositReleased { signer: AccountIdOf<T>, listing_id: ListingId },
		/// Someone proposed to take over a region.
		TakeoverProposed { region: RegionId, signer:AccountIdOf<T> },
		/// A takeover has been accepted from the region owner.
		TakeoverAccepted { region: RegionId, new_owner:AccountIdOf<T> },
		/// A takeover has been rejected from the region owner.
		TakeoverRejected { region: RegionId },
		/// Listing duration of a region changed.
		ListingDurationChanged { region: RegionId, listing_duration: BlockNumberFor<T> },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// This index is not taken.
		InvalidIndex,
		/// The buyer doesn't have enough funds.
		NotEnoughFunds,
		/// Not enough token available to buy.
		NotEnoughTokenAvailable,
		/// Error by converting a type.
		ConversionError,
		/// Error by dividing a number.
		DivisionError,
		/// Error by multiplying a number.
		MultiplyError,
		/// No sufficient permission.
		NoPermission,
		/// The SPV has already been created.
		SpvAlreadyCreated,
		/// User did not pass the kyc.
		UserNotWhitelisted,
		ArithmeticUnderflow,
		ArithmeticOverflow,
		/// The token is not for sale.
		TokenNotForSale,
		/// The nft has not been registered on the marketplace.
		NftNotFound,
		/// There are already too many token buyer.
		TooManyTokenBuyer,
		/// This Region is not known.
		RegionUnknown,
		/// The location is already registered.
		LocationRegistered,
		/// The location is not registered.
		LocationUnknown,
		/// The object can not be divided in so many token.
		TooManyToken,
		/// The object needs more token.
		TokenAmountTooLow,
		/// A user can only make one offer per listing.
		OnlyOneOfferPerUser,
		/// The lawyer has already been registered.
		LawyerAlreadyRegistered,
		/// The lawyer job has already been taken.
		LawyerJobTaken,
		/// A lawyer has not been set.
		LawyerNotFound,
		/// The lawyer already submitted his answer.
		AlreadyConfirmed,
		/// The costs of the lawyer can't be that high.
		CostsTooHigh,
		/// This Asset is not supported for payment.
		AssetNotSupported,
		ExceedsMaxEntries,
		/// The property is not refunded.
		TokenNotRefunded,
		/// The duration of a listing can not be zero.
		ListingDurationCantBeZero,
		/// The property is already sold.
		PropertyAlreadySold,
		/// Listing has already expired.
		ListingExpired,
		/// Signer has not bought any token.
		NoTokenBought,
		/// The listing has not expired.
		ListingNotExpired,
		/// Price of a token can not be zero.
		InvalidTokenPrice,
		/// Token amount can not be zero.
		AmountCannotBeZero,
		/// Marketplace fee needs to be below 100 %.
		InvalidFeePercentage,
		/// Marketplace tax needs to be below 100 %.
		InvalidTaxPercentage,
		/// The sender has not enough token.
		NotEnoughToken,
		/// Token have not been returned yet.
		TokenNotReturned,
		/// Listing limit is set too high.
		ListingDurationTooHigh,
		/// The proposer is already owner of this region.
		AlreadyRegionOwner,
		/// There is already a takeover request pending.
		TakeoverAlreadyPending,
		/// There is no pending takeover request.
		NoTakeoverRequest,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
 		/// Creates a new region for the marketplace.
		/// This function calls the nfts-pallet to create a new collection.
		///
		/// The origin must be the LocationOrigin.
		///
		/// Parameters:
		/// - `listing_duration`: Duration of a listing in this region.
		///
		/// Emits `RegionCreated` event when succesfful.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_new_region())]
		pub fn create_new_region(origin: OriginFor<T>, listing_duration: BlockNumberFor<T>, tax: Balance) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(!listing_duration.is_zero(), Error::<T>::ListingDurationCantBeZero);
			ensure!(listing_duration <= T::MaxListingDuration::get(), Error::<T>::ListingDurationTooHigh);
			T::NativeCurrency::hold(&HoldReason::RegionDepositReserve.into(), &signer, T::RegionDeposit::get())?;
			
			let pallet_id: AccountIdOf<T> = Self::account_id();
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
				owner: signer,
				tax,
			};
			Regions::<T>::insert(current_region_id, region_info);
			//RegionOwner::<T>::insert(current_region_id, signer);
			RegionTax::<T>::insert(current_region_id, tax);
			NextRegionId::<T>::put(next_region_id);
			
			Self::deposit_event(Event::<T>::RegionCreated { region_id: current_region_id, collection_id });
			Ok(())
		}

		#[pallet::call_index(30)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn adjust_listing_duration(origin: OriginFor<T>, region: RegionId, listing_duration: BlockNumberFor<T>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);

			let mut region_info = Regions::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			ensure!(signer == region_info.owner, Error::<T>::NoPermission);

			ensure!(!listing_duration.is_zero(), Error::<T>::ListingDurationCantBeZero);
			ensure!(listing_duration <= T::MaxListingDuration::get(), Error::<T>::ListingDurationTooHigh);

			region_info.listing_duration = listing_duration;

			Regions::<T>::insert(region, region_info);

			Self::deposit_event(Event::<T>::ListingDurationChanged { region, listing_duration });
			Ok(())
		}

		#[pallet::call_index(31)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn propose_region_takeover(origin: OriginFor<T>, region: RegionId) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let current_owner = RegionOwner::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
		
			ensure!(signer != current_owner, Error::<T>::AlreadyRegionOwner);
			ensure!(!TakeoverRequests::<T>::contains_key(region), Error::<T>::TakeoverAlreadyPending);
		
			T::NativeCurrency::hold(
				&HoldReason::RegionDepositReserve.into(),
				&signer,
				T::RegionDeposit::get(),
			)?;
		
			TakeoverRequests::<T>::insert(region, signer.clone());
			Self::deposit_event(Event::<T>::TakeoverProposed { region, signer });
			Ok(())
		}

		#[pallet::call_index(32)]
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
			let current_owner = RegionOwner::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			ensure!(signer == current_owner, Error::<T>::NoPermission);

			let requester = TakeoverRequests::<T>::take(region).ok_or(Error::<T>::NoTakeoverRequest)?;

			match action {
				TakeoverAction::Accept => {	
					T::NativeCurrency::release(
						&HoldReason::RegionDepositReserve.into(),
						&current_owner,
						T::RegionDeposit::get(),
						Precision::Exact,
					)?;
	
					RegionOwner::<T>::insert(region, requester.clone());
	
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

		/// Creates a new location for a region.
		///
		/// The origin must be the LocationOrigin.
		///
		/// Parameters:
		/// - `region`: The region where the new location should be created.
		/// - `location`: The postcode of the new location.
		///
		/// Emits `LocationCreated` event when succesfful.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_new_location())]
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

		/// List a real estate object. A new nft gets minted.
		/// This function calls the nfts-pallet to mint a new nft and sets the Metadata.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: The region where the object is located.
		/// - `location`: The location where the object is located.
		/// - `token_price`: The price of a single token.
		/// - `token_amount`: The amount of tokens for a object.
		/// - `data`: The Metadata of the nft.
		///
		/// Emits `ObjectListed` event when succesfful
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::list_object())]
		pub fn list_object(
			origin: OriginFor<T>,
			region: RegionId,
			location: LocationId<T>,
			token_price: Balance,
			token_amount: u32,
			data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
		) -> DispatchResult {
			let signer = ensure_signed(origin.clone())?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(token_amount > 0, Error::<T>::AmountCannotBeZero);
			ensure!(token_amount <= T::MaxNftToken::get(), Error::<T>::TooManyToken);
			ensure!(token_amount >= T::MinNftToken::get(), Error::<T>::TokenAmountTooLow);
			ensure!(token_price > 0, Error::<T>::InvalidTokenPrice);

			let region_info = Regions::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			ensure!(
				LocationRegistration::<T>::get(region, location.clone()),
				Error::<T>::LocationUnknown
			);
			let item_id = NextNftId::<T>::get(region_info.collection_id);
			let mut asset_number: u32 = NextAssetId::<T>::get();
			let mut asset_id: LocalAssetIdOf<T> = asset_number;
			while !T::LocalCurrency::total_issuance(asset_id)
				.is_zero()
			{
				asset_number = asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
				asset_id = asset_number;
			}
			let asset_id: FractionalizedAssetId<T> = asset_number.into();
			let listing_id = NextListingId::<T>::get();
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let listing_duration = region_info.listing_duration;
			let listing_expiry =
				current_block_number.saturating_add(listing_duration);

			let mut collected_funds = BoundedBTreeMap::default();
			collected_funds.try_insert(PaymentAssets::USDC, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?;
			collected_funds.try_insert(PaymentAssets::USDT, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?; 
			
			// Calculate listing deposit
			let property_price = token_price
				.checked_mul(token_amount as u128)
				.ok_or(Error::<T>::MultiplyError)?;
			let deposit_amount = property_price
				.checked_mul(T::ListingDeposit::get())
				.ok_or(Error::<T>::MultiplyError)?
				.checked_div(100)
				.ok_or(Error::<T>::DivisionError)?;

			// Check signer balance before doing anything
			match T::NativeCurrency::can_withdraw(&signer, deposit_amount) {
				WithdrawConsequence::Success => {},
				_ => return Err(Error::<T>::NotEnoughFunds.into()),
			}

			let property_account = Self::property_account_id(asset_number);
			T::NativeCurrency::transfer(
				&signer,
				&property_account,
				T::PropertyAccountFundingAmount::get(),
				Preservation::Expendable
			)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;

			let pallet_account = Self::account_id();
			<T as pallet::Config>::Nfts::mint_into(
				&region_info.collection_id,
				&item_id,
				&property_account.clone(),
				&Self::default_item_config(),
				true
			)?;
			<T as pallet::Config>::Nfts::set_item_metadata(
				Some(&pallet_account),
				&region_info.collection_id,
				&item_id,
				&data,
			)?;

			// Register the NFT metadata
			RegisteredNftDetails::<T>::insert(
				region_info.collection_id,
				item_id,
				NftDetails {
					spv_created: false,
					asset_id: asset_number,
					region,
					location: location.clone(),
				},
			);

			let nft = NftListingDetails {
				real_estate_developer: signer.clone(),
				token_price,
				collected_funds: collected_funds.clone(),
				collected_tax: collected_funds.clone(),
				collected_fees: collected_funds,
				asset_id: asset_number,
				item_id,
				collection_id: region_info.collection_id,
				token_amount,
				listing_expiry,
			};
			OngoingObjectListing::<T>::insert(listing_id, nft);
			ListedToken::<T>::insert(listing_id, token_amount);

			// Fractionalize NFT
			let property_origin: OriginFor<T> = RawOrigin::Signed(property_account.clone()).into();
			let user_lookup = <T::Lookup as StaticLookup>::unlookup(property_account.clone());
			let fractionalize_collection_id = FractionalizeCollectionId::<T>::from(region_info.collection_id);
			let fractionalize_item_id = FractionalizeItemId::<T>::from(item_id);

   			pallet_nft_fractionalization::Pallet::<T>::fractionalize(
				property_origin.clone(),
				fractionalize_collection_id.into(),
				fractionalize_item_id.into(),
				asset_id.into(),
				user_lookup,
				token_amount.into(),
			)?;   

			T::NativeCurrency::hold(&HoldReason::ListingDepositReserve.into(), &signer, deposit_amount)?;
			
			ListingDeposits::<T>::insert(listing_id, (signer.clone(), deposit_amount));

			// Store asset details
			AssetIdDetails::<T>::insert(
				asset_number,
				AssetDetails {
					collection_id: region_info.collection_id,
					item_id,
					region,
					location,
					price: property_price,
					token_amount,
				},
			);
			let next_item_id = item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
			asset_number = asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
			let next_listing_id = Self::next_listing_id(listing_id)?;

			NextNftId::<T>::insert(region_info.collection_id, next_item_id);
			NextAssetId::<T>::put(asset_number);			
			NextListingId::<T>::put(next_listing_id);

			Self::deposit_event(Event::<T>::ObjectListed {
				collection_index: region_info.collection_id,
				item_index: item_id,
				price: token_price,
				seller: signer,
			});
			Ok(())
		}

		/// Buy listed token from the marketplace.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy token from.
		/// - `amount`: The amount of token that the investor wants to buy.
		/// - `payment_asset`: Asset in which the investor wants to pay.
		///
		/// Emits `TokenBoughtObject` event when succesfful.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::buy_token())]
		pub fn buy_token(origin: OriginFor<T>, listing_id: ListingId, amount: u32, payment_asset: PaymentAssets) -> DispatchResult {
			let signer = ensure_signed(origin.clone())?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);

			ListedToken::<T>::try_mutate_exists(listing_id, |maybe_listed_token| {
				let listed_token = maybe_listed_token.as_mut().ok_or(Error::<T>::TokenNotForSale)?;
				ensure!(*listed_token >= amount, Error::<T>::NotEnoughTokenAvailable);
				let mut nft_details =
					OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;

				let registered_details = RegisteredNftDetails::<T>::get(nft_details.collection_id, nft_details.item_id)
					.ok_or(Error::<T>::InvalidIndex)?;
		
				ensure!(!registered_details.spv_created, Error::<T>::SpvAlreadyCreated);

				let current_block_number = <frame_system::Pallet<T>>::block_number();
				ensure!(nft_details.listing_expiry > current_block_number, Error::<T>::ListingExpired);

				let transfer_price = nft_details
					.token_price
					.checked_mul(amount as u128)
					.ok_or(Error::<T>::MultiplyError)?;

				let fee_percent = T::MarketplaceFeePercentage::get();
				ensure!(fee_percent < 100, Error::<T>::InvalidFeePercentage);
				let tax_percent = RegionTax::<T>::get(registered_details.region).ok_or(Error::<T>::RegionUnknown)?;
				ensure!(tax_percent < 100, Error::<T>::InvalidTaxPercentage);

				let fee = transfer_price
 					.checked_mul(fee_percent)
					.ok_or(Error::<T>::MultiplyError)?
					.checked_div(100) 
					.ok_or(Error::<T>::DivisionError)?;
				
				let tax = transfer_price
 					.checked_mul(tax_percent)
					.ok_or(Error::<T>::MultiplyError)?
					.checked_div(100) 
					.ok_or(Error::<T>::DivisionError)?;
				
				let total_transfer_price = transfer_price
					.checked_add(fee)
					.ok_or(Error::<T>::ArithmeticOverflow)?
					.checked_add(tax)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				let frozen_balance = T::ForeignAssetsFreezer::balance_frozen(payment_asset.id(), &TestId::Marketplace, &signer);
				let balance = T::ForeignCurrency::balance(payment_asset.id(), &signer);
				let available_balance = balance.checked_sub(frozen_balance).ok_or(Error::<T>::ArithmeticUnderflow)?;
				ensure!(available_balance >= total_transfer_price, Error::<T>::NotEnoughFunds);

				let new_frozen_balance = frozen_balance.checked_add(total_transfer_price).ok_or(Error::<T>::ArithmeticOverflow)?;
				T::ForeignAssetsFreezer::set_freeze(payment_asset.id(), &TestId::Marketplace, &signer, new_frozen_balance)?;
				*listed_token =
					listed_token.checked_sub(amount).ok_or(Error::<T>::ArithmeticUnderflow)?;

				TokenBuyer::<T>::try_mutate(listing_id, |buyers| {
					if !buyers.contains(&signer) {
						buyers.try_push(signer.clone()).map_err(|_| Error::<T>::TooManyTokenBuyer)?;
					}
					Ok::<(), DispatchError>(())
				})?;
				
				TokenOwner::<T>::try_mutate_exists(signer.clone(), listing_id, |maybe_token_owner_details| {
					let mut initial_funds = BoundedBTreeMap::default();
					initial_funds.try_insert(PaymentAssets::USDC, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					initial_funds.try_insert(PaymentAssets::USDT, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?; 
					let token_owner_details = maybe_token_owner_details.get_or_insert( TokenOwnerDetails {
						token_amount: 0,
						paid_funds: initial_funds.clone(),
						paid_tax: initial_funds,
					});
					token_owner_details.token_amount = token_owner_details.token_amount
						.checked_add(amount)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
						
					for (map, value) in [
						(&mut token_owner_details.paid_funds, transfer_price),
						(&mut token_owner_details.paid_tax, tax),
					] {
						match map.get_mut(&payment_asset) {
							Some(existing) => {
								*existing = existing.checked_add(value).ok_or(Error::<T>::ArithmeticOverflow)?;
								Ok(())
							}
							None => {
								map.try_insert(payment_asset.clone(), value)
									.map(|_| ()) // <- Convert Option<u128> to ()
									.map_err(|_| Error::<T>::ExceedsMaxEntries)
							}
						}?
					}

					Ok::<(), DispatchError>(())
				})?;
				for (map, value) in [
					(&mut nft_details.collected_funds, transfer_price),
					(&mut nft_details.collected_tax, tax),
					(&mut nft_details.collected_fees, fee),
				] {
					match map.get_mut(&payment_asset) {
						Some(existing) => *existing = existing.checked_add(value).ok_or(Error::<T>::ArithmeticOverflow)?,
						None => map.try_insert(payment_asset.clone(), value).map(|_| ()).map_err(|_| Error::<T>::ExceedsMaxEntries)?,
					}
				}	
				let asset_id = nft_details.asset_id;
				OngoingObjectListing::<T>::insert(listing_id, &nft_details);
				let mut initial_funds = BoundedBTreeMap::default();
				initial_funds.try_insert(PaymentAssets::USDC, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?;
				initial_funds.try_insert(PaymentAssets::USDT, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?; 
				if *listed_token == 0 {
					let property_lawyer_details = PropertyLawyerDetails {
						real_estate_developer_lawyer: None,
						spv_lawyer: None,
						real_estate_developer_status: DocumentStatus::Pending,
						spv_status: DocumentStatus::Pending,
						real_estate_developer_lawyer_costs: initial_funds.clone(),
						spv_lawyer_costs: initial_funds,
						second_attempt: false,
					};
					Self::token_distribution(listing_id, nft_details, &[PaymentAssets::USDT, PaymentAssets::USDC])?;
					PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
					*maybe_listed_token = None;
				} 
				Self::deposit_event(Event::<T>::TokenBoughtObject {
					asset_id,
					buyer: signer.clone(),
					amount,
					price: transfer_price,
				});
				Ok::<(), DispatchError>(())
			})?;
			Ok(())
		}

		/// Relist token on the marketplace.
		/// The nft must be registered on the marketplace.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `region`: The region where the object is located.
		/// - `item_id`: The item id of the nft.
		/// - `token_price`: The price of a single token.
		/// - `amount`: The amount of token of the real estate object that should be listed.
		///
		/// Emits `TokenListed` event when succesfful
		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::relist_token())]
		pub fn relist_token(
			origin: OriginFor<T>,
			region: RegionId,
			item_id: <T as pallet::Config>::NftId,
			token_price: Balance,
			amount: u32,
		) -> DispatchResult {
			let signer = ensure_signed(origin.clone())?;

			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			ensure!(token_price > 0, Error::<T>::InvalidTokenPrice);

			let region_info = Regions::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			let collection_id = region_info.collection_id;

			let nft_details = RegisteredNftDetails::<T>::get(collection_id, item_id)
				.ok_or(Error::<T>::NftNotFound)?;
			ensure!(
				LocationRegistration::<T>::get(region, nft_details.location),
				Error::<T>::LocationUnknown
			);
			let token_amount: Balance = amount.into();
			let listing_id = NextListingId::<T>::get();

			let frozen_balance = T::LocalAssetsFreezer::balance_frozen(nft_details.asset_id, &TestId::Marketplace, &signer);
			let balance = T::LocalCurrency::balance(nft_details.asset_id, &signer);
			let available_balance = balance.checked_sub(frozen_balance).ok_or(Error::<T>::ArithmeticUnderflow)?;
			ensure!(available_balance >= token_amount, Error::<T>::NotEnoughFunds);
			let new_frozen_balance = frozen_balance.checked_add(token_amount).ok_or(Error::<T>::ArithmeticOverflow)?;
			T::LocalAssetsFreezer::set_freeze(nft_details.asset_id, &TestId::Marketplace, &signer, new_frozen_balance)?;
			
			let token_listing = TokenListingDetails {
				seller: signer.clone(),
				token_price,
				asset_id: nft_details.asset_id,
				item_id,
				collection_id,
				amount,
			};
			TokenListings::<T>::insert(listing_id, token_listing);
			let next_listing_id = Self::next_listing_id(listing_id)?;
			NextListingId::<T>::put(next_listing_id);

			Self::deposit_event(Event::<T>::TokenListed {
				asset_id: nft_details.asset_id,
				price: token_price,
				seller: signer,
			});
			Ok(())
		}

		/// Buy token from the marketplace.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		/// - `amount`: The amount of token the investor wants to buy.
		/// - `payment_asset`: Asset in which the investor wants to pay.
		///
		/// Emits `TokenBought` event when succesfful.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::buy_relisted_token())]
		pub fn buy_relisted_token(
			origin: OriginFor<T>,
			listing_id: ListingId,
			amount: u32,
			payment_asset: PaymentAssets,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(&buyer),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			let listing_details =
				TokenListings::<T>::take(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
			ensure!(listing_details.amount >= amount, Error::<T>::NotEnoughTokenAvailable);
			let price = listing_details
				.token_price
				.checked_mul(amount.into())
				.ok_or(Error::<T>::MultiplyError)?;
			Self::buying_token_process(
				listing_id,
				&buyer.clone(),
				buyer,
				listing_details,
				price,
				amount,
				payment_asset,
			)?;
			Ok(())
		}

		/// Lets a investor cancel the property token purchase.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		///
		/// Emits `BuyCancelled` event when succesfful.
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn cancel_buy(
			origin: OriginFor<T>,
			listing_id: ListingId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let mut nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			ensure!(nft_details.listing_expiry > <frame_system::Pallet<T>>::block_number(), Error::<T>::ListingExpired);
			ensure!(PropertyLawyer::<T>::get(listing_id).is_none(), Error::<T>::PropertyAlreadySold);

			let token_details: TokenOwnerDetails<Balance, T> = TokenOwner::<T>::take(signer.clone(), listing_id);
			ensure!(!token_details.token_amount.is_zero(), Error::<T>::NoTokenBought);
			
			// Process refunds
			Self::unfreeze_token(&mut nft_details, &token_details, &signer)?;

			ListedToken::<T>::try_mutate(listing_id, |maybe_listed_token| {
				let listed_token = maybe_listed_token.as_mut().ok_or(Error::<T>::TokenNotForSale)?;
				*listed_token =
					listed_token.checked_add(token_details.token_amount).ok_or(Error::<T>::ArithmeticOverflow)?;
				Ok::<(), DispatchError>(())
			})?;
			TokenBuyer::<T>::try_mutate(nft_details.asset_id, |buyer_list| {
				let index = buyer_list
					.iter()
					.position(|x| x == &signer)
					.ok_or(Error::<T>::InvalidIndex)?;
				buyer_list.remove(index);
				Ok::<(), DispatchError>(())
			})?;
			OngoingObjectListing::<T>::insert(listing_id, &nft_details);

			Self::deposit_event(Event::<T>::BuyCancelled {
				listing_id,
				buyer: signer,
				amount: token_details.token_amount,
			});
			Ok(())
		}


		/// Created an offer for a token listing.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		/// - `offer_price`: The offer price for token that are offered.
		/// - `amount`: The amount of token that the investor wants to buy.
		/// - `payment_asset`: Asset in which the investor wants to pay.
		///
		/// Emits `OfferCreated` event when succesfful.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::make_offer())]
		pub fn make_offer(
			origin: OriginFor<T>,
			listing_id: ListingId,
			offer_price: Balance,
			amount: u32,
			payment_asset: PaymentAssets,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(OngoingOffers::<T>::get(listing_id, signer.clone()).is_none(), Error::<T>::OnlyOneOfferPerUser);
			let listing_details =
				TokenListings::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
			ensure!(listing_details.amount >= amount, Error::<T>::NotEnoughTokenAvailable);
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			ensure!(offer_price > 0, Error::<T>::InvalidTokenPrice);
			let price = offer_price
				.checked_mul(amount as u128)
				.ok_or(Error::<T>::MultiplyError)?;
			let frozen_balance = T::ForeignAssetsFreezer::balance_frozen(payment_asset.id(), &TestId::Marketplace, &signer);
			let balance = T::ForeignCurrency::balance(payment_asset.id(), &signer);
			let available_balance = balance.checked_sub(frozen_balance).ok_or(Error::<T>::ArithmeticUnderflow)?;
			ensure!(available_balance >= price, Error::<T>::NotEnoughFunds);

			let new_frozen_balance = frozen_balance.checked_add(price).ok_or(Error::<T>::ArithmeticOverflow)?;
			T::ForeignAssetsFreezer::set_freeze(payment_asset.id(), &TestId::Marketplace, &signer, new_frozen_balance)?;
			let offer_details = OfferDetails { buyer: signer.clone(), token_price: offer_price, amount, payment_assets: payment_asset };
			OngoingOffers::<T>::insert(listing_id, signer, offer_details);
			Self::deposit_event(Event::<T>::OfferCreated { listing_id, price: offer_price });
			Ok(())
		}

		/// Lets the investor handle an offer.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		/// - `offeror`: AccountId of the person that the seller wants to handle the offer from.
		/// - `offer`: Enum for offer which is either Accept or Reject.
		///
		/// Emits `OfferAccepted` event when offer gets accepted succesffully.
		/// Emits `OfferRejected` event when offer gets rejected succesffully.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::handle_offer())]
		pub fn handle_offer(
			origin: OriginFor<T>,
			listing_id: ListingId,
			offeror: AccountIdOf<T>,
			offer: Offer,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let listing_details =
				TokenListings::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
			ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
			let offer_details =
				OngoingOffers::<T>::take(listing_id, offeror).ok_or(Error::<T>::InvalidIndex)?;
			ensure!(listing_details.amount >= offer_details.amount, Error::<T>::NotEnoughTokenAvailable);
			let price = offer_details.get_total_amount()?;
			let frozen_balance = T::ForeignAssetsFreezer::balance_frozen(offer_details.payment_assets.id(), &TestId::Marketplace, &offer_details.buyer);
			let new_frozen_balance = frozen_balance.checked_sub(price).ok_or(Error::<T>::ArithmeticOverflow)?;
			T::ForeignAssetsFreezer::set_freeze(offer_details.payment_assets.id(), &TestId::Marketplace, &offer_details.buyer, new_frozen_balance)?;
			match offer {
				Offer::Accept => {
					Self::buying_token_process(
						listing_id,
						&offer_details.buyer,
						offer_details.buyer.clone(),
						listing_details,
						price,
						offer_details.amount,
						offer_details.payment_assets,
					)?;
					Self::deposit_event(Event::<T>::OfferAccepted {
						listing_id,
						offeror: offer_details.buyer,
						amount: offer_details.amount,
						price,
					});
				}
				Offer::Reject => {
					Self::deposit_event(Event::<T>::OfferRejected {
						listing_id,
						offeror: offer_details.buyer,
						amount: offer_details.amount,
						price,
					});
				}
			}
			Ok(())
		} 

		/// Lets the investor cancel an offer.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		///
		/// Emits `OfferCancelled` event when succesfful.
		#[pallet::call_index(9)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_offer())]
		pub fn cancel_offer(
			origin: OriginFor<T>,
			listing_id: ListingId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let offer_details =
				OngoingOffers::<T>::take(listing_id, signer.clone()).ok_or(Error::<T>::InvalidIndex)?;
			ensure!(offer_details.buyer == signer.clone(), Error::<T>::NoPermission);
			let price = offer_details.get_total_amount()?;
			let frozen_balance = T::ForeignAssetsFreezer::balance_frozen(offer_details.payment_assets.id(), &TestId::Marketplace, &offer_details.buyer);
			let new_frozen_balance = frozen_balance.checked_sub(price).ok_or(Error::<T>::ArithmeticOverflow)?;
			T::ForeignAssetsFreezer::set_freeze(offer_details.payment_assets.id(), &TestId::Marketplace, &offer_details.buyer, new_frozen_balance)?;
			Self::deposit_event(Event::<T>::OfferCancelled { listing_id, account_id: signer.clone() });
			Ok(())
		}

		/// Lets the investor withdraw his funds after a property deal was unsuccessful.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		///
		/// Emits `FundsWithdrawn` event when succesfful.
		#[pallet::call_index(10)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn withdraw_funds(
			origin: OriginFor<T>,
			listing_id: ListingId
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let token_details: TokenOwnerDetails<Balance, T> = TokenOwner::<T>::take(signer.clone(), listing_id);
			let nft_details =
					OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let property_account = Self::property_account_id(nft_details.asset_id);
			let token_amount = token_details.token_amount;
			let mut refund_infos = RefundToken::<T>::take(listing_id).ok_or(Error::<T>::TokenNotRefunded)?;
			refund_infos.refund_amount = refund_infos.refund_amount.checked_sub(token_amount).ok_or(Error::<T>::NotEnoughTokenAvailable)?;

			for asset in [PaymentAssets::USDT, PaymentAssets::USDC] {
				if let (Some(paid_funds), Some(paid_tax)) = (
					token_details.paid_funds.get(&asset),
					token_details.paid_tax.get(&asset),
				) {
					if paid_funds.is_zero() || paid_tax.is_zero() {
						continue;
					}

					let refund_amount = paid_funds
						.checked_add(paid_tax)
						.ok_or(Error::<T>::ArithmeticOverflow)?;

					// Transfer USDT funds to owner account
					Self::transfer_funds(
						&property_account,
						&signer,
						refund_amount,
						asset.id(),
					)?;	
				}
			}
			T::LocalCurrency::transfer(
				nft_details.asset_id,
				&signer,
				&property_account,
				token_amount.into(),
				Preservation::Expendable,
			)?;	
			if refund_infos.refund_amount == 0 {
				Self::burn_tokens_and_nfts(listing_id)?;
				Self::refund_investors_with_fees(listing_id, refund_infos.property_lawyer_details)?;
				let (depositor, deposit_amount) = ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
				T::NativeCurrency::release(
					&HoldReason::ListingDepositReserve.into(),
					&depositor,
					deposit_amount,
					Precision::Exact,			
				)?;
				let native_balance = T::NativeCurrency::balance(&property_account);
				if !native_balance.is_zero() {
					T::NativeCurrency::transfer(
						&property_account,
						&nft_details.real_estate_developer,
						native_balance,
						Preservation::Expendable,
					)?;
				}
			} else {
				RefundToken::<T>::insert(listing_id, refund_infos);
			}
			PropertyOwnerToken::<T>::take(nft_details.asset_id, signer.clone());
			Self::deposit_event(Event::<T>::FundsWithdrawn{signer, listing_id});
			Ok(())
		}

		/// Lets the investor unfreeze his funds after a property listing expired.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the investor wants to buy from.
		///
		/// Emits `FundsRefunded` event when succesfful.
		#[pallet::call_index(11)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn refund_expired(
			origin: OriginFor<T>,
			listing_id: ListingId
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let mut nft_details =
            	OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			ensure!(
				nft_details.listing_expiry < <frame_system::Pallet<T>>::block_number(),
				Error::<T>::ListingNotExpired
			);

			ensure!(PropertyLawyer::<T>::get(listing_id).is_none(), Error::<T>::PropertyAlreadySold);

			let token_details = TokenOwner::<T>::take(&signer, listing_id);
			ensure!(
				!token_details.token_amount.is_zero(),
				Error::<T>::NoTokenBought,
			);

			// Process refunds for supported assets (USDT and USDC)
			Self::unfreeze_token(&mut nft_details, &token_details, &signer)?;
			
			// Update ListedToken
			ListedToken::<T>::try_mutate(listing_id, |maybe_listed_token| {
				let listed_token = maybe_listed_token.as_mut().ok_or(Error::<T>::TokenNotForSale)?;
				*listed_token = listed_token
					.checked_add(token_details.token_amount)
					.ok_or(Error::<T>::ArithmeticOverflow)?;
				
				// Check if all tokens are returned
				if *listed_token >= nft_details.token_amount {
					// Listing is over, burn and clean everything
					Self::burn_tokens_and_nfts(listing_id)?;
					let (depositor, deposit_amount) = ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
					T::NativeCurrency::release(
						&HoldReason::ListingDepositReserve.into(),
						&depositor,
						deposit_amount,
						Precision::Exact,			
					)?;
					let property_account = Self::property_account_id(nft_details.asset_id);
					let native_balance = T::NativeCurrency::balance(&property_account);
					if !native_balance.is_zero() {
						T::NativeCurrency::transfer(
							&property_account,
							&nft_details.real_estate_developer,
							native_balance,
							Preservation::Expendable,
						)?;
					}
					OngoingObjectListing::<T>::remove(listing_id);
					ListedToken::<T>::remove(listing_id);
					TokenBuyer::<T>::remove(listing_id);
					*maybe_listed_token = None;
				} else {
					TokenBuyer::<T>::try_mutate(listing_id, |buyers| {
						let index = buyers
							.iter()
							.position(|b| b == &signer)
							.ok_or(Error::<T>::InvalidIndex)?;
						buyers.swap_remove(index); 
						Ok::<(), DispatchError>(())
					})?;
					OngoingObjectListing::<T>::insert(listing_id, &nft_details);
				}
				
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::<T>::FundsRefunded{signer, listing_id});
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn reclaim_unsold(
			origin: OriginFor<T>,
			listing_id: ListingId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			ensure!(nft_details.real_estate_developer == signer, Error::<T>::NoPermission);
			ensure!(
				nft_details.listing_expiry < <frame_system::Pallet<T>>::block_number(),
				Error::<T>::ListingNotExpired
			);

			ensure!(PropertyLawyer::<T>::get(listing_id).is_none(), Error::<T>::PropertyAlreadySold);
			
			// Update ListedToken
			ListedToken::<T>::try_mutate(listing_id, |maybe_listed_token| {
				let listed_token = maybe_listed_token.as_mut().ok_or(Error::<T>::TokenNotForSale)?;
				
				// Check if all tokens are returned
				ensure!(*listed_token >= nft_details.token_amount, Error::<T>::TokenNotReturned);
				// Listing is over, burn and clean everything
				Self::burn_tokens_and_nfts(listing_id)?;
				let (depositor, deposit_amount) = ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
				T::NativeCurrency::release(
					&HoldReason::ListingDepositReserve.into(),
					&depositor,
					deposit_amount,
					Precision::Exact,			
				)?;
				let property_account = Self::property_account_id(nft_details.asset_id);
				let native_balance = T::NativeCurrency::balance(&property_account);
				if !native_balance.is_zero() {
					T::NativeCurrency::transfer(
						&property_account,
						&nft_details.real_estate_developer,
						native_balance,
						Preservation::Expendable,
					)?;
				}
				OngoingObjectListing::<T>::remove(listing_id);
				ListedToken::<T>::remove(listing_id);
				TokenBuyer::<T>::remove(listing_id);
				*maybe_listed_token = None;
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::<T>::ListingDepositReleased {
				signer,
				listing_id,
			});
			Ok(())
		}

		/// Upgrade the price from a listing.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the seller wants to update.
		/// - `new_price`: The new price of the nft.
		///
		/// Emits `ListingUpdated` event when succesfful.
		#[pallet::call_index(13)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upgrade_listing())]
		pub fn upgrade_listing(
			origin: OriginFor<T>,
			listing_id: ListingId,
			new_price: Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			TokenListings::<T>::try_mutate(listing_id, |maybe_listing_details| {
				let listing_details = maybe_listing_details.as_mut().ok_or(Error::<T>::TokenNotForSale)?;
				ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
				listing_details.token_price = new_price;
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::<T>::ListingUpdated {
				listing_index: listing_id,
				new_price,
			});
			Ok(())
		}

		/// Upgrade the price from a listed object.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the seller wants to update.
		/// - `new_price`: The new price of the object.
		///
		/// Emits `ObjectUpdated` event when succesfful.
		#[pallet::call_index(14)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upgrade_object())]
		pub fn upgrade_object(
			origin: OriginFor<T>,
			listing_id: ListingId,
			new_price: Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(ListedToken::<T>::contains_key(listing_id), Error::<T>::TokenNotForSale);
			OngoingObjectListing::<T>::try_mutate(listing_id, |maybe_nft_details| {
				let nft_details = maybe_nft_details.as_mut().ok_or(Error::<T>::InvalidIndex)?;
				ensure!(nft_details.real_estate_developer == signer.clone(), Error::<T>::NoPermission);
				ensure!(
					!RegisteredNftDetails::<T>::get(nft_details.collection_id, nft_details.item_id)
						.ok_or(Error::<T>::InvalidIndex)?
						.spv_created,
					Error::<T>::SpvAlreadyCreated
				);
				nft_details.token_price = new_price;
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::<T>::ObjectUpdated { listing_index: listing_id, new_price });
			Ok(())
		}

		/// Delist the choosen listing from the marketplace.
		/// Works only for relisted token.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing that the seller wants to delist.
		///
		/// Emits `ListingDelisted` event when succesfful.
		#[pallet::call_index(15)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::delist_token())]
		pub fn delist_token(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(signer.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let listing_details =
				TokenListings::<T>::take(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
			ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
			let token_amount = listing_details.amount.into();

			let frozen_balance = T::LocalAssetsFreezer::balance_frozen(listing_details.asset_id, &TestId::Marketplace, &signer);
			let new_frozen_balance = frozen_balance.checked_sub(token_amount).ok_or(Error::<T>::ArithmeticOverflow)?;
			T::ForeignAssetsFreezer::set_freeze(listing_details.asset_id, &TestId::Marketplace, &signer, new_frozen_balance)?;

			Self::deposit_event(Event::<T>::ListingDelisted { listing_index: listing_id });
			Ok(())
		}

		/// Registers a new lawyer.
		///
		/// The origin must be the LocationOrigin.
		///
		/// Parameters:
		/// - `lawyer`: The lawyer that should be registered.
		///
		/// Emits `LawyerRegistered` event when succesfful.
		#[pallet::call_index(16)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn register_lawyer(
			origin: OriginFor<T>,
			lawyer: AccountIdOf<T>,
		) -> DispatchResult {
			T::LocationOrigin::ensure_origin(origin)?;
			ensure!(!RealEstateLawyer::<T>::get(lawyer.clone()), Error::<T>::LawyerAlreadyRegistered);
			RealEstateLawyer::<T>::insert(lawyer.clone(), true);
			Self::deposit_event(Event::<T>::LawyerRegistered {lawyer});
			Ok(())
		}

		/// Lets a lawyer claim a property to handle the legal work.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing from the property.
		/// - `legal_side`: The side that the lawyer wants to represent.
		/// - `costs`: The costs thats the lawyer demands for his work.
		///
		/// Emits `LawyerClaimedProperty` event when succesfful.
		#[pallet::call_index(17)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn lawyer_claim_property(
			origin: OriginFor<T>,
			listing_id: ListingId,
			legal_side: LegalProperty,
			costs: Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(RealEstateLawyer::<T>::get(signer.clone()), Error::<T>::NoPermission);
			let mut property_lawyer_details = PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let collected_fee_usdt = nft_details
				.collected_fees
				.get(&PaymentAssets::USDT)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let collected_fee_usdc = nft_details
				.collected_fees
				.get(&PaymentAssets::USDC)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let collected_fees = collected_fee_usdt
				.checked_add(collected_fee_usdc)
				.ok_or(Error::<T>::ArithmeticOverflow)?;
			ensure!(collected_fees >= costs, Error::<T>::CostsTooHigh);

			match legal_side {
				LegalProperty::RealEstateDeveloperSide => {
					ensure!(property_lawyer_details.real_estate_developer_lawyer.is_none(), Error::<T>::LawyerJobTaken);
					ensure!(property_lawyer_details.spv_lawyer != Some(signer.clone()), Error::<T>::NoPermission);
					property_lawyer_details.real_estate_developer_lawyer = Some(signer.clone());
					if *collected_fee_usdt >= costs {
						property_lawyer_details
							.real_estate_developer_lawyer_costs
							.try_insert(PaymentAssets::USDT, costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;					
					} else if *collected_fee_usdc >= costs {
						property_lawyer_details
							.real_estate_developer_lawyer_costs
							.try_insert(PaymentAssets::USDC, costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					} else {
						let remaining_costs = costs.checked_sub(*collected_fee_usdt).ok_or(Error::<T>::ArithmeticUnderflow)?;
						ensure!(*collected_fee_usdc >= remaining_costs, Error::<T>::CostsTooHigh);
						property_lawyer_details
							.real_estate_developer_lawyer_costs
							.try_insert(PaymentAssets::USDT, costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
						property_lawyer_details
							.real_estate_developer_lawyer_costs
							.try_insert(PaymentAssets::USDC, remaining_costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					}
					PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
				}
				LegalProperty::SpvSide => {
					ensure!(property_lawyer_details.spv_lawyer.is_none(), Error::<T>::LawyerJobTaken);
					ensure!(property_lawyer_details.real_estate_developer_lawyer != Some(signer.clone()), Error::<T>::NoPermission);
					property_lawyer_details.spv_lawyer = Some(signer.clone());
					if *collected_fee_usdt >= costs {
						property_lawyer_details
							.spv_lawyer_costs
							.try_insert(PaymentAssets::USDT, costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;					
					} else if *collected_fee_usdc >= costs {
						property_lawyer_details
							.spv_lawyer_costs
							.try_insert(PaymentAssets::USDC, costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					} else {
						let remaining_costs = costs.checked_sub(*collected_fee_usdt).ok_or(Error::<T>::ArithmeticUnderflow)?;
						ensure!(*collected_fee_usdc >= remaining_costs, Error::<T>::CostsTooHigh);
						property_lawyer_details
							.spv_lawyer_costs
							.try_insert(PaymentAssets::USDT, costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
						property_lawyer_details
							.spv_lawyer_costs
							.try_insert(PaymentAssets::USDC, remaining_costs)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					}
					PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
				}
			}
			Self::deposit_event(Event::<T>::LawyerClaimedProperty {lawyer: signer, listing_id, legal_side});
			Ok(())
		}

		/// Lets a lawyer step back from a case.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing from the property.
		///
		/// Emits `LawyerRemovedFromCase` event when succesfful.
		#[pallet::call_index(18)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn remove_from_case(
			origin: OriginFor<T>,
			listing_id: ListingId,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(RealEstateLawyer::<T>::get(signer.clone()), Error::<T>::NoPermission);
			let mut property_lawyer_details = PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			if property_lawyer_details.real_estate_developer_lawyer == Some(signer.clone()) {
				ensure!(property_lawyer_details.real_estate_developer_status == DocumentStatus::Pending,
					Error::<T>::AlreadyConfirmed);
				property_lawyer_details.real_estate_developer_lawyer = None;
			} else if property_lawyer_details.spv_lawyer == Some(signer.clone()) {
				ensure!(property_lawyer_details.spv_status == DocumentStatus::Pending,
					Error::<T>::AlreadyConfirmed);
				property_lawyer_details.spv_lawyer = None;
			} else {
				return Err(Error::<T>::NoPermission.into());
			}
			PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);	
			Self::deposit_event(Event::<T>::LawyerRemovedFromCase {lawyer: signer, listing_id});	
			Ok(())
		}

		/// Lets a lawyer confirm a legal case.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `listing_id`: The listing from the property.
		/// - `approve`: Approves or Rejects the case.
		///
		/// Emits `DocumentsConfirmed` event when succesfful.
		#[pallet::call_index(19)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn lawyer_confirm_documents(
			origin: OriginFor<T>,
			listing_id: ListingId,
			approve: bool,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;

			let mut property_lawyer_details = PropertyLawyer::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			if property_lawyer_details.real_estate_developer_lawyer == Some(signer.clone()) {
				ensure!(property_lawyer_details.real_estate_developer_status == DocumentStatus::Pending,
					Error::<T>::AlreadyConfirmed);
				property_lawyer_details.real_estate_developer_status = if approve {
					DocumentStatus::Approved
				} else {
					DocumentStatus::Rejected
				};
				Self::deposit_event(Event::<T>::DocumentsConfirmed { signer, listing_id, approve });
			} else if property_lawyer_details.spv_lawyer == Some(signer.clone()) {
				ensure!(property_lawyer_details.spv_status == DocumentStatus::Pending,
					Error::<T>::AlreadyConfirmed);
				property_lawyer_details.spv_status = if approve {
					DocumentStatus::Approved
				} else {
					DocumentStatus::Rejected
				};
				Self::deposit_event(Event::<T>::DocumentsConfirmed {signer, listing_id, approve});
			} else {
				return Err(Error::<T>::NoPermission.into());
			}

			let developer_status = property_lawyer_details.real_estate_developer_status.clone();
			let spv_status = property_lawyer_details.spv_status.clone();

			match (developer_status, spv_status) {
				(DocumentStatus::Approved, DocumentStatus::Approved) => {
 					Self::execute_deal(
						listing_id, 
						property_lawyer_details.clone(),
						&[PaymentAssets::USDT, PaymentAssets::USDC],
					)?; 
					let list = <TokenBuyer<T>>::take(listing_id);
					for owner in list {
						TokenOwner::<T>::take(owner, listing_id);
					}
				}
				(DocumentStatus::Rejected, DocumentStatus::Rejected) => {
					let nft_details = OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
					RefundToken::<T>::insert(listing_id, RefundInfos {
						refund_amount: nft_details.token_amount,
						property_lawyer_details :property_lawyer_details.clone(),
					});
				}
				(DocumentStatus::Approved, DocumentStatus::Rejected) => {
					if !property_lawyer_details.second_attempt {
						property_lawyer_details.spv_status = DocumentStatus::Pending;
						property_lawyer_details.real_estate_developer_status = DocumentStatus::Pending;
						property_lawyer_details.second_attempt = true;
						PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
					} else {
						let nft_details = OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
						RefundToken::<T>::insert(listing_id, RefundInfos {
							refund_amount: nft_details.token_amount,
							property_lawyer_details :property_lawyer_details.clone(),
						});
					}
				}
				(DocumentStatus::Rejected, DocumentStatus::Approved) => {
					if !property_lawyer_details.second_attempt {
						property_lawyer_details.spv_status = DocumentStatus::Pending;
						property_lawyer_details.real_estate_developer_status = DocumentStatus::Pending;
						property_lawyer_details.second_attempt = true;
						PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
					} else {
						let nft_details = OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
						RefundToken::<T>::insert(listing_id, RefundInfos {
							refund_amount: nft_details.token_amount,
							property_lawyer_details :property_lawyer_details.clone(),
						});
					}
				}
				_ => {
					PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
				}
			}
			Ok(())
		} 

		/// Lets the sender send property token to another account.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `asset_id`: The asset id of the property.
		/// - `receiver`: AccountId of the person that the seller wants to handle the offer from.
		/// - `token_amount`: The amount of token the sender wants to send.
		///
		/// Emits `DocumentsConfirmed` event when succesfful.
		#[pallet::call_index(20)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn send_property_token(
			origin: OriginFor<T>,
			asset_id: u32,
			receiver: AccountIdOf<T>,
			token_amount: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(sender.clone()),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(receiver.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let sender_token_amount = PropertyOwnerToken::<T>::take(asset_id, sender.clone());
			let new_sender_token_amount = sender_token_amount.checked_sub(token_amount)
				.ok_or(Error::<T>::NotEnoughToken)?;
			T::LocalCurrency::transfer(
				asset_id,
				&sender,
				&receiver,
				token_amount.into(),
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::NotEnoughToken)?;
			if new_sender_token_amount == 0 {
				let mut owner_list = PropertyOwner::<T>::take(asset_id);
				let index = owner_list
					.iter()
					.position(|x| *x == sender.clone())
					.ok_or(Error::<T>::InvalidIndex)?;
				owner_list.remove(index);
				PropertyOwner::<T>::insert(asset_id, owner_list);
			} else {
				PropertyOwnerToken::<T>::insert(asset_id, sender.clone(), new_sender_token_amount);
			}
			if PropertyOwner::<T>::get(asset_id).contains(&receiver) {
				PropertyOwnerToken::<T>::try_mutate(asset_id, &receiver, |receiver_balance| {
					*receiver_balance = receiver_balance.checked_add(token_amount).ok_or(Error::<T>::ArithmeticOverflow)?;
					Ok::<(), DispatchError>(())
				})?;
			} else {
				PropertyOwner::<T>::try_mutate(asset_id, |owner_list| {
					owner_list.try_push(receiver.clone()).map_err(|_| Error::<T>::TooManyTokenBuyer)?;
					Ok::<(), DispatchError>(())
				})?;
				PropertyOwnerToken::<T>::insert(asset_id, receiver.clone(), token_amount);
			}
			Self::deposit_event(Event::<T>::PropertyTokenSend { 
				asset_id, 
				sender, 
				receiver, 
				amount: token_amount, 
			});
			Ok(())
		}

	}

	impl<T: Config> Pallet<T> {
		/// Get the account id of the pallet
		pub fn account_id() -> AccountIdOf<T> {
			<T as pallet::Config>::PalletId::get().into_account_truncating()
		}

 		pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
			<T as pallet::Config>::PalletId::get().into_sub_account_truncating(("pr", asset_id))
		}

		/// Get the account id of the treasury pallet
		pub fn treasury_account_id() -> AccountIdOf<T> {
			T::TreasuryId::get().into_account_truncating()
		}

		pub fn next_listing_id(listing_id: ListingId) -> Result<ListingId, Error<T>> {
			listing_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)
		}

		/// Sends the token to the new owners and the funds to the real estate developer once all 100 token
		/// of a collection are sold.
		fn execute_deal(listing_id: u32, property_lawyer_details: PropertyLawyerDetails<T>, payment_assets: &[PaymentAssets]) -> DispatchResult {
			let nft_details =
				OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let treasury_id = Self::treasury_account_id();
			let property_account = Self::property_account_id(nft_details.asset_id);

			// Get lawyer accounts
			let real_estate_developer_lawyer_id = property_lawyer_details
				.real_estate_developer_lawyer
				.ok_or(Error::<T>::LawyerNotFound)?;
			let spv_lawyer_id = property_lawyer_details
				.spv_lawyer
				.ok_or(Error::<T>::LawyerNotFound)?;

			// Distribute funds from property account for each asset
			for asset in payment_assets {
				// Get total collected amounts and lawyer costs
				let total_collected_funds = nft_details
					.collected_funds
					.get(asset)
					.ok_or(Error::<T>::AssetNotSupported)?;
				let real_estate_developer_lawyer_costs = property_lawyer_details
					.real_estate_developer_lawyer_costs
					.get(asset)
					.ok_or(Error::<T>::AssetNotSupported)?;
				let spv_lawyer_costs = property_lawyer_details
					.spv_lawyer_costs
					.get(asset)
					.ok_or(Error::<T>::AssetNotSupported)?;
				let tax = nft_details
					.collected_tax
					.get(asset)
					.ok_or(Error::<T>::AssetNotSupported)?;
				let collected_fees = nft_details
					.collected_fees
					.get(asset)
					.ok_or(Error::<T>::AssetNotSupported)?;

				let fee_percentage = T::MarketplaceFeePercentage::get();
				ensure!(fee_percentage <= 100, Error::<T>::InvalidFeePercentage);

				let developer_percentage = 100.checked_sub(&fee_percentage).ok_or(Error::<T>::ArithmeticUnderflow)?;

				// Calculate amounts to distribute
				let developer_amount = total_collected_funds
					.checked_mul(&developer_percentage)
					.ok_or(Error::<T>::MultiplyError)?
					.checked_div(100)
					.ok_or(Error::<T>::DivisionError)?;
				let real_estate_developer_amount = tax
					.checked_add(real_estate_developer_lawyer_costs)
					.ok_or(Error::<T>::ArithmeticOverflow)?;
				let treasury_amount = total_collected_funds
					.checked_div(&100u128)
					.ok_or(Error::<T>::DivisionError)?
					.checked_add(*collected_fees)
					.ok_or(Error::<T>::ArithmeticOverflow)?
					.saturating_sub(*real_estate_developer_lawyer_costs)
					.saturating_sub(*spv_lawyer_costs);

				// Transfer funds from property account
				Self::transfer_funds(
					&property_account,
					&nft_details.real_estate_developer,
					developer_amount,
					asset.id(),
				)?;
				Self::transfer_funds(
					&property_account,
					&real_estate_developer_lawyer_id,
					real_estate_developer_amount,
					asset.id(),
				)?;
				Self::transfer_funds(
					&property_account,
					&spv_lawyer_id,
					*spv_lawyer_costs,
					asset.id(),
				)?;
				Self::transfer_funds(
					&property_account,
					&treasury_id,
					treasury_amount,
					asset.id(),
				)?;
			}

			// Update registered NFT details to mark SPV as created
			let mut registered_nft_details =
					RegisteredNftDetails::<T>::get(nft_details.collection_id, nft_details.item_id)
						.ok_or(Error::<T>::InvalidIndex)?;
			registered_nft_details.spv_created = true;
			RegisteredNftDetails::<T>::insert(
				nft_details.collection_id,
				nft_details.item_id,
				registered_nft_details,
			);
			// Release deposit
			if let Some((depositor, deposit_amount)) = ListingDeposits::<T>::take(listing_id) {
				T::NativeCurrency::release(
					&HoldReason::ListingDepositReserve.into(),
					&depositor,
					deposit_amount,
					Precision::Exact,
				)?;
			}
			Self::deposit_event(Event::<T>::PropertySuccessfullySold{ listing_id, item_index: nft_details.item_id, asset_id: nft_details.asset_id });
			Ok(())
		}

		fn token_distribution(listing_id: u32, nft_details: NftListingDetailsType<T>, payment_assets: &[PaymentAssets]) -> DispatchResult {
			let list = <TokenBuyer<T>>::get(listing_id);		
			let property_account = Self::property_account_id(nft_details.asset_id);
			
			// Process each investor once for all assets and token distribution
			for owner in list {
				let token_details = TokenOwner::<T>::get(owner.clone(), listing_id);

				// Process each payment asset
				for asset in payment_assets {
					if let (Some(paid_funds), Some(paid_tax)) = (
						token_details.paid_funds.get(asset),
						token_details.paid_tax.get(asset),
					) {
						if !paid_funds.is_zero() && !paid_tax.is_zero() {
							let fee_percent = T::MarketplaceFeePercentage::get(); 
							ensure!(fee_percent < 100, Error::<T>::InvalidFeePercentage);
							// Calculate investor's fee (1% of paid_funds)
							let investor_fee = paid_funds
								.checked_mul(&fee_percent)
								.ok_or(Error::<T>::MultiplyError)?
								.checked_div(100)
								.ok_or(Error::<T>::DivisionError)?;

							// Total amount to unfreeze (paid_funds + fee + tax)
							let total_investor_amount = paid_funds
								.checked_add(&investor_fee)
								.ok_or(Error::<T>::ArithmeticOverflow)?
								.checked_add(*paid_tax)
								.ok_or(Error::<T>::ArithmeticOverflow)?;

							// Unfreeze the investor's funds
							let frozen_balance = T::ForeignAssetsFreezer::balance_frozen(
								asset.id(),
								&TestId::Marketplace,
								&owner,
							);
							let new_frozen_balance = frozen_balance
								.checked_sub(total_investor_amount)
								.ok_or(Error::<T>::ArithmeticOverflow)?;
							T::ForeignAssetsFreezer::set_freeze(
								asset.id(),
								&TestId::Marketplace,
								&owner,
								new_frozen_balance,
							)?;

							// Transfer funds to property account
							Self::transfer_funds(
								&owner,
								&property_account,
								total_investor_amount,
								asset.id(),
							)?;
						}
					}
				}

				// Distribute property tokens
				let token_amount = token_details.token_amount.into();

				T::LocalCurrency::transfer(
					nft_details.asset_id,
					&property_account,
					&owner,
					token_amount,
					Preservation::Expendable,
				)?;

				PropertyOwner::<T>::try_mutate(nft_details.asset_id, |keys| {
					keys.try_push(owner.clone())
						.map_err(|_| Error::<T>::TooManyTokenBuyer)?;
					Ok::<(), DispatchError>(())
				})?;
				PropertyOwnerToken::<T>::insert(nft_details.asset_id, owner.clone(), token_details.token_amount);
			}
			Self::deposit_event(Event::<T>::PropertyTokenSent{ listing_id, asset_id: nft_details.asset_id });
			Ok(())
		}

		fn burn_tokens_and_nfts(listing_id: ListingId) -> DispatchResult {
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let pallet_account = Self::property_account_id(nft_details.asset_id);
			let pallet_origin: OriginFor<T> = RawOrigin::Signed(pallet_account.clone()).into();
			let user_lookup = <T::Lookup as StaticLookup>::unlookup(pallet_account);
			let fractionalize_collection_id = FractionalizeCollectionId::<T>::from(nft_details.collection_id);
			let fractionalize_item_id = FractionalizeItemId::<T>::from(nft_details.item_id);
			let fractionalize_asset_id = FractionalizedAssetId::<T>::from(nft_details.asset_id);
 			pallet_nft_fractionalization::Pallet::<T>::unify(
				pallet_origin.clone(),
				fractionalize_collection_id.into(),
				fractionalize_item_id.into(),
				fractionalize_asset_id.into(),
				user_lookup,
			)?;
			<T as pallet::Config>::Nfts::burn(
				&nft_details.collection_id,
				&nft_details.item_id,
				None,
			)?; 
			Self::deposit_event(Event::<T>::PropertyNftBurned { 
				collection_id: nft_details.collection_id, 
				item_id: nft_details.item_id,
				asset_id: nft_details.asset_id, 
			});
			RegisteredNftDetails::<T>::take(nft_details.collection_id, nft_details.item_id)
				.ok_or(Error::<T>::InvalidIndex)?;
			Ok(())
		}

		fn refund_investors_with_fees(listing_id: ListingId, property_lawyer_details: PropertyLawyerDetails<T>) -> DispatchResult {
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let property_account = Self::property_account_id(nft_details.asset_id);
			let treasury_id = Self::treasury_account_id();
			let spv_lawyer_id = property_lawyer_details.spv_lawyer
				.ok_or(Error::<T>::LawyerNotFound)?;

			// Process fees and transfers for each asset
			for asset in [PaymentAssets::USDT, PaymentAssets::USDC] {
				// Fetch fees and lawyer costs
				let fees = nft_details
					.collected_fees
					.get(&asset)
					.ok_or(Error::<T>::AssetNotSupported)?;
				let lawyer_costs = property_lawyer_details
					.spv_lawyer_costs
					.get(&asset)
					.ok_or(Error::<T>::AssetNotSupported)?;

				// Calculate treasury amount
				let treasury_amount = fees
					.checked_sub(lawyer_costs)
					.ok_or(Error::<T>::ArithmeticUnderflow)?;

				// Perform fund transfers
				Self::transfer_funds(&property_account, &treasury_id, treasury_amount, asset.id())?;
				Self::transfer_funds(&property_account, &spv_lawyer_id, *lawyer_costs, asset.id())?;
			}
			PropertyOwner::<T>::take(nft_details.asset_id);
			<TokenBuyer<T>>::take(listing_id);
			Ok(())
		}

		fn buying_token_process(
			listing_id: u32,
			transfer_from: &AccountIdOf<T>,
			account: AccountIdOf<T>,
			mut listing_details: ListingDetailsType<T>,
			price: Balance,
			amount: u32,
			payment_asset: PaymentAssets,
		) -> DispatchResult {
			Self::calculate_fees(price, transfer_from, &listing_details.seller, payment_asset.id())?;
			let token_amount = amount.into();
			let frozen_balance = T::LocalAssetsFreezer::balance_frozen(
				listing_details.asset_id,
				&TestId::Marketplace,
				&listing_details.seller,
			);
			let new_frozen_balance = frozen_balance
				.checked_sub(token_amount)
				.ok_or(Error::<T>::ArithmeticOverflow)?;
			T::LocalAssetsFreezer::set_freeze(
				listing_details.asset_id,
				&TestId::Marketplace,
				&listing_details.seller,
				new_frozen_balance,
			)?;
			T::LocalCurrency::transfer(
				listing_details.asset_id,
				&listing_details.seller,
				&account.clone(),
				token_amount,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;
			let mut seller_amount = PropertyOwnerToken::<T>::take(
				listing_details.asset_id,
				listing_details.seller.clone(),
			);
			seller_amount = seller_amount
				.checked_sub(amount)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			if seller_amount == 0 {
				PropertyOwner::<T>::try_mutate(listing_details.asset_id, |owner_list| {
					let index = owner_list
						.iter()
						.position(|x| *x == listing_details.seller.clone())
						.ok_or(Error::<T>::InvalidIndex)?;
					owner_list.remove(index);
					Ok::<(), DispatchError>(())
				})?;
			} else {
				PropertyOwnerToken::<T>::insert(
					listing_details.asset_id,
					listing_details.seller.clone(),
					seller_amount,
				);
			}
			if PropertyOwner::<T>::get(listing_details.asset_id).contains(&account) {
				let mut buyer_token_amount =
					PropertyOwnerToken::<T>::take(listing_details.asset_id, account.clone());
				buyer_token_amount =
					buyer_token_amount.checked_add(amount).ok_or(Error::<T>::ArithmeticOverflow)?;
				PropertyOwnerToken::<T>::insert(
					listing_details.asset_id,
					account.clone(),
					buyer_token_amount,
				);
			} else {
				PropertyOwner::<T>::try_mutate(listing_details.asset_id, |keys| {
					keys.try_push(account.clone()).map_err(|_| Error::<T>::TooManyTokenBuyer)?;
					Ok::<(), DispatchError>(())
				})?;
				PropertyOwnerToken::<T>::insert(listing_details.asset_id, account.clone(), amount);
			}
			listing_details.amount = listing_details
				.amount
				.checked_sub(amount)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			if listing_details.amount > 0 {
				TokenListings::<T>::insert(listing_id, listing_details.clone());
			}
			Self::deposit_event(Event::<T>::TokenBought {
				asset_id: listing_details.asset_id,
				buyer: account.clone(),
				price: listing_details.token_price,
			});
			Ok(())
		}

		fn unfreeze_token(nft_details: &mut NftListingDetailsType<T>, token_details: &TokenOwnerDetails<Balance, T>, signer: &AccountIdOf<T>) -> DispatchResult {
			for asset in [PaymentAssets::USDT, PaymentAssets::USDC] {
				if let (Some(paid_funds), Some(paid_tax)) = (
					token_details.paid_funds.get(&asset),
					token_details.paid_tax.get(&asset),
				) {
					if paid_funds.is_zero() || paid_tax.is_zero() {
						continue;
					}

					// Calculate refund and investor fee (1% of paid funds)
					let refund_amount = paid_funds
						.checked_add(paid_tax)
						.ok_or(Error::<T>::ArithmeticOverflow)?;
					let investor_fee = paid_funds
						.checked_div(&100)
						.ok_or(Error::<T>::DivisionError)?;
					let total_investor_amount = refund_amount
						.checked_add(investor_fee)
						.ok_or(Error::<T>::ArithmeticOverflow)?;

					// Unfreeze funds
					let frozen_balance = T::ForeignAssetsFreezer::balance_frozen(
						asset.id(),
						&TestId::Marketplace,
						signer,
					);
					let new_frozen_balance = frozen_balance
						.checked_sub(total_investor_amount)
						.ok_or(Error::<T>::ArithmeticUnderflow)?;
					T::ForeignAssetsFreezer::set_freeze(
						asset.id(),
						&TestId::Marketplace,
						signer,
						new_frozen_balance,
					)?;
					if let Some(funds) = nft_details.collected_funds.get_mut(&asset) {
						*funds = funds.checked_sub(*paid_funds).ok_or(Error::<T>::ArithmeticUnderflow)?;
					} 
					if let Some(tax) = nft_details.collected_tax.get_mut(&asset) {
						*tax = tax.checked_sub(*paid_tax).ok_or(Error::<T>::ArithmeticUnderflow)?;
					}
					if let Some(fee) = nft_details.collected_fees.get_mut(&asset) {
						*fee = fee.checked_sub(investor_fee).ok_or(Error::<T>::ArithmeticUnderflow)?;
					}
				}
			}
			Ok(())
		}

		fn calculate_fees(
			price: Balance,
			sender: &AccountIdOf<T>,
			receiver: &AccountIdOf<T>,
			asset: u32,
		) -> DispatchResult {
			let fee_percent = T::MarketplaceFeePercentage::get(); 
			ensure!(fee_percent < 100, Error::<T>::InvalidFeePercentage);

			let fees = price
				.checked_mul(fee_percent)
				.ok_or(Error::<T>::MultiplyError)?
				.checked_div(100u128)
				.ok_or(Error::<T>::DivisionError)?;
			let treasury_id = Self::treasury_account_id();
			let seller_part = price
				.checked_sub(fees)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;

			Self::transfer_funds(sender, &treasury_id, fees, asset)?;
			Self::transfer_funds(sender, receiver, seller_part, asset)?;
			Ok(())
		}

		/// Set the default collection configuration for creating a collection.
		fn default_collection_config() -> CollectionConfig<
			NativeBalance<T>,
			BlockNumberFor<T>,
			<T as pallet_nfts::Config>::CollectionId,
		> {
			Self::collection_config_with_all_settings_enabled()
		}

		fn collection_config_with_all_settings_enabled() -> CollectionConfig<
			NativeBalance<T>,
			BlockNumberFor<T>,
			<T as pallet_nfts::Config>::CollectionId,
		> {
			CollectionConfig {
				settings: CollectionSettings::all_enabled(),
				max_supply: None,
				mint_settings: MintSettings::default(),
			}
		}

		/// Set the default item configuration for minting a nft.
		fn default_item_config() -> ItemConfig {
			ItemConfig { settings: ItemSettings::all_enabled() }
		}

		fn transfer_funds(
			from: &AccountIdOf<T>,
			to: &AccountIdOf<T>,
			amount: Balance,
			asset: u32,
		) -> DispatchResult {
			if !amount.is_zero() {
				T::ForeignCurrency::transfer(asset, from, to, amount, Preservation::Expendable)
					.map_err(|_| Error::<T>::NotEnoughFunds)?;
			}
			Ok(())
		}
	}
}

sp_api::decl_runtime_apis! {
    pub trait NftMarketplaceApi<AccountId> 
	where
		AccountId: Codec
	{
        fn get_marketplace_account_id() -> AccountId;
    }
}