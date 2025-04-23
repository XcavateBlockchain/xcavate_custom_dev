#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::{
	traits::{
		tokens::{fungible, fungibles, nonfungibles_v2},
		fungible::Mutate,	
		fungibles::Mutate as FungiblesMutate,
		fungibles::Inspect as FungiblesInspect,
		fungibles::{InspectFreeze, MutateFreeze},
		nonfungibles_v2::Mutate as NonfungiblesMutate,
		nonfungibles_v2::{Create, Transfer},
		tokens::Preservation,
	},
	PalletId, DefaultNoBound,
	storage::bounded_btree_map::BoundedBTreeMap,
};

use frame_support::sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedSub, CheckedDiv, CheckedMul, StaticLookup, Zero, One,
	},
};

use pallet_nfts::{
	CollectionConfig, CollectionSettings, ItemConfig, ItemSettings, MintSettings,
};

use frame_system::RawOrigin;

use codec::Codec;

use types::TestId;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type Balance = u128;

pub type LocalAssetIdOf<T> =
	<<T as Config>::LocalCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

pub type ForeignAssetIdOf<T> =
	<<T as Config>::ForeignCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

type FrationalizedNftBalanceOf<T> = <T as pallet_nft_fractionalization::Config>::AssetBalance;

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
			+ fungible::BalancedHold<AccountIdOf<Self>, Balance = Balance>;

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

		type AssetsFreezer: fungibles::MutateFreeze<AccountIdOf<Self>, AssetId = u32, Balance = Balance, Id = TestId>
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

		/// The CommunityProjects's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type CommunityProjectsId: Get<PalletId>;

		/// The maximum length of data stored in for post codes.
		#[pallet::constant]
		type PostcodeLimit: Get<u32>;

		/// The maximum amount of token of a nft.
		#[pallet::constant]
		type MaxPaymentOptions: Get<u32>;

		/// A deposit for listing a property.
		type ListingDeposit: Get<Balance>;
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

	/// Mapping of a collection id to the region.
	#[pallet::storage]
	pub type RegionCollections<T: Config> =
		StorageMap<_, Blake2_128Concat, RegionId, <T as pallet::Config>::NftCollectionId, OptionQuery>;

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
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// This index is not taken.
		InvalidIndex,
		/// The buyer doesn't have enough funds.
		NotEnoughFunds,
		NotEnoughFunds1,
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
		ArithmeticUnderflow1,
		ArithmeticUnderflow2,
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
		AssetNotSupported1,
		ExceedsMaxEntries,
		InitializationFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
 		/// Creates a new region for the marketplace.
		/// This function calls the nfts-pallet to create a new collection.
		///
		/// The origin must be the LocationOrigin.
		///
		/// Emits `RegionCreated` event when succesfful.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_new_region())]
		pub fn create_new_region(origin: OriginFor<T>) -> DispatchResult {
			T::LocationOrigin::ensure_origin(origin)?;
			//let collection_id: CollectionId<T> = collection_id.into();
			let pallet_id: AccountIdOf<T> =
				Self::account_id();
			let collection_id = <T as pallet::Config>::Nfts::create_collection(
				&pallet_id, 
				&pallet_id, 
				&Self::default_collection_config(),
			)?;
			let mut region_id = NextRegionId::<T>::get();
			RegionCollections::<T>::insert(region_id, collection_id);
			region_id = region_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
			NextRegionId::<T>::put(region_id);
			Self::deposit_event(Event::<T>::RegionCreated { region_id, collection_id });
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
			T::LocationOrigin::ensure_origin(origin)?;
			ensure!(RegionCollections::<T>::get(region).is_some(), Error::<T>::RegionUnknown);
			ensure!(
				!LocationRegistration::<T>::get(region, location.clone()),
				Error::<T>::LocationRegistered
			);
			LocationRegistration::<T>::insert(region, location.clone(), true);
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
			ensure!(token_amount <= T::MaxNftToken::get(), Error::<T>::TooManyToken);
			let collection_id = RegionCollections::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			ensure!(
				LocationRegistration::<T>::get(region, location.clone()),
				Error::<T>::LocationUnknown
			);
			let item_id = NextNftId::<T>::get(collection_id);
			let mut asset_number: u32 = NextAssetId::<T>::get();
			let mut asset_id: LocalAssetIdOf<T> = asset_number.into();
			while !T::LocalCurrency::total_issuance(asset_id)
				.is_zero()
			{
				asset_number = asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
				asset_id = asset_number.into();
			}
			let asset_id: FractionalizedAssetId<T> = asset_number.into();
			let mut listing_id = NextListingId::<T>::get();
			let mut initial_funds = BoundedBTreeMap::default();
			initial_funds.try_insert(PaymentAssets::USDC, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?;
			initial_funds.try_insert(PaymentAssets::USDT, Default::default()).map_err(|_| Error::<T>::ExceedsMaxEntries)?; 
			let nft = NftListingDetails {
				real_estate_developer: signer.clone(),
				token_price,
				collected_funds: initial_funds.clone(),
				collected_tax: initial_funds.clone(),
				collected_fees: initial_funds,
				asset_id: asset_number,
				item_id,
				collection_id,
				token_amount,
			};
			let property_account = Self::property_account_id(asset_number);
			T::NativeCurrency::transfer(
				&signer,
				&property_account,
				T::ListingDeposit::get(),
				Preservation::Expendable
			)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;
			let pallet_account = Self::account_id();
			<T as pallet::Config>::Nfts::mint_into(
				&collection_id,
				&item_id,
				&property_account.clone(),
				&Self::default_item_config(),
				true
			)?;
			<T as pallet::Config>::Nfts::set_item_metadata(
				Some(&pallet_account),
				&collection_id,
				&item_id,
				&data,
			)?;
			let registered_nft_details = NftDetails {
				spv_created: false,
				asset_id: asset_number,
				region,
				location: location.clone(),
			};
			RegisteredNftDetails::<T>::insert(collection_id, item_id, registered_nft_details);
			OngoingObjectListing::<T>::insert(listing_id, nft);
			ListedToken::<T>::insert(listing_id, token_amount);

			let property_origin: OriginFor<T> = RawOrigin::Signed(property_account.clone()).into();
			let user_lookup = <T::Lookup as StaticLookup>::unlookup(property_account.clone());
			let nft_balance: FrationalizedNftBalanceOf<T> = token_amount.into();
			let fractionalize_collection_id = FractionalizeCollectionId::<T>::from(collection_id);
			let fractionalize_item_id = FractionalizeItemId::<T>::from(item_id);
  			pallet_nft_fractionalization::Pallet::<T>::fractionalize(
				property_origin.clone(),
				fractionalize_collection_id.into(),
				fractionalize_item_id.into(),
				asset_id.into(),
				user_lookup,
				nft_balance,
			)?;  
			let property_price = token_price
				.checked_mul(token_amount as u128)
				.ok_or(Error::<T>::MultiplyError)?;
			let asset_details =
				AssetDetails { collection_id, item_id, region, location, price: property_price, token_amount };
			AssetIdDetails::<T>::insert(asset_number, asset_details);
			let next_item_id = item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
			asset_number = asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
			NextNftId::<T>::insert(collection_id, next_item_id);
			NextAssetId::<T>::put(asset_number);
			listing_id = Self::next_listing_id(listing_id)?;
			NextListingId::<T>::put(listing_id);

			Self::deposit_event(Event::<T>::ObjectListed {
				collection_index: collection_id,
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

			ListedToken::<T>::try_mutate_exists(listing_id, |maybe_listed_token| {
				let listed_token = maybe_listed_token.as_mut().ok_or(Error::<T>::TokenNotForSale)?;
				ensure!(*listed_token >= amount, Error::<T>::NotEnoughTokenAvailable);
				let mut nft_details =
					OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
				ensure!(
					!RegisteredNftDetails::<T>::get(nft_details.collection_id, nft_details.item_id)
						.ok_or(Error::<T>::InvalidIndex)?
						.spv_created,
					Error::<T>::SpvAlreadyCreated
				);

				let transfer_price = nft_details
					.token_price
					.checked_mul(amount as u128)
					.ok_or(Error::<T>::MultiplyError)?;

				let fee = transfer_price
 					.checked_mul(1)
					.ok_or(Error::<T>::MultiplyError)?
					.checked_div(100) 
					.ok_or(Error::<T>::DivisionError)?;
				
				let tax = transfer_price
 					.checked_mul(3)
					.ok_or(Error::<T>::MultiplyError)?
					.checked_div(100) 
					.ok_or(Error::<T>::DivisionError)?;
				
				let total_transfer_price = transfer_price
					.checked_add(fee)
					.ok_or(Error::<T>::ArithmeticOverflow)?
					.checked_add(tax)
					.ok_or(Error::<T>::ArithmeticOverflow)?;

				//Self::transfer_funds(signer.clone(), Self::property_account_id(nft_details.asset_id), total_transfer_price, payment_asset.id())?;
				let frozen_balance = T::AssetsFreezer::balance_frozen(payment_asset.id(), &TestId::Marketplace, &signer);
				let new_frozen_balance = frozen_balance.checked_add(total_transfer_price).ok_or(Error::<T>::ArithmeticOverflow)?;
				T::AssetsFreezer::set_freeze(payment_asset.id(), &TestId::Marketplace, &signer, new_frozen_balance)?;
				*listed_token =
					listed_token.checked_sub(amount).ok_or(Error::<T>::ArithmeticUnderflow)?;
				if !TokenBuyer::<T>::get(listing_id).contains(&signer) {
					TokenBuyer::<T>::try_mutate(listing_id, |keys| {
						keys.try_push(signer.clone()).map_err(|_| Error::<T>::TooManyTokenBuyer)?;
						Ok::<(), DispatchError>(())
					})?;
				}
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
					if let Some(balance) = token_owner_details.paid_funds.get_mut(&payment_asset) {
						*balance = balance.checked_add(transfer_price).ok_or(Error::<T>::ArithmeticOverflow)?;
					} else {
						token_owner_details
							.paid_funds
							.try_insert(payment_asset.clone(), transfer_price)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					}
					if let Some(balance) = token_owner_details.paid_tax.get_mut(&payment_asset) {
						*balance = balance.checked_add(tax).ok_or(Error::<T>::ArithmeticOverflow)?;
					} else {
						token_owner_details
							.paid_tax
							.try_insert(payment_asset.clone(), tax)
							.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
					}
					Ok::<(), DispatchError>(())
				})?;
				if let Some(balance) = nft_details.collected_funds.get_mut(&payment_asset) {
					*balance = balance.checked_add(transfer_price).ok_or(Error::<T>::ArithmeticOverflow)?;
				} else {
					nft_details
						.collected_funds
						.try_insert(payment_asset.clone(), transfer_price)
						.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
				}
				if let Some(balance) = nft_details.collected_tax.get_mut(&payment_asset) {
					*balance = balance.checked_add(tax).ok_or(Error::<T>::ArithmeticOverflow)?;
				} else {
					nft_details
						.collected_tax
						.try_insert(payment_asset.clone(), tax)
						.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
				}					
				if let Some(balance) = nft_details.collected_fees.get_mut(&payment_asset) {
					*balance = balance.checked_add(fee).ok_or(Error::<T>::ArithmeticOverflow)?;
				} else {
					nft_details
						.collected_fees
						.try_insert(payment_asset.clone(), fee)
						.map_err(|_| Error::<T>::ExceedsMaxEntries)?;
				}		
				let asset_id = nft_details.asset_id;
				OngoingObjectListing::<T>::insert(listing_id, nft_details);
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
			let collection_id = RegionCollections::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;

			let nft_details = RegisteredNftDetails::<T>::get(collection_id, item_id)
				.ok_or(Error::<T>::NftNotFound)?;
			ensure!(
				LocationRegistration::<T>::get(region, nft_details.location),
				Error::<T>::LocationUnknown
			);
			let token_amount: Balance = amount.try_into().map_err(|_| Error::<T>::ConversionError)?;
			let mut listing_id = NextListingId::<T>::get();
			T::LocalCurrency::transfer(
				nft_details.asset_id,
				&signer,
				&Self::account_id(),
				token_amount,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;
			let token_listing = TokenListingDetails {
				seller: signer.clone(),
				token_price,
				asset_id: nft_details.asset_id,
				item_id,
				collection_id,
				amount,
			};
			TokenListings::<T>::insert(listing_id, token_listing);
			listing_id = Self::next_listing_id(listing_id)?;
			NextListingId::<T>::put(listing_id);

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
			let origin = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::Pallet::<T>::whitelisted_accounts(origin.clone()),
				Error::<T>::UserNotWhitelisted
			);
			let listing_details =
				TokenListings::<T>::take(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
			ensure!(listing_details.amount >= amount, Error::<T>::NotEnoughTokenAvailable);
			let price = listing_details
				.token_price
				.checked_mul(amount as u128)
				.ok_or(Error::<T>::MultiplyError)?;
			Self::buying_token_process(
				listing_id,
				origin.clone(),
				origin,
				listing_details,
				price,
				amount,
				payment_asset,
			)?;
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
		///
		/// Emits `OfferCreated` event when succesfful.
		#[pallet::call_index(6)]
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
			let price = offer_price
				.checked_mul(amount as u128)
				.ok_or(Error::<T>::MultiplyError)?;
			Self::transfer_funds(signer.clone(), Self::property_account_id(listing_details.asset_id), price, payment_asset.id())?;
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
		#[pallet::call_index(7)]
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
			let pallet_account = Self::property_account_id(listing_details.asset_id);
			match offer {
				Offer::Accept => {
					Self::buying_token_process(
						listing_id,
						pallet_account,
						offer_details.buyer,
						listing_details,
						price,
						offer_details.amount,
						offer_details.payment_assets,
					)?;
				}
				Offer::Reject => {
					Self::transfer_funds(pallet_account, offer_details.buyer, price, offer_details.payment_assets.id())?;
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
		#[pallet::call_index(8)]
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
			let listing_details = TokenListings::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
			Self::transfer_funds(Self::property_account_id(listing_details.asset_id), offer_details.buyer, price, offer_details.payment_assets.id())?;
			Self::deposit_event(Event::<T>::OfferCancelled { listing_id, account_id: signer.clone() });
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
		#[pallet::call_index(9)]
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
			let _ = TokenListings::<T>::try_mutate(listing_id, |maybe_listing_details| {
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
		#[pallet::call_index(10)]
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
			let _ = OngoingObjectListing::<T>::try_mutate(listing_id, |maybe_nft_details| {
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
		#[pallet::call_index(11)]
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
			T::LocalCurrency::transfer(
				listing_details.asset_id,
				&Self::account_id(),
				&signer.clone(),
				token_amount,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;
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
		#[pallet::call_index(12)]
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
		#[pallet::call_index(13)]
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
		#[pallet::call_index(14)]
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
		#[pallet::call_index(15)]
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
						PaymentAssets::USDT,
					)?; 
 					Self::execute_deal(
						listing_id, 
						property_lawyer_details,
						PaymentAssets::USDC,
					)?; 
					Self::distribute_property_token(listing_id)?;
					OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
				}
				(DocumentStatus::Rejected, DocumentStatus::Rejected) => {
					Self::burn_tokens_and_nfts(listing_id)?;
					Self::refund_investors(listing_id, property_lawyer_details.clone())?;
					OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
				}
				(DocumentStatus::Approved, DocumentStatus::Rejected) => {
					if !property_lawyer_details.second_attempt {
						property_lawyer_details.spv_status = DocumentStatus::Pending;
						property_lawyer_details.real_estate_developer_status = DocumentStatus::Pending;
						property_lawyer_details.second_attempt = true;
						PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
					} else {
						Self::burn_tokens_and_nfts(listing_id)?;
						Self::refund_investors(listing_id, property_lawyer_details)?;
						OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
					}
				}
				(DocumentStatus::Rejected, DocumentStatus::Approved) => {
					if !property_lawyer_details.second_attempt {
						property_lawyer_details.spv_status = DocumentStatus::Pending;
						property_lawyer_details.real_estate_developer_status = DocumentStatus::Pending;
						property_lawyer_details.second_attempt = true;
						PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
					} else {
						Self::burn_tokens_and_nfts(listing_id)?;
						Self::refund_investors(listing_id, property_lawyer_details)?;
						OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
					}
				}
				_ => {
					PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
				}
			}
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

		/// Get the account id of the community pallet
		pub fn community_account_id() -> AccountIdOf<T> {
			T::CommunityProjectsId::get().into_account_truncating()
		}

		pub fn next_listing_id(listing_id: ListingId) -> Result<ListingId, Error<T>> {
			listing_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)
		}

		/// Sends the token to the new owners and the funds to the real estate developer once all 100 token
		/// of a collection are sold.
		fn execute_deal(listing_id: u32, property_lawyer_details: PropertyLawyerDetails<T>, payment_asset: PaymentAssets) -> DispatchResult {
			let list = <TokenBuyer<T>>::get(listing_id);
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let pallet_account = Self::property_account_id(nft_details.asset_id);
			let treasury_id = Self::treasury_account_id();
			let property_account = Self::property_account_id(nft_details.asset_id);

			// Get lawyer accounts
			let real_estate_developer_lawyer_id = property_lawyer_details
				.real_estate_developer_lawyer
				.ok_or(Error::<T>::LawyerNotFound)?;
			let spv_lawyer_id = property_lawyer_details
				.spv_lawyer
				.ok_or(Error::<T>::LawyerNotFound)?;

			// Get total collected amounts for proportional calculations
			let total_collected_funds = nft_details
				.collected_funds
				.get(&payment_asset)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let real_estate_developer_lawyer_costs = property_lawyer_details
				.real_estate_developer_lawyer_costs
				.get(&payment_asset)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let spv_lawyer_costs = property_lawyer_details
				.spv_lawyer_costs
				.get(&payment_asset)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let developer_amount = total_collected_funds
				.checked_mul(&99)
				.ok_or(Error::<T>::MultiplyError)?
				.checked_div(100)
				.ok_or(Error::<T>::DivisionError)?;
			let treasury_amount = total_collected_funds
				.checked_div(&100u128)
				.ok_or(Error::<T>::DivisionError)?
				.checked_add(*nft_details.collected_fees
					.get(&payment_asset)
					.ok_or(Error::<T>::AssetNotSupported)?)
				.ok_or(Error::<T>::ArithmeticOverflow)?
				.saturating_sub(*real_estate_developer_lawyer_costs)
				.saturating_sub(*spv_lawyer_costs);
			let tax = nft_details
				.collected_tax
				.get(&payment_asset)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let real_estate_developer_amount = tax
				.checked_add(&real_estate_developer_lawyer_costs)
				.ok_or(Error::<T>::ArithmeticOverflow)?;

			for owner in list {
				let token_details: TokenOwnerDetails<Balance, T> = TokenOwner::<T>::get(owner.clone(), listing_id);
				if let Some(paid_funds) = token_details.paid_funds.get(&payment_asset) {
					let paid_funds = token_details
						.paid_funds
						.get(&payment_asset)
						.ok_or(Error::<T>::AssetNotSupported1)?;
					let paid_tax = token_details
						.paid_tax
						.get(&payment_asset)
						.ok_or(Error::<T>::AssetNotSupported)?;
					if !paid_funds.is_zero() {
						// Calculate investor's fee (1% of paid_funds)
						let investor_fee = paid_funds
							.checked_mul(&1)
							.ok_or(Error::<T>::MultiplyError)?
							.checked_div(100)
							.ok_or(Error::<T>::DivisionError)?;

						// Total amount to unfreeze (paid_funds + fee + tax)
						let total_investor_amount = paid_funds
							.checked_add(&investor_fee)
							.ok_or(Error::<T>::ArithmeticOverflow)?
							.checked_add(*paid_tax)
							.ok_or(Error::<T>::ArithmeticOverflow)?;
						
						let frozen_balance = T::AssetsFreezer::balance_frozen(payment_asset.id(), &TestId::Marketplace, &owner);
						let new_frozen_balance = frozen_balance.checked_sub(total_investor_amount).ok_or(Error::<T>::ArithmeticOverflow)?;
						T::AssetsFreezer::set_freeze(payment_asset.id(), &TestId::Marketplace, &owner, new_frozen_balance)?;
						
						Self::transfer_funds(
							owner.clone(),
							property_account.clone(),
							total_investor_amount,
							payment_asset.id(),
						)?;
					}
				}
			}
			Self::transfer_funds(
				property_account.clone(),
				nft_details.real_estate_developer.clone(),
				developer_amount,
				payment_asset.id(),
			)?;
			Self::transfer_funds(
				property_account.clone(),
				real_estate_developer_lawyer_id.clone(),
				real_estate_developer_amount,
				payment_asset.id(),
			)?;
			Self::transfer_funds(
				property_account.clone(),
				spv_lawyer_id.clone(),
				*spv_lawyer_costs,
				payment_asset.id(),
			)?;
			Self::transfer_funds(
				property_account,
				treasury_id,
				treasury_amount,
				payment_asset.id(),
			)?;

			let mut registered_nft_details =
				RegisteredNftDetails::<T>::get(nft_details.collection_id, nft_details.item_id)
					.ok_or(Error::<T>::InvalidIndex)?;
			registered_nft_details.spv_created = true;
			RegisteredNftDetails::<T>::insert(
				nft_details.collection_id,
				nft_details.item_id,
				registered_nft_details,
			);
			Ok(())
		}

		fn distribute_property_token(listing_id: u32) -> DispatchResult {
			let list = <TokenBuyer<T>>::take(listing_id);
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let pallet_account = Self::property_account_id(nft_details.asset_id);
			for owner in list {
				let token_details: TokenOwnerDetails<Balance, T> = TokenOwner::<T>::take(owner.clone(), listing_id);
				let token_amount = token_details.token_amount.try_into().map_err(|_| Error::<T>::ConversionError)?;
				T::LocalCurrency::transfer(
					nft_details.asset_id,
					&pallet_account.clone(),
					&owner.clone(),
					token_amount,
					Preservation::Expendable,
				)?;
				
				PropertyOwner::<T>::try_mutate(nft_details.asset_id, |keys| {
					keys.try_push(owner.clone()).map_err(|_| Error::<T>::TooManyTokenBuyer)?;
					Ok::<(), DispatchError>(())
				})?;
				PropertyOwnerToken::<T>::insert(nft_details.asset_id, owner, token_details.token_amount as u32)
			}
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

		fn refund_investors(listing_id: ListingId, property_lawyer_details: PropertyLawyerDetails<T>) -> DispatchResult {
			let list = <TokenBuyer<T>>::take(listing_id);
			let nft_details =
				OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
			let property_account = Self::property_account_id(nft_details.asset_id);
			let payment_asset_usdt = PaymentAssets::USDT;
			let payment_asset_usdc = PaymentAssets::USDC;

			let treasury_id = Self::treasury_account_id();
			let spv_lawyer_id = property_lawyer_details.spv_lawyer
				.ok_or(Error::<T>::LawyerNotFound)?;

			let fees_usdt = nft_details
				.collected_fees
				.get(&payment_asset_usdt)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let fees_usdc = nft_details
				.collected_fees
				.get(&payment_asset_usdc)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let spv_lawyer_costs_usdt = property_lawyer_details
				.spv_lawyer_costs
				.get(&payment_asset_usdt)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let spv_lawyer_costs_usdc = property_lawyer_details
				.spv_lawyer_costs
				.get(&payment_asset_usdc)
				.ok_or(Error::<T>::AssetNotSupported)?;
			let treasury_amount_usdt = fees_usdt
				.checked_sub(spv_lawyer_costs_usdt)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			let treasury_amount_usdc = fees_usdc
				.checked_sub(spv_lawyer_costs_usdc)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			for owner in list {
				let token_details: TokenOwnerDetails<Balance, T> = TokenOwner::<T>::take(owner.clone(), listing_id);
				
				// Process USDT payments if the owner has paid in USDT
				if let (Some(paid_funds_usdt), Some(paid_tax_usdt)) = (
					token_details.paid_funds.get(&payment_asset_usdt),
					token_details.paid_tax.get(&payment_asset_usdt),
				) {
					if !paid_funds_usdt.is_zero() && !paid_tax_usdt.is_zero() {
						// Calculate total refund amount (paid funds + tax)
						let refund_amount_usdt = paid_funds_usdt
							.checked_add(&paid_tax_usdt)
							.ok_or(Error::<T>::ArithmeticOverflow)?;

						// Calculate investor fee (1% of paid funds)
						let investor_fee_usdt = paid_funds_usdt
							.checked_mul(&1)
							.ok_or(Error::<T>::MultiplyError)?
							.checked_div(100)
							.ok_or(Error::<T>::DivisionError)?;

						// Total amount to unfreeze (refund + fee)
						let total_investor_amount_usdt = refund_amount_usdt
							.checked_add(investor_fee_usdt)
							.ok_or(Error::<T>::ArithmeticOverflow)?;

						// Unfreeze the investor's USDT funds
						let frozen_balance_usdt = T::AssetsFreezer::balance_frozen(
							payment_asset_usdt.id(),
							&TestId::Marketplace,
							&owner,
						);
						let new_frozen_balance_usdt = frozen_balance_usdt
							.checked_sub(total_investor_amount_usdt)
							.ok_or(Error::<T>::ArithmeticOverflow)?;
						T::AssetsFreezer::set_freeze(
							payment_asset_usdt.id(),
							&TestId::Marketplace,
							&owner,
							new_frozen_balance_usdt,
						)?;

						// Transfer USDT funds to property account
						Self::transfer_funds(
							owner.clone(),
							property_account.clone(),
							investor_fee_usdt,
							payment_asset_usdt.id(),
						)?;
					}
				}
				// Process USDC payments if the owner has paid in USDC
				if let (Some(paid_funds_usdc), Some(paid_tax_usdc)) = (
					token_details.paid_funds.get(&payment_asset_usdc),
					token_details.paid_tax.get(&payment_asset_usdc),
				) {
					if !paid_funds_usdc.is_zero() && !paid_tax_usdc.is_zero() {
						// Calculate total refund amount (paid funds + tax)
						let refund_amount_usdc = paid_funds_usdc
							.checked_add(&paid_tax_usdc)
							.ok_or(Error::<T>::ArithmeticOverflow)?;

						// Calculate investor fee (1% of paid funds)
						let investor_fee_usdc = paid_funds_usdc
							.checked_mul(&1)
							.ok_or(Error::<T>::MultiplyError)?
							.checked_div(100)
							.ok_or(Error::<T>::DivisionError)?;

						// Total amount to unfreeze (refund + fee)
						let total_investor_amount_usdc = refund_amount_usdc
							.checked_add(investor_fee_usdc)
							.ok_or(Error::<T>::ArithmeticOverflow)?;

						// Unfreeze the investor's USDC funds
						let frozen_balance_usdc = T::AssetsFreezer::balance_frozen(
							payment_asset_usdc.id(),
							&TestId::Marketplace,
							&owner,
						);
						let new_frozen_balance_usdc = frozen_balance_usdc
							.checked_sub(total_investor_amount_usdc)
							.ok_or(Error::<T>::ArithmeticOverflow)?;
						T::AssetsFreezer::set_freeze(
							payment_asset_usdc.id(),
							&TestId::Marketplace,
							&owner,
							new_frozen_balance_usdc,
						)?;

						// Transfer USDC funds to treasury and SPV lawyer
						Self::transfer_funds(
							owner.clone(),
							property_account.clone(),
							investor_fee_usdc,
							payment_asset_usdc.id(),
						)?;
					}
				}				
				PropertyOwner::<T>::take(nft_details.asset_id);
				PropertyOwnerToken::<T>::take(nft_details.asset_id, owner);
			}
			Self::transfer_funds(property_account.clone(), treasury_id.clone(), treasury_amount_usdt, payment_asset_usdt.id())?;
			Self::transfer_funds(property_account.clone(), treasury_id, treasury_amount_usdc, payment_asset_usdc.id())?;
			Self::transfer_funds(property_account.clone(), spv_lawyer_id.clone(), *spv_lawyer_costs_usdt, payment_asset_usdt.id())?;
			Self::transfer_funds(property_account.clone(), spv_lawyer_id, *spv_lawyer_costs_usdc, payment_asset_usdc.id())?;
			Ok(())
		}

		fn buying_token_process(
			listing_id: u32,
			transfer_from: AccountIdOf<T>,
			account: AccountIdOf<T>,
			mut listing_details: ListingDetailsType<T>,
			price: Balance,
			amount: u32,
			payment_asset: PaymentAssets,
		) -> DispatchResult {
			Self::calculate_fees(price, transfer_from.clone(), listing_details.seller.clone(), payment_asset.id())?;
			let token_amount = amount.into();
			T::LocalCurrency::transfer(
				listing_details.asset_id,
				&Self::account_id(),
				&account.clone(),
				token_amount,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;
			let mut old_token_owner_amount = PropertyOwnerToken::<T>::take(
				listing_details.asset_id,
				listing_details.seller.clone(),
			);
			old_token_owner_amount = old_token_owner_amount
				.checked_sub(amount)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			if old_token_owner_amount == 0 {
				let mut owner_list = PropertyOwner::<T>::take(listing_details.asset_id);
				let index = owner_list
					.iter()
					.position(|x| *x == listing_details.seller.clone())
					.ok_or(Error::<T>::InvalidIndex)?;
				owner_list.remove(index);
				PropertyOwner::<T>::insert(listing_details.asset_id, owner_list);
			} else {
				PropertyOwnerToken::<T>::insert(
					listing_details.asset_id,
					listing_details.seller.clone(),
					old_token_owner_amount,
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

		fn calculate_fees(
			price: Balance,
			sender: AccountIdOf<T>,
			receiver: AccountIdOf<T>,
			asset: u32,
		) -> DispatchResult {
			let fees = price
				.checked_div(100u128)
				.ok_or(Error::<T>::DivisionError)?;
			let treasury_id = Self::treasury_account_id();
			let seller_part = price
				.checked_mul(99u128)
				.ok_or(Error::<T>::MultiplyError)?
				.checked_div(100)
				.ok_or(Error::<T>::DivisionError)?;
			Self::transfer_funds(sender.clone(), treasury_id, fees, asset)?;
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
			from: AccountIdOf<T>,
			to: AccountIdOf<T>,
			amount: Balance,
			asset: u32,
		) -> DispatchResult {
			if !amount.is_zero() {
				T::ForeignCurrency::transfer(asset, &from, &to, amount, Preservation::Expendable)
					.map_err(|_| Error::<T>::NotEnoughFunds)?;
			}
			Ok(())
		}

		fn transfer_funds1(
			from: AccountIdOf<T>,
			to: AccountIdOf<T>,
			amount: Balance,
			asset: u32,
		) -> DispatchResult {
			if !amount.is_zero() {
				T::ForeignCurrency::transfer(asset, &from, &to, amount, Preservation::Expendable)
					.map_err(|_| Error::<T>::NotEnoughFunds1)?;
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