#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::{
    pallet_prelude::*
};

use frame_support::{
	traits::{
		tokens::nonfungibles_v2,	
		nonfungibles_v2::Mutate as NonfungiblesMutate,
		nonfungibles_v2::Transfer,
	},
};

use pallet_nfts::{
	CollectionConfig, ItemConfig, ItemSettings,
};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{CheckedAdd, One};

	#[pallet::config]
	pub trait Config: frame_system::Config 
		+ pallet_nfts::Config 
		+ pallet_xcavate_whitelist::Config
		+ pallet_regions::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;	

		/// The type used to identify an NFT within a collection.
		type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

		type Nfts: nonfungibles_v2::Inspect<Self::AccountId, ItemId = <Self as pallet::Config>::NftId,
			CollectionId = <Self as pallet_regions::Config>::NftCollectionId>	
			+ Transfer<Self::AccountId>
			+ nonfungibles_v2::Mutate<Self::AccountId, ItemConfig>
			+ nonfungibles_v2::Create<Self::AccountId, CollectionConfig<<Self as pallet_regions::Config>::Balance, 
			BlockNumberFor<Self>, <Self as pallet_nfts::Config>::CollectionId>>;
	}

	pub type RegionId = u16;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Id for the next nft in a collection.
	#[pallet::storage]
	pub(super) type NextNftId<T: Config> =
		StorageMap<_, Blake2_128Concat, <T as pallet_regions::Config>::NftCollectionId, <T as pallet::Config>::NftId, ValueQuery>;
	
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Test
		TestCreated { test_id: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// This Region is not known.
		RegionUnknown,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(11)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn create_property_token(
			origin: OriginFor<T>,
			region: RegionId,
			data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let region_info = pallet_regions::RegionDetails::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			let item_id = NextNftId::<T>::get(region_info.collection_id);
			<T as pallet::Config>::Nfts::mint_into(
				&region_info.collection_id,
				&item_id,
				&signer.clone(),
				&Self::default_item_config(),
				true
			)?;
			<T as pallet::Config>::Nfts::set_item_metadata(
				Some(&signer),
				&region_info.collection_id,
				&item_id,
				&data,
			)?;
			Ok(())
		}
	}

    impl<T: Config> Pallet<T> {
		/// Set the default item configuration for minting a nft.
		fn default_item_config() -> ItemConfig {
			ItemConfig { settings: ItemSettings::all_enabled() }
		}
    }
}



