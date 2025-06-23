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
	sp_runtime::{Saturating, traits::Zero, Percent},
    pallet_prelude::*, PalletId,
    traits::{
        tokens::{fungible, nonfungibles_v2, Balance, Precision},
		nonfungibles_v2::{Create, Transfer},
        fungible::{MutateHold, Inspect},
    }
};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::{CheckedAdd, One, AccountIdConversion}, Permill};

    /// Infos regarding regions.
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionInfo<T: Config> {
        pub collection_id: <T as pallet::Config>::NftCollectionId,
        pub listing_duration: BlockNumberFor<T>,
        pub owner: T::AccountId,
        pub tax: Permill,
    }

	/// Infos regarding the proposal of a region
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
	pub struct RegionProposal<T: Config> {
		pub proposer: T::AccountId,
		pub data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
		pub created_at: BlockNumberFor<T>,
		pub proposal_expiry: BlockNumberFor<T>,
	}

	/// Voting stats.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct VoteStats<T: Config> {
		pub yes_voting_power: T::Balance,
		pub no_voting_power: T::Balance,
	}

	/// Info for region auctions.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct RegionAuction<T: Config> {
		pub highest_bidder: Option<T::AccountId>,
		pub collateral: T::Balance,
		pub auction_expiry: BlockNumberFor<T>,
	}

	/// Takeover enum.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	pub enum TakeoverAction {
		Accept,
		Reject,
	}

	/// Vote enum.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	pub enum Vote {
		Yes,
		No,
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

		/// The amount of time give to vote for a new region.
		#[pallet::constant]
		type RegionVotingTime: Get<BlockNumberFor<Self>>;

		/// The amount of time give to vote for a new region.
		#[pallet::constant]
		type RegionAuctionTime: Get<BlockNumberFor<Self>>;

		/// Threshold that needs to be reached to let a region get created.
		#[pallet::constant]
		type RegionThreshold: Get<Percent>;

		/// Minimum number of blocks between two proposals.
		#[pallet::constant]
		type RegionProposalCooldown: Get<BlockNumberFor<Self>>;

		/// Origin who can add and remove users to the region operators.
		type RegionOperatorOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}
	pub type LocationId<T> = BoundedVec<u8, <T as Config>::PostcodeLimit>;

    pub type RegionId = u32;
	pub type ProposalIndex = u32;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type RegionOperatorAccounts<T: Config> = 
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, OptionQuery>;

	#[pallet::storage]
	pub type LastRegionProposalBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	#[pallet::storage]
	pub type RegionProposalCount<T> = StorageValue<_, ProposalIndex, ValueQuery>;

	#[pallet::storage]
	pub type RegionProposals<T: Config> =
		StorageMap<_, Blake2_128Concat, ProposalIndex, RegionProposal<T>, OptionQuery>;

	#[pallet::storage]
	pub type OngoingRegionProposalVotes<T: Config> =
		StorageMap<_, Blake2_128Concat, ProposalIndex, VoteStats<T>, OptionQuery>;

	#[pallet::storage]
	pub(super) type UserRegionVote<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProposalIndex,
		Blake2_128Concat,
		T::AccountId,
		Vote,
		OptionQuery,	
	>;

	#[pallet::storage]
	pub(super) type RegionAuctions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ProposalIndex,
		RegionAuction<T>,
		OptionQuery,
	>;

    /// Mapping of region to the region information.
	#[pallet::storage]
	pub type RegionDetails<T: Config> = 
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
		/// A new region has been proposed.
		RegionProposed { region_proposal_id: ProposalIndex, proposer: T::AccountId },
		/// Voted on region proposal.
		VotedOnRegionProposal { region_proposal_id: ProposalIndex, voter: T::AccountId, vote: Vote },
        /// New region has been created.
		RegionCreated { region_id: u32, collection_id: <T as pallet::Config>::NftCollectionId, owner: T::AccountId, listing_duration: BlockNumberFor<T>, tax: Permill },
		/// No region has been created.
		NoRegionCreated { region_proposal_id: ProposalIndex },
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
		/// An auction for a region has started.
		RegionAuctionStarted { proposal_id: ProposalIndex },
		/// A region got rejected.
		RegionRejected { proposal_id: ProposalIndex },
		/// A bid for a region got placed.
		BidSuccessfullyPlaced { proposal_id: ProposalIndex, bidder: T::AccountId, new_leading_bid: T::Balance },
		/// A new regional operator has been added.
		NewRegionOperatorAdded { new_operator: T::AccountId },
		/// A regional operator has been removed.
		RegionOperatorRemoved { regional_operator: T::AccountId },
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
		/// The proposal is not ongoing.
		NotOngoing,
		/// There is no auction to bid on.
		NoOngoingAuction,
		/// The bid is lower than the current highest bid.
		BidTooLow,
		/// The voting has not ended yet.
		VotingStillOngoing,
		/// No Auction found.
		NoAuction,
		/// Auction is still ongoing.
		AuctionNotFinished,
		/// Noone bid on the region so there is no region owner.
		NoNewRegionOwner,
		/// Cant propose a new regions since the cooldown is still active.
		RegionProposalCooldownActive,
		/// The proposa has already expired.
		ProposalExpired,
		/// Bid amount can not be zero.
		BidCannotBeZero,
		/// This account is already registers as a region operator.
		AlreadyRegionOperator,
		/// This account is not a regional operator.
		NoRegionalOperator,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(10)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn propose_new_region(
			origin: OriginFor<T>, 
			data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			if let Some(last_proposal_block) = LastRegionProposalBlock::<T>::get() {
				let cooldown = T::RegionProposalCooldown::get();
				ensure!(
					current_block_number.saturating_sub(last_proposal_block) >= cooldown,
					Error::<T>::RegionProposalCooldownActive
				);
			}
			let region_proposal_id = RegionProposalCount::<T>::get();
			let expiry_block = current_block_number
				.saturating_add(<T as Config>::RegionVotingTime::get());
			let sale_proposal = RegionProposal {
				proposer: signer.clone(),
				created_at: current_block_number,
				proposal_expiry: expiry_block,
				data,
			};
			let vote_stats = VoteStats { yes_voting_power: Zero::zero(), no_voting_power: Zero::zero() };
			let next_region_proposal_id = region_proposal_id.saturating_add(1);
			RegionProposals::<T>::insert(region_proposal_id, sale_proposal);
			OngoingRegionProposalVotes::<T>::insert(region_proposal_id, vote_stats);
			RegionProposalCount::<T>::put(next_region_proposal_id);
			LastRegionProposalBlock::<T>::put(current_block_number);
			Self::deposit_event(Event::RegionProposed { region_proposal_id, proposer: signer });
			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn vote_on_region_proposal(
			origin: OriginFor<T>,
			proposal_id: ProposalIndex,
			vote: Vote,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			let region_proposal = RegionProposals::<T>::get(proposal_id).ok_or(Error::<T>::NotOngoing)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(region_proposal.proposal_expiry > current_block_number, Error::<T>::ProposalExpired);
			let voting_power = T::NativeCurrency::total_balance(&signer);
 			OngoingRegionProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote|{
				let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
				let previous_vote_opt = UserRegionVote::<T>::get(proposal_id, &signer);
				if let Some(previous_vote) = previous_vote_opt {
					match previous_vote {
						Vote::Yes => current_vote.yes_voting_power = current_vote.yes_voting_power.saturating_sub(voting_power),
						Vote::No => current_vote.no_voting_power = current_vote.no_voting_power.saturating_sub(voting_power),
					}
				}
				
				match vote {
					Vote::Yes => current_vote.yes_voting_power = current_vote.yes_voting_power.saturating_add(voting_power),
					Vote::No => current_vote.no_voting_power = current_vote.no_voting_power.saturating_add(voting_power),
				}
				Ok::<(), DispatchError>(())
			})?;
			UserRegionVote::<T>::insert(proposal_id, &signer, vote.clone());
			Self::deposit_event(Event::VotedOnRegionProposal { region_proposal_id: proposal_id, voter: signer, vote });
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn process_region_voting(
			origin: OriginFor<T>,
			proposal_id: ProposalIndex,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			let region_proposal = RegionProposals::<T>::get(proposal_id).ok_or(Error::<T>::NotOngoing)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(region_proposal.proposal_expiry <= current_block_number, Error::<T>::VotingStillOngoing);
			Self::finalize_region_proposal(proposal_id, current_block_number)?;
			Ok(())
		}

		#[pallet::call_index(13)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn bid_on_region(
			origin: OriginFor<T>,
			proposal_id: ProposalIndex,
			amount: T::Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(!amount.is_zero(), Error::<T>::BidCannotBeZero);
			RegionAuctions::<T>::try_mutate(proposal_id, |maybe_auction| -> DispatchResult {
				let auction = maybe_auction.as_mut().ok_or(Error::<T>::NoOngoingAuction)?;
				let current_block_number = <frame_system::Pallet<T>>::block_number();
				ensure!(auction.auction_expiry > current_block_number, Error::<T>::NoOngoingAuction);
				ensure!(amount > auction.collateral, Error::<T>::BidTooLow);
				match &auction.highest_bidder {
					Some(old_bidder) if old_bidder == &signer => {
						let additional = amount.saturating_sub(auction.collateral);
						if additional > Zero::zero() {
							T::NativeCurrency::hold(&HoldReason::RegionDepositReserve.into(), &signer, additional)?;
						}
					},
					Some(old_bidder) => {
						T::NativeCurrency::hold(&HoldReason::RegionDepositReserve.into(), &signer, amount)?;
						T::NativeCurrency::release(
							&HoldReason::RegionDepositReserve.into(),
							old_bidder,
							auction.collateral,
							Precision::Exact,
						)?;
					},
					None => {
						T::NativeCurrency::hold(&HoldReason::RegionDepositReserve.into(), &signer, amount)?;
					}
				}
				auction.highest_bidder = Some(signer.clone());
				auction.collateral = amount;
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::BidSuccessfullyPlaced { proposal_id, bidder: signer, new_leading_bid: amount });
			Ok(())
		}

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
		pub fn create_new_region(origin: OriginFor<T>, proposal_id: ProposalIndex, listing_duration: BlockNumberFor<T>, tax: Permill) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			let auction = RegionAuctions::<T>::take(proposal_id).ok_or(Error::<T>::NoAuction)?;
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			ensure!(auction.auction_expiry <= current_block_number, Error::<T>::AuctionNotFinished);

			if let Some(region_owner) = auction.highest_bidder {
				if auction.collateral.is_zero() {
					Self::deposit_event(Event::<T>::NoRegionCreated { region_proposal_id: proposal_id });
					return Ok(());
				}
				ensure!(!listing_duration.is_zero(), Error::<T>::ListingDurationCantBeZero);
				ensure!(listing_duration <= T::MaxListingDuration::get(), Error::<T>::ListingDurationTooHigh);
				
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
					owner: region_owner.clone(),
					tax,
				};
				RegionDetails::<T>::insert(current_region_id, region_info);
				NextRegionId::<T>::put(next_region_id);
				
				Self::deposit_event(Event::<T>::RegionCreated { 
					region_id: current_region_id, 
					collection_id,
					owner: region_owner,
					listing_duration,
					tax, 
				});
			} else {
				Self::deposit_event(Event::<T>::NoRegionCreated { region_proposal_id: proposal_id });
				return Ok(());
			}
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
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);

			RegionDetails::<T>::try_mutate(region, |maybe_region| {
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
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);

			RegionDetails::<T>::try_mutate(region, |maybe_region| {
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
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			let region_info = RegionDetails::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
		
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
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			let mut region_info = RegionDetails::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
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
					RegionDetails::<T>::insert(region, region_info);
	
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
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
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
				pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
				Error::<T>::UserNotWhitelisted
			);
			ensure!(RegionDetails::<T>::contains_key(region), Error::<T>::RegionUnknown);
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

		#[pallet::call_index(7)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn add_regional_operator(
			origin: OriginFor<T>,
			new_operator: T::AccountId,
		) -> DispatchResult {
			T::RegionOperatorOrigin::ensure_origin(origin)?;

			ensure!(RegionOperatorAccounts::<T>::get(&new_operator).is_none(), Error::<T>::AlreadyRegionOperator);
			RegionOperatorAccounts::<T>::insert(&new_operator, true);
			Self::deposit_event(Event::<T>::NewRegionOperatorAdded { new_operator });
			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn remove_regional_operator(
			origin: OriginFor<T>,
			regional_operator: T::AccountId,
		) -> DispatchResult {
			T::RegionOperatorOrigin::ensure_origin(origin)?;

			RegionOperatorAccounts::<T>::take(&regional_operator).ok_or(Error::<T>::NoRegionalOperator)?;
			Self::deposit_event(Event::<T>::RegionOperatorRemoved { regional_operator });
			Ok(())
		}
	}

    impl<T: Config> Pallet<T> {
		/// Get the account id of the pallet
		pub fn account_id() -> T::AccountId {
			<T as pallet::Config>::PalletId::get().into_account_truncating()
		}

		fn finalize_region_proposal(proposal_id: ProposalIndex, current_block_number: BlockNumberFor<T>) -> DispatchResult {
			let voting_results = <OngoingRegionProposalVotes<T>>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
			let _ = <RegionProposals<T>>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
			let _ = UserRegionVote::<T>::clear_prefix(proposal_id, u32::MAX, None);	
			let required_threshold = T::RegionThreshold::get();
			let total_voting_amount = voting_results.yes_voting_power.checked_add(&voting_results.no_voting_power).ok_or(Error::<T>::ArithmeticOverflow)?;
			let yes_votes_percentage = Percent::from_rational(voting_results.yes_voting_power, total_voting_amount);
			let auction_expiry_block = current_block_number.saturating_add(T::RegionAuctionTime::get());
			if yes_votes_percentage > required_threshold {
				let auction = RegionAuction {
					highest_bidder: None,
					collateral: Zero::zero(),
					auction_expiry: auction_expiry_block,
				};
				RegionAuctions::<T>::insert(proposal_id, auction);
				Self::deposit_event(Event::RegionAuctionStarted { proposal_id });
			} else {
				Self::deposit_event(Event::RegionRejected { proposal_id });
			}						
			Ok(())
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

