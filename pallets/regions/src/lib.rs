#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use pallet_nfts::{CollectionConfig, CollectionSettings, ItemConfig, MintSettings};

use frame_support::{
    pallet_prelude::*,
    sp_runtime::{traits::Zero, Percent, Saturating},
    traits::{
        fungible::{BalancedHold, Credit, Inspect, InspectHold, Mutate, MutateHold},
        nonfungibles_v2::{Create, Transfer},
        tokens::{
            fungible, imbalance::OnUnbalanced, nonfungibles_v2, Balance, Precision, Preservation,
        },
        EnsureOriginWithArg,
    },
    PalletId,
};

pub type NegativeImbalanceOf<T> =
    Credit<<T as frame_system::Config>::AccountId, <T as Config>::NativeCurrency>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, CheckedAdd, One},
        Permill,
    };

    /// Infos regarding regions.
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionInfo<T: Config> {
        pub collection_id: <T as pallet::Config>::NftCollectionId,
        pub listing_duration: BlockNumberFor<T>,
        pub owner: T::AccountId,
        pub collateral: T::Balance,
        pub active_strikes: u8,
        pub tax: Permill,
        pub next_owner_change: BlockNumberFor<T>,
        pub location_count: u32,
    }

    /// Infos regarding the proposal of a region
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionProposal<T: Config> {
        pub proposer: T::AccountId,
        pub created_at: BlockNumberFor<T>,
        pub proposal_expiry: BlockNumberFor<T>,
        pub deposit: T::Balance,
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

    /// Infos regarding the proposal of removing a region owner
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RemoveRegionOwnerProposal<T: Config> {
        pub proposer: T::AccountId,
        pub proposal_expiry: BlockNumberFor<T>,
        pub deposit: T::Balance,
    }

    /// Vote record of a user.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct VoteRecord<T: Config> {
        pub vote: Vote,
        pub region_id: RegionId,
        pub power: T::Balance,
    }

    /// Infos of a lawyer.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct LawyerInfo<T: Config> {
        pub region: RegionId,
        pub deposit: T::Balance,
        pub active_cases: u32,
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
    #[repr(u16)]
    pub enum RegionIdentifier {
        England = 1,
        France = 2,
        Japan = 3,
        India = 4,
    }

    impl RegionIdentifier {
        pub fn into_u16(self) -> u16 {
            self as u16
        }
    }

    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Funds are held for operating a region.
        #[codec(index = 0)]
        RegionDepositReserve,
        #[codec(index = 1)]
        RegionalOperatorRemovalReserve,
        #[codec(index = 2)]
        RegionProposalReserve,
        #[codec(index = 3)]
        LawyerDepositReserve,
        #[codec(index = 4)]
        RegionVotingReserve,
        #[codec(index = 5)]
        RegionOperatorRemovalVoting,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_nfts::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;

        type Balance: Balance + TypeInfo;

        type NativeCurrency: fungible::Inspect<Self::AccountId>
            + fungible::Mutate<Self::AccountId>
            + fungible::InspectHold<Self::AccountId, Balance = Self::Balance>
            + fungible::BalancedHold<Self::AccountId, Balance = Self::Balance>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<
                Self::AccountId,
                Reason = <Self as pallet::Config>::RuntimeHoldReason,
            >;

        /// The overarching hold reason.
        type RuntimeHoldReason: From<HoldReason>;

        type Nfts: nonfungibles_v2::Inspect<
                Self::AccountId,
                ItemId = <Self as pallet::Config>::NftId,
                CollectionId = <Self as pallet::Config>::NftCollectionId,
            > + Transfer<Self::AccountId>
            + nonfungibles_v2::Mutate<Self::AccountId, ItemConfig>
            + nonfungibles_v2::Create<
                Self::AccountId,
                CollectionConfig<
                    Self::Balance,
                    BlockNumberFor<Self>,
                    <Self as pallet_nfts::Config>::CollectionId,
                >,
            >;

        /// Identifier for the collection of NFT.
        type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy;

        /// The type used to identify an NFT within a collection.
        type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

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

        /// The amount of time give to vote against a region operator.
        #[pallet::constant]
        type RegionOperatorVotingTime: Get<BlockNumberFor<Self>>;

        /// The maximum amount of proposals per block.
        #[pallet::constant]
        type MaxProposalsForBlock: Get<u32>;

        /// The Trasury's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryId: Get<PalletId>;

        /// The minimum amount of a regional operator that will be slashed.
        #[pallet::constant]
        type RegionSlashingAmount: Get<Self::Balance>;

        /// The time period required between region owner change.
        #[pallet::constant]
        type RegionOwnerChangePeriod: Get<BlockNumberFor<Self>>;

        /// Handler for the unbalanced reduction when slashing a region owner.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Delay after a region owner resigns before a new auction can begin.
        #[pallet::constant]
        type RegionOwnerNoticePeriod: Get<BlockNumberFor<Self>>;

        /// Deposit amount for a remove regional operator proposal.
        #[pallet::constant]
        type RegionOwnerDisputeDeposit: Get<Self::Balance>;

        /// Minimum deposit for a location.
        #[pallet::constant]
        type MinimumRegionDeposit: Get<Self::Balance>;

        /// Deposit for a region proposal.
        #[pallet::constant]
        type RegionProposalDeposit: Get<Self::Balance>;

        /// Minimum voting amount.
        #[pallet::constant]
        type MinimumVotingAmount: Get<Self::Balance>;

        /// The maximum amount of voters for a region.
        #[pallet::constant]
        type MaxRegionVoters: Get<u32>;

        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            pallet_xcavate_whitelist::Role,
            Success = Self::AccountId,
        >;

        /// A deposit for being active as a lawyer.
        #[pallet::constant]
        type LawyerDeposit: Get<<Self as pallet::Config>::Balance>;
    }
    pub type LocationId<T> = BoundedVec<u8, <T as Config>::PostcodeLimit>;

    pub type RegionId = u16;
    pub type ProposalId = u64;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Block number of the last region proposal made.
    #[pallet::storage]
    pub(super) type LastRegionProposalBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, OptionQuery>;

    /// Currently proposed region IDs.
    #[pallet::storage]
    pub(super) type ProposedRegionIds<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, (), OptionQuery>;

    /// Active region proposals by region ID.
    #[pallet::storage]
    pub(super) type RegionProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, RegionProposal<T>, OptionQuery>;

    /// Voting statistics for ongoing proposals by region ID.
    #[pallet::storage]
    pub(super) type OngoingRegionProposalVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats<T>, OptionQuery>;

    /// User votes on region proposals.
    #[pallet::storage]
    pub(super) type UserRegionVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        T::AccountId,
        VoteRecord<T>,
        OptionQuery,
    >;

    /// Active region auctions.
    #[pallet::storage]
    pub(super) type RegionAuctions<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, RegionAuction<T>, OptionQuery>;

    /// Replacement auctions for regions.
    #[pallet::storage]
    pub(super) type RegionReplacementAuctions<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, RegionAuction<T>, OptionQuery>;

    /// Mapping of region to the region information.
    #[pallet::storage]
    pub type RegionDetails<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, RegionInfo<T>, OptionQuery>;

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

    /// Mapping from Region ID to a proposal to remove the region owner.
    #[pallet::storage]
    pub(super) type RegionOwnerProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, RemoveRegionOwnerProposal<T>, OptionQuery>;

    #[pallet::storage]
    pub(super) type OngoingRegionOwnerProposalVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats<T>, OptionQuery>;

    #[pallet::storage]
    pub(super) type UserRegionOwnerVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        T::AccountId,
        VoteRecord<T>,
        OptionQuery,
    >;

    /// Stores the project keys and round types ending on a given block for region owner removal votings.
    #[pallet::storage]
    pub(super) type RegionOwnerRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<RegionId, T::MaxProposalsForBlock>,
        ValueQuery,
    >;

    /// Stores the project keys and round types ending on a given block for region owner removal votings.
    #[pallet::storage]
    pub(super) type ReplacementAuctionExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<RegionId, T::MaxProposalsForBlock>,
        ValueQuery,
    >;

    /// Stores in which region a lawyer is active.
    #[pallet::storage]
    pub type RealEstateLawyer<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LawyerInfo<T>, OptionQuery>;

    /// Counter of proposal ids.
    #[pallet::storage]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    #[pallet::storage]
    pub type RegionProposalId<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, ProposalId, OptionQuery>;

    #[pallet::storage]
    pub type RegionOwnerProposalId<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, ProposalId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new region has been proposed.
        RegionProposed {
            region_id: RegionId,
            proposer: T::AccountId,
            proposal_id: ProposalId,
        },
        /// Voted on region proposal.
        VotedOnRegionProposal {
            region_id: RegionId,
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
            voting_power: T::Balance,
            new_yes_power: T::Balance,
            new_no_power: T::Balance,
        },
        /// New region has been created.
        RegionCreated {
            region_id: RegionId,
            collection_id: <T as pallet::Config>::NftCollectionId,
            owner: T::AccountId,
            listing_duration: BlockNumberFor<T>,
            tax: Permill,
        },
        /// No region has been created.
        NoRegionCreated { region_id: RegionId },
        /// Listing duration of a region changed.
        ListingDurationChanged {
            region_id: RegionId,
            listing_duration: BlockNumberFor<T>,
        },
        /// Tax of a region changed.
        RegionTaxChanged {
            region_id: RegionId,
            new_tax: Permill,
        },
        /// New location has been created.
        LocationCreated {
            region_id: RegionId,
            location_id: LocationId<T>,
            new_collateral_balance: T::Balance,
            new_location_count: u32,
        },
        /// An auction for a region has started.
        RegionAuctionStarted { region_id: RegionId },
        /// A region got rejected.
        RegionProposalRejected {
            region_id: RegionId,
            slashed_account: T::AccountId,
            amount: T::Balance,
        },
        /// A bid for a region got placed.
        BidSuccessfullyPlaced {
            region_id: RegionId,
            bidder: T::AccountId,
            new_leading_bid: T::Balance,
            previous_bidder: Option<T::AccountId>,
        },
        /// A new regional operator has been added.
        NewRegionOperatorAdded { new_operator: T::AccountId },
        /// A regional operator has been removed.
        RegionOperatorRemoved { regional_operator: T::AccountId },
        /// A proposal to remove the region owner has been proposed.
        RemoveRegionOwnerProposed {
            region_id: RegionId,
            proposal_id: ProposalId,
            proposer: T::AccountId,
            proposal_expiry: BlockNumberFor<T>,
        },
        /// Voted on proposal to remove region owner.
        VotedOnRegionOwnerProposal {
            region_id: RegionId,
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
            voting_power: T::Balance,
            new_yes_power: T::Balance,
            new_no_power: T::Balance,
        },
        /// A proposal for removing the region owner got rejected.
        RegionOwnerRemovalRejected { region_id: RegionId },
        /// A regional operator has been slashed.
        RegionalOperatorSlashed {
            region_id: RegionId,
            slashed_account: T::AccountId,
            amount: T::Balance,
            new_collateral_balance: T::Balance,
            new_active_strikes: u8,
        },
        /// The region is now eligible for an owner change after the specified block.
        RegionOwnerChangeEnabled {
            region_id: RegionId,
            next_change_allowed: BlockNumberFor<T>,
        },
        /// A bid for a region got placed.
        ReplacementBidSuccessfullyPlaced {
            region_id: RegionId,
            bidder: T::AccountId,
            new_leading_bid: T::Balance,
        },
        /// The owner of a region has been changed.
        RegionOwnerChanged {
            region_id: RegionId,
            new_owner: T::AccountId,
            next_owner_change: BlockNumberFor<T>,
        },
        /// The owner of a region has initiated resignation.
        RegionOwnerResignationInitiated {
            region_id: RegionId,
            region_owner: T::AccountId,
            next_owner_change: BlockNumberFor<T>,
        },
        /// Processing of a proposal failed.
        RegionOwnerProposalFailed {
            region_id: RegionId,
            error: DispatchResult,
        },
        /// Processing of a region owner replacement failed.
        RegionOwnerReplacementFailed {
            region_id: RegionId,
            error: DispatchResult,
        },
        /// A lawyer has been registered.
        LawyerRegistered {
            lawyer: T::AccountId,
            region_id: RegionId,
            deposit: T::Balance,
        },
        /// Lawyer has been unregistered.
        LawyerUnregistered {
            lawyer: T::AccountId,
            region_id: RegionId,
        },
        LawyerActiveCasesUpdated {
            lawyer_account: T::AccountId,
            new_active_cases: u32,
        },
        /// A user has unfrozen his token.
        TokenUnlocked {
            region_id: RegionId,
            proposal_id: ProposalId,
            voter: T::AccountId,
            amount: T::Balance,
        },
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
        ArithmeticUnderflow,
        /// This Region is not known.
        RegionUnknown,
        /// No sufficient permission.
        NoPermission,
        /// The location is already registered.
        LocationRegistered,
        /// The proposal is not ongoing.
        NotOngoing,
        /// There is no auction to bid on.
        NoOngoingAuction,
        /// The bid is lower than the current highest bid.
        BidTooLow,
        /// The bid is below the minimum.
        BidBelowMinimum,
        /// The voting has not ended yet.
        VotingStillOngoing,
        /// No Auction found.
        NoAuction,
        /// Auction is still ongoing.
        AuctionNotFinished,
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
        /// The user is not a regional operator.
        UserNotRegionalOperator,
        /// There is alerady a proposal ongoing for this region.
        ProposalAlreadyOngoing,
        /// There are already too many proposals in the ending block.
        TooManyProposals,
        /// Region owner cant be changed at the moment.
        RegionOwnerCantBeChanged,
        /// There are already too many auctions in the ending block.
        TooManyAuctions,
        /// Caller is not the region owner.
        NotRegionOwner,
        /// Owner would change before resignation period would be over.
        OwnerChangeAlreadyScheduled,
        /// The proposal could not be found.
        ProposalNotFound,
        /// The region has already been created.
        RegionAlreadyCreated,
        /// This region has an ongoing proposal.
        RegionProposalAlreadyExists,
        /// The caller does not have enough token to vote.
        NotEnoughTokenToVote,
        /// The auction does not have an winning bidder.
        RegionHasNoWinningBidder,
        /// The lawyer has already been registered.
        LawyerAlreadyRegistered,
        /// There are already too many voters for this voting.
        TooManyVoters,
        /// The account can has not lawyer permission.
        AccountNotLawyer,
        /// Lawyer is not registered.
        LawyerNotRegistered,
        /// The lawyer is still active in some cases.
        LawyerStillActive,
        /// The user has no token amount frozen.
        NoFrozenAmount,
        /// The token amount for voting is below minimum.
        BelowMinimumVotingAmount,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: frame_system::pallet_prelude::BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads_writes(1, 1);

            let ended_region_owner_votings = RegionOwnerRoundsExpiring::<T>::take(n);
            // checks if there is a voting for an proposal ending in this block.
            ended_region_owner_votings.iter().for_each(|item| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(6, 6));
                if let Err(e) = Self::finish_region_owner_proposal(*item) {
                    Self::deposit_event(Event::RegionOwnerProposalFailed {
                        region_id: *item,
                        error: Err(e),
                    });
                };
            });

            let ended_replacement_auction = ReplacementAuctionExpiring::<T>::take(n);
            // checks if there is a voting for an auction ending in this block.
            ended_replacement_auction.iter().for_each(|item| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));
                if let Err(e) = Self::finish_region_owner_replacement(*item, n) {
                    Self::deposit_event(Event::RegionOwnerReplacementFailed {
                        region_id: *item,
                        error: Err(e),
                    });
                };
            });
            weight
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a proposal for a new region.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_identifier`: The id of the region the caller is proposing.
        ///
        /// Emits `RegionProposed` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose_new_region())]
        pub fn propose_new_region(
            origin: OriginFor<T>,
            region_identifier: RegionIdentifier,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            let region_id = region_identifier.into_u16();

            ensure!(
                !ProposedRegionIds::<T>::contains_key(region_id),
                Error::<T>::RegionProposalAlreadyExists
            );
            ensure!(
                !RegionDetails::<T>::contains_key(region_id),
                Error::<T>::RegionAlreadyCreated
            );

            let proposal_id = ProposalCounter::<T>::get();
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            if let Some(last_proposal_block) = LastRegionProposalBlock::<T>::get() {
                let cooldown = T::RegionProposalCooldown::get();
                ensure!(
                    current_block_number.saturating_sub(last_proposal_block) >= cooldown,
                    Error::<T>::RegionProposalCooldownActive
                );
            }
            let deposit_amount = T::RegionProposalDeposit::get();
            T::NativeCurrency::hold(
                &HoldReason::RegionProposalReserve.into(),
                &signer,
                deposit_amount,
            )?;
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::RegionVotingTime::get());
            let proposal = RegionProposal {
                proposer: signer.clone(),
                created_at: current_block_number,
                proposal_expiry: expiry_block,
                deposit: deposit_amount,
            };
            let vote_stats = VoteStats {
                yes_voting_power: Zero::zero(),
                no_voting_power: Zero::zero(),
            };
            RegionProposalId::<T>::insert(region_id, proposal_id);
            ProposedRegionIds::<T>::insert(region_id, ());
            RegionProposals::<T>::insert(proposal_id, proposal);
            OngoingRegionProposalVotes::<T>::insert(proposal_id, vote_stats);
            LastRegionProposalBlock::<T>::put(current_block_number);
            let next_proposal_id = proposal_id
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCounter::<T>::put(next_proposal_id);
            Self::deposit_event(Event::RegionProposed {
                region_id,
                proposer: signer,
                proposal_id,
            });
            Ok(())
        }

        /// Lets a xcav holder vote on a proposal for a region.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: Id of the region.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount that the caller is using for voting.
        ///
        /// Emits `VotedOnRegionProposal` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_region_proposal())]
        pub fn vote_on_region_proposal(
            origin: OriginFor<T>,
            region_id: RegionId,
            vote: Vote,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let proposal_id =
                RegionProposalId::<T>::get(region_id).ok_or(Error::<T>::NotOngoing)?;
            let region_proposal =
                RegionProposals::<T>::get(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                region_proposal.proposal_expiry > current_block_number,
                Error::<T>::ProposalExpired
            );
            let free_balance = T::NativeCurrency::balance(&signer);
            let held_balance = T::NativeCurrency::balance_on_hold(
                &HoldReason::RegionVotingReserve.into(),
                &signer,
            );
            let total_available = free_balance.saturating_add(held_balance);
            ensure!(total_available >= amount, Error::<T>::NotEnoughTokenToVote);
            ensure!(
                amount >= T::MinimumVotingAmount::get(),
                Error::<T>::BelowMinimumVotingAmount
            );

            let mut new_yes_power = Default::default();
            let mut new_no_power = Default::default();

            OngoingRegionProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserRegionVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    if let Some(previous_vote) = maybe_vote_record.take() {
                        T::NativeCurrency::release(
                            &HoldReason::RegionVotingReserve.into(),
                            &signer,
                            previous_vote.power,
                            Precision::Exact,
                        )?;

                        match previous_vote.vote {
                            Vote::Yes => {
                                current_vote.yes_voting_power = current_vote
                                    .yes_voting_power
                                    .saturating_sub(previous_vote.power)
                            }
                            Vote::No => {
                                current_vote.no_voting_power = current_vote
                                    .no_voting_power
                                    .saturating_sub(previous_vote.power)
                            }
                        }
                    }

                    T::NativeCurrency::hold(
                        &HoldReason::RegionVotingReserve.into(),
                        &signer,
                        amount,
                    )?;

                    match vote {
                        Vote::Yes => {
                            current_vote.yes_voting_power =
                                current_vote.yes_voting_power.saturating_add(amount)
                        }
                        Vote::No => {
                            current_vote.no_voting_power =
                                current_vote.no_voting_power.saturating_add(amount)
                        }
                    }

                    new_yes_power = current_vote.yes_voting_power;
                    new_no_power = current_vote.no_voting_power;

                    *maybe_vote_record = Some(VoteRecord {
                        vote: vote.clone(),
                        region_id,
                        power: amount,
                    });
                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnRegionProposal {
                region_id,
                proposal_id,
                voter: signer,
                vote,
                voting_power: amount,
                new_yes_power,
                new_no_power,
            });
            Ok(())
        }

        /// Lets a voter unlock his locked token after voting on a region.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the region proposal.
        ///
        /// Emits `TokenUnlocked` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn unlock_region_voting_token(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record =
                UserRegionVote::<T>::get(proposal_id, &signer).ok_or(Error::<T>::NoFrozenAmount)?;

            if let Some(proposal) = RegionProposals::<T>::get(proposal_id) {
                let current_block_number = frame_system::Pallet::<T>::block_number();
                ensure!(
                    proposal.proposal_expiry <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            T::NativeCurrency::release(
                &HoldReason::RegionVotingReserve.into(),
                &signer,
                vote_record.power,
                Precision::Exact,
            )?;

            UserRegionVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::TokenUnlocked {
                region_id: vote_record.region_id,
                proposal_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets a registered account bid on a region to become the regional operator.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: Id of the region.
        /// - `amount`: The amount that the caller is willing to bid and to have locked.
        ///
        /// Emits `BidSuccessfullyPlaced` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::bid_on_region())]
        pub fn bid_on_region(
            origin: OriginFor<T>,
            region_id: RegionId,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;

            if let Some(proposal_id) = RegionProposalId::<T>::get(region_id) {
                let region_proposal =
                    RegionProposals::<T>::get(proposal_id).ok_or(Error::<T>::NotOngoing)?;
                let current_block_number = <frame_system::Pallet<T>>::block_number();
                ensure!(
                    region_proposal.proposal_expiry <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
                let auction_started =
                    Self::finalize_region_proposal(region_id, current_block_number)?;
                if !auction_started {
                    return Ok(());
                }
            }

            ensure!(!amount.is_zero(), Error::<T>::BidCannotBeZero);
            RegionAuctions::<T>::try_mutate(region_id, |maybe_auction| -> DispatchResult {
                let auction = maybe_auction.as_mut().ok_or(Error::<T>::NoOngoingAuction)?;
                let current_block_number = <frame_system::Pallet<T>>::block_number();
                let previous_highest_bidder = auction.highest_bidder.clone();
                ensure!(
                    auction.auction_expiry > current_block_number,
                    Error::<T>::NoOngoingAuction
                );
                match &previous_highest_bidder {
                    Some(old_bidder) => {
                        ensure!(amount > auction.collateral, Error::<T>::BidTooLow);
                        if old_bidder == &signer {
                            let additional = amount.saturating_sub(auction.collateral);
                            if additional > Zero::zero() {
                                T::NativeCurrency::hold(
                                    &HoldReason::RegionDepositReserve.into(),
                                    &signer,
                                    additional,
                                )?;
                            }
                        } else {
                            T::NativeCurrency::hold(
                                &HoldReason::RegionDepositReserve.into(),
                                &signer,
                                amount,
                            )?;
                            T::NativeCurrency::release(
                                &HoldReason::RegionDepositReserve.into(),
                                old_bidder,
                                auction.collateral,
                                Precision::Exact,
                            )?;
                        }
                    }
                    None => {
                        ensure!(amount >= auction.collateral, Error::<T>::BidBelowMinimum);
                        T::NativeCurrency::hold(
                            &HoldReason::RegionDepositReserve.into(),
                            &signer,
                            amount,
                        )?;
                    }
                }
                auction.highest_bidder = Some(signer.clone());
                auction.collateral = amount;
                Self::deposit_event(Event::BidSuccessfullyPlaced {
                    region_id,
                    bidder: signer,
                    new_leading_bid: amount,
                    previous_bidder: previous_highest_bidder,
                });
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }

        /// Creates a new region for the marketplace.
        /// This function calls the nfts-pallet to create a new collection.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: Id of the region.
        /// - `listing_duration`: Duration of a listing in this region.
        /// - `tax`: Tax percentage for selling a property in this region.
        ///
        /// Emits `RegionCreated` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_new_region())]
        pub fn create_new_region(
            origin: OriginFor<T>,
            region_id: RegionId,
            listing_duration: BlockNumberFor<T>,
            tax: Permill,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            let auction = RegionAuctions::<T>::get(region_id).ok_or(Error::<T>::NoAuction)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                auction.auction_expiry <= current_block_number,
                Error::<T>::AuctionNotFinished
            );

            let region_owner = auction
                .highest_bidder
                .ok_or(Error::<T>::RegionHasNoWinningBidder)?;

            ensure!(region_owner == signer, Error::<T>::NotRegionOwner);

            if auction.collateral.is_zero() {
                Self::deposit_event(Event::<T>::NoRegionCreated { region_id });
                return Ok(());
            }
            ensure!(
                !listing_duration.is_zero(),
                Error::<T>::ListingDurationCantBeZero
            );
            ensure!(
                listing_duration <= T::MaxListingDuration::get(),
                Error::<T>::ListingDurationTooHigh
            );

            let pallet_id: T::AccountId = Self::account_id();
            let collection_id = <T as pallet::Config>::Nfts::create_collection(
                &pallet_id,
                &pallet_id,
                &Self::default_collection_config(),
            )?;

            let next_owner_change =
                current_block_number.saturating_add(T::RegionOwnerChangePeriod::get());

            let region_info = RegionInfo {
                collection_id,
                listing_duration,
                owner: region_owner.clone(),
                collateral: auction.collateral,
                active_strikes: Default::default(),
                tax,
                next_owner_change,
                location_count: Default::default(),
            };
            RegionDetails::<T>::insert(region_id, region_info);
            RegionAuctions::<T>::get(region_id);
            ProposedRegionIds::<T>::remove(region_id);

            Self::deposit_event(Event::<T>::RegionCreated {
                region_id,
                collection_id,
                owner: region_owner,
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
        /// - `region_id`: Region in where the listing duration should be changed.
        /// - `listing_duration`: New duration of a listing in this region.
        ///
        /// Emits `ListingDurationChanged` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::adjust_listing_duration())]
        pub fn adjust_listing_duration(
            origin: OriginFor<T>,
            region_id: RegionId,
            listing_duration: BlockNumberFor<T>,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            ensure!(
                !listing_duration.is_zero(),
                Error::<T>::ListingDurationCantBeZero
            );
            ensure!(
                listing_duration <= T::MaxListingDuration::get(),
                Error::<T>::ListingDurationTooHigh
            );

            RegionDetails::<T>::try_mutate(region_id, |maybe_region| {
                let region = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
                ensure!(signer == region.owner, Error::<T>::NoPermission);

                region.listing_duration = listing_duration;
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::<T>::ListingDurationChanged {
                region_id,
                listing_duration,
            });
            Ok(())
        }

        /// Region owner can adjust the tax in a region.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: Region in where the tax should be changed.
        /// - `new_tax`: New tax for a property sell in this region.
        ///
        /// Emits `RegionTaxChanged` event when successful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::adjust_region_tax())]
        pub fn adjust_region_tax(
            origin: OriginFor<T>,
            region_id: RegionId,
            new_tax: Permill,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;

            RegionDetails::<T>::try_mutate(region_id, |maybe_region| {
                let region = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
                ensure!(region.owner == signer, Error::<T>::NoPermission);

                region.tax = new_tax;
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::<T>::RegionTaxChanged { region_id, new_tax });
            Ok(())
        }

        /// Creates a new location for a region.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: The region where the new location should be created.
        /// - `location`: The postcode of the new location.
        ///
        /// Emits `LocationCreated` event when successful.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_new_location())]
        pub fn create_new_location(
            origin: OriginFor<T>,
            region_id: RegionId,
            location: LocationId<T>,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            ensure!(
                !LocationRegistration::<T>::contains_key(region_id, &location),
                Error::<T>::LocationRegistered
            );
            let deposit_amount = T::LocationDeposit::get();

            let mut new_collateral_balance = 0u32.into();
            let mut new_location_count = 0u32;

            RegionDetails::<T>::try_mutate(region_id, |maybe_region| {
                let region_info = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
                ensure!(region_info.owner == signer, Error::<T>::NoPermission);

                region_info.collateral = region_info
                    .collateral
                    .checked_add(&deposit_amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                region_info.location_count = region_info
                    .location_count
                    .checked_add(1)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                T::NativeCurrency::hold(
                    &HoldReason::RegionDepositReserve.into(),
                    &signer,
                    deposit_amount,
                )?;

                new_collateral_balance = region_info.collateral;
                new_location_count = region_info.location_count;

                Ok::<(), DispatchError>(())
            })?;

            LocationRegistration::<T>::insert(region_id, &location, true);
            Self::deposit_event(Event::<T>::LocationCreated {
                region_id,
                location_id: location,
                new_collateral_balance,
                new_location_count,
            });
            Ok(())
        }

        /// Creates proposal to remove a region owner.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region owner should be removed.
        ///
        /// Emits `RemoveRegionOwnerProposed` event when successful.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose_remove_regional_operator())]
        pub fn propose_remove_regional_operator(
            origin: OriginFor<T>,
            region_id: RegionId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(
                RegionDetails::<T>::contains_key(region_id),
                Error::<T>::RegionUnknown
            );
            ensure!(
                RegionOwnerProposalId::<T>::get(region_id).is_none(),
                Error::<T>::ProposalAlreadyOngoing
            );
            let deposit_amount = T::RegionOwnerDisputeDeposit::get();
            T::NativeCurrency::hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &signer,
                deposit_amount,
            )?;
            let proposal_id = ProposalCounter::<T>::get();
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let expiry_block =
                current_block_number.saturating_add(T::RegionOperatorVotingTime::get());
            let proposal = RemoveRegionOwnerProposal {
                proposer: signer.clone(),
                proposal_expiry: expiry_block,
                deposit: deposit_amount,
            };

            RegionOwnerRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
                keys.try_push(region_id)
                    .map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;
            let vote_stats = VoteStats {
                yes_voting_power: Zero::zero(),
                no_voting_power: Zero::zero(),
            };
            RegionOwnerProposalId::<T>::insert(region_id, proposal_id);
            RegionOwnerProposals::<T>::insert(proposal_id, proposal);
            OngoingRegionOwnerProposalVotes::<T>::insert(proposal_id, vote_stats);
            let next_proposal_id = proposal_id
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCounter::<T>::put(next_proposal_id);
            Self::deposit_event(Event::<T>::RemoveRegionOwnerProposed {
                region_id,
                proposal_id,
                proposer: signer,
                proposal_expiry: expiry_block,
            });
            Ok(())
        }

        /// Vote on proposal to remove a region owner.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region owner should be removed.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount that the caller is using for voting.
        ///
        /// Emits `VotedOnRegionOwnerProposal` event when successful.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_remove_owner_proposal())]
        pub fn vote_on_remove_owner_proposal(
            origin: OriginFor<T>,
            region_id: RegionId,
            vote: Vote,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let proposal_id =
                RegionOwnerProposalId::<T>::get(region_id).ok_or(Error::<T>::NotOngoing)?;
            let free_balance = T::NativeCurrency::balance(&signer);
            let held_balance = T::NativeCurrency::balance_on_hold(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &signer,
            );
            let total_available = free_balance.saturating_add(held_balance);
            ensure!(total_available >= amount, Error::<T>::NotEnoughTokenToVote);
            ensure!(
                amount >= T::MinimumVotingAmount::get(),
                Error::<T>::BelowMinimumVotingAmount
            );

            let mut new_yes_power = Default::default();
            let mut new_no_power = Default::default();

            OngoingRegionOwnerProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserRegionOwnerVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    if let Some(previous_vote) = maybe_vote_record.take() {
                        T::NativeCurrency::release(
                            &HoldReason::RegionOperatorRemovalVoting.into(),
                            &signer,
                            previous_vote.power,
                            Precision::Exact,
                        )?;

                        match previous_vote.vote {
                            Vote::Yes => {
                                current_vote.yes_voting_power = current_vote
                                    .yes_voting_power
                                    .saturating_sub(previous_vote.power)
                            }
                            Vote::No => {
                                current_vote.no_voting_power = current_vote
                                    .no_voting_power
                                    .saturating_sub(previous_vote.power)
                            }
                        }
                    }

                    T::NativeCurrency::hold(
                        &HoldReason::RegionOperatorRemovalVoting.into(),
                        &signer,
                        amount,
                    )?;

                    match vote {
                        Vote::Yes => {
                            current_vote.yes_voting_power =
                                current_vote.yes_voting_power.saturating_add(amount)
                        }
                        Vote::No => {
                            current_vote.no_voting_power =
                                current_vote.no_voting_power.saturating_add(amount)
                        }
                    }

                    new_yes_power = current_vote.yes_voting_power;
                    new_no_power = current_vote.no_voting_power;

                    *maybe_vote_record = Some(VoteRecord {
                        vote: vote.clone(),
                        region_id,
                        power: amount,
                    });

                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnRegionOwnerProposal {
                region_id,
                proposal_id,
                voter: signer,
                vote,
                voting_power: amount,
                new_yes_power,
                new_no_power,
            });
            Ok(())
        }

        /// Lets a voter unlock his locked token after voting on removal of a regional operator.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the region proposal.
        ///
        /// Emits `TokenUnlocked` event when successful.
        #[pallet::call_index(10)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn unlock_region_onwer_removal_voting_token(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record = UserRegionOwnerVote::<T>::get(proposal_id, &signer)
                .ok_or(Error::<T>::NoFrozenAmount)?;

            if let Some(proposal) = RegionOwnerProposals::<T>::get(proposal_id) {
                let current_block_number = frame_system::Pallet::<T>::block_number();
                ensure!(
                    proposal.proposal_expiry <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            T::NativeCurrency::release(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &signer,
                vote_record.power,
                Precision::Exact,
            )?;

            UserRegionOwnerVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::TokenUnlocked {
                region_id: vote_record.region_id,
                proposal_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets a registered account bid on a region to become the new regional operator.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region owner should be removed.
        /// - `amount`: The amount that the caller is willing to bid and to have locked.
        ///
        /// Emits `ReplacementBidSuccessfullyPlaced` event when successful.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::bid_on_region_replacement())]
        pub fn bid_on_region_replacement(
            origin: OriginFor<T>,
            region_id: RegionId,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            let region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                region_info.next_owner_change < current_block_number,
                Error::<T>::RegionOwnerCantBeChanged
            );

            let mut new_auction = false;

            RegionReplacementAuctions::<T>::try_mutate(region_id, |maybe_auction| {
                let auction = maybe_auction.get_or_insert_with(|| {
                    new_auction = true;

                    let mut minimum_deposit = T::MinimumRegionDeposit::get();
                    let location_deposits =
                        T::LocationDeposit::get().saturating_mul(region_info.location_count.into());
                    minimum_deposit = minimum_deposit.saturating_add(location_deposits);

                    RegionAuction {
                        highest_bidder: None,
                        collateral: minimum_deposit,
                        auction_expiry: current_block_number
                            .saturating_add(T::RegionAuctionTime::get()),
                    }
                });
                ensure!(
                    auction.auction_expiry > current_block_number,
                    Error::<T>::NoOngoingAuction
                );
                match &auction.highest_bidder {
                    Some(old_bidder) => {
                        ensure!(amount > auction.collateral, Error::<T>::BidTooLow);
                        if old_bidder == &signer {
                            let additional = amount.saturating_sub(auction.collateral);
                            if additional > Zero::zero() {
                                T::NativeCurrency::hold(
                                    &HoldReason::RegionDepositReserve.into(),
                                    &signer,
                                    additional,
                                )?;
                            }
                        } else {
                            T::NativeCurrency::hold(
                                &HoldReason::RegionDepositReserve.into(),
                                &signer,
                                amount,
                            )?;
                            T::NativeCurrency::release(
                                &HoldReason::RegionDepositReserve.into(),
                                old_bidder,
                                auction.collateral,
                                Precision::Exact,
                            )?;
                        }
                    }
                    None => {
                        ensure!(amount >= auction.collateral, Error::<T>::BidBelowMinimum);
                        T::NativeCurrency::hold(
                            &HoldReason::RegionDepositReserve.into(),
                            &signer,
                            amount,
                        )?;
                    }
                }
                auction.highest_bidder = Some(signer.clone());
                auction.collateral = amount;
                Ok::<(), DispatchError>(())
            })?;

            // Register expiry only if auction was newly created
            if new_auction {
                let expiry_block = current_block_number.saturating_add(T::RegionAuctionTime::get());
                ReplacementAuctionExpiring::<T>::try_mutate(expiry_block, |keys| {
                    keys.try_push(region_id)
                        .map_err(|_| Error::<T>::TooManyAuctions)?;
                    Ok::<(), DispatchError>(())
                })?;
            }

            Self::deposit_event(Event::ReplacementBidSuccessfullyPlaced {
                region_id,
                bidder: signer,
                new_leading_bid: amount,
            });
            Ok(())
        }

        /// Lets a regional operator resign.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region wants to resign.
        ///
        /// Emits `RegionOwnerResignationInitiated` event when successful.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::initiate_region_owner_resignation())]
        pub fn initiate_region_owner_resignation(
            origin: OriginFor<T>,
            region_id: RegionId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;

            RegionDetails::<T>::try_mutate(region_id, |maybe_region| -> DispatchResult {
                let region_info = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
                ensure!(region_info.owner == signer, Error::<T>::NotRegionOwner);

                let current_block_number = <frame_system::Pallet<T>>::block_number();
                let next_owner_change =
                    current_block_number.saturating_add(T::RegionOwnerNoticePeriod::get());
                ensure!(
                    region_info.next_owner_change > next_owner_change,
                    Error::<T>::OwnerChangeAlreadyScheduled
                );
                region_info.next_owner_change = next_owner_change;

                Self::deposit_event(Event::RegionOwnerResignationInitiated {
                    region_id,
                    region_owner: signer,
                    next_owner_change: region_info.next_owner_change,
                });
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }

        /// Registers a new lawyer.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `lawyer`: The lawyer that should be registered.
        ///
        /// Emits `LawyerRegistered` event when successful.
        #[pallet::call_index(13)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::register_lawyer())]
        pub fn register_lawyer(origin: OriginFor<T>, region: RegionId) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::Lawyer,
            )?;
            ensure!(
                RegionDetails::<T>::contains_key(region),
                Error::<T>::RegionUnknown
            );
            ensure!(
                RealEstateLawyer::<T>::get(&signer).is_none(),
                Error::<T>::LawyerAlreadyRegistered
            );
            let deposit_amount = T::LawyerDeposit::get();
            T::NativeCurrency::hold(
                &HoldReason::LawyerDepositReserve.into(),
                &signer,
                deposit_amount,
            )?;
            let lawyer_info = LawyerInfo {
                region,
                deposit: deposit_amount,
                active_cases: 0,
            };
            RealEstateLawyer::<T>::insert(&signer, lawyer_info);
            Self::deposit_event(Event::<T>::LawyerRegistered {
                lawyer: signer,
                region_id: region,
                deposit: deposit_amount,
            });
            Ok(())
        }

        /// Unegisters a new lawyer.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `lawyer`: The lawyer that should be runegistered.
        ///
        /// Emits `LawyerUnregistered` event when successful.
        #[pallet::call_index(14)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn unregister_lawyer(origin: OriginFor<T>, region: RegionId) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::Lawyer,
            )?;
            ensure!(
                RegionDetails::<T>::contains_key(region),
                Error::<T>::RegionUnknown
            );
            let lawyer_info =
                RealEstateLawyer::<T>::get(&signer).ok_or(Error::<T>::NoPermission)?;
            ensure!(
                lawyer_info.active_cases.is_zero(),
                Error::<T>::LawyerStillActive
            );
            <T as pallet::Config>::NativeCurrency::release(
                &HoldReason::LawyerDepositReserve.into(),
                &signer,
                lawyer_info.deposit,
                Precision::Exact,
            )?;
            RealEstateLawyer::<T>::remove(&signer);
            Self::deposit_event(Event::<T>::LawyerUnregistered {
                lawyer: signer,
                region_id: region,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get the account id of the pallet
        pub fn account_id() -> T::AccountId {
            <T as pallet::Config>::PalletId::get().into_account_truncating()
        }

        /// Get the account id of the treasury pallet
        pub fn treasury_account_id() -> T::AccountId {
            T::TreasuryId::get().into_account_truncating()
        }

        /// Process a proposal for a new region.
        fn finalize_region_proposal(
            region_id: RegionId,
            current_block_number: BlockNumberFor<T>,
        ) -> Result<bool, DispatchError> {
            let proposal_id =
                RegionProposalId::<T>::take(region_id).ok_or(Error::<T>::NotOngoing)?;
            let voting_results =
                OngoingRegionProposalVotes::<T>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let proposal = RegionProposals::<T>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let total_voting_amount = voting_results
                .yes_voting_power
                .checked_add(&voting_results.no_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let threshold_percent: T::Balance = T::RegionThreshold::get().deconstruct().into();

            let meets_threshold = !total_voting_amount.is_zero()
                && voting_results
                    .yes_voting_power
                    .saturating_mul(100u32.into())
                    >= total_voting_amount.saturating_mul(threshold_percent);

            let auction_expiry_block =
                current_block_number.saturating_add(T::RegionAuctionTime::get());
            if meets_threshold {
                T::NativeCurrency::release(
                    &HoldReason::RegionProposalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                    Precision::Exact,
                )?;
                let treasury_account = Self::treasury_account_id();
                if T::NativeCurrency::balance(&treasury_account) >= proposal.deposit {
                    T::NativeCurrency::transfer(
                        &treasury_account,
                        &proposal.proposer,
                        proposal.deposit,
                        Preservation::Expendable,
                    )?;
                }
                let auction = RegionAuction {
                    highest_bidder: None,
                    collateral: T::MinimumRegionDeposit::get(),
                    auction_expiry: auction_expiry_block,
                };
                RegionAuctions::<T>::insert(region_id, auction);
                Self::deposit_event(Event::RegionAuctionStarted { region_id });
                Ok(true)
            } else {
                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &HoldReason::RegionProposalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                );

                T::Slash::on_unbalanced(imbalance);
                ProposedRegionIds::<T>::remove(region_id);
                Self::deposit_event(Event::RegionProposalRejected {
                    region_id,
                    slashed_account: proposal.proposer,
                    amount: proposal.deposit,
                });
                Ok(false)
            }
        }

        /// Processes a proposal for removing a regional operator.
        fn finish_region_owner_proposal(region_id: RegionId) -> DispatchResult {
            let proposal_id =
                RegionOwnerProposalId::<T>::take(region_id).ok_or(Error::<T>::ProposalNotFound)?;
            let proposal =
                RegionOwnerProposals::<T>::take(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
            let voting_result = OngoingRegionOwnerProposalVotes::<T>::take(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;

            let total_voting_amount = voting_result
                .yes_voting_power
                .checked_add(&voting_result.no_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let threshold_percent: T::Balance = T::RegionThreshold::get().deconstruct().into();

            let meets_threshold = !total_voting_amount.is_zero()
                && voting_result.yes_voting_power.saturating_mul(100u32.into())
                    >= total_voting_amount.saturating_mul(threshold_percent);

            if meets_threshold {
                let updated_strikes = Self::slash_region_owner(region_id)?;
                if updated_strikes >= 3 {
                    Self::enable_region_owner_change(region_id)?;
                }
                T::NativeCurrency::release(
                    &HoldReason::RegionalOperatorRemovalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                    Precision::Exact,
                )?;
            } else {
                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &HoldReason::RegionalOperatorRemovalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                );

                T::Slash::on_unbalanced(imbalance);
                Self::deposit_event(Event::RegionOwnerRemovalRejected { region_id });
            }

            Ok(())
        }

        // Slashes the region owner.
        fn slash_region_owner(region_id: RegionId) -> Result<u8, DispatchError> {
            let mut region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;
            let amount = <T as Config>::RegionSlashingAmount::get();

            let region_owner = region_info.owner.clone();

            let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                &HoldReason::RegionDepositReserve.into(),
                &region_owner,
                amount,
            );

            T::Slash::on_unbalanced(imbalance);

            region_info.collateral = region_info.collateral.saturating_sub(amount);
            region_info.active_strikes = region_info.active_strikes.saturating_add(1);

            let updated_strikes = region_info.active_strikes;

            let new_collateral_balance = region_info.collateral;
            let new_active_strikes = region_info.active_strikes;

            RegionDetails::<T>::insert(region_id, region_info);
            Self::deposit_event(Event::RegionalOperatorSlashed {
                region_id,
                slashed_account: region_owner,
                amount,
                new_collateral_balance,
                new_active_strikes,
            });
            Ok(updated_strikes)
        }

        /// Enable changing the regional operator of a given region.
        fn enable_region_owner_change(region_id: RegionId) -> DispatchResult {
            let mut region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            region_info.next_owner_change = current_block_number;
            RegionDetails::<T>::insert(region_id, region_info);
            Self::deposit_event(Event::RegionOwnerChangeEnabled {
                region_id,
                next_change_allowed: current_block_number,
            });
            Ok(())
        }

        /// Change the regional operator of a given region.
        fn finish_region_owner_replacement(
            region_id: RegionId,
            current_block_number: BlockNumberFor<T>,
        ) -> DispatchResult {
            let auction_info =
                RegionReplacementAuctions::<T>::take(region_id).ok_or(Error::<T>::NoAuction)?;
            let mut region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;
            if let Some(new_owner) = auction_info.highest_bidder {
                T::NativeCurrency::release(
                    &HoldReason::RegionDepositReserve.into(),
                    &region_info.owner,
                    region_info.collateral,
                    Precision::Exact,
                )?;
                let next_owner_change =
                    current_block_number.saturating_add(T::RegionOwnerChangePeriod::get());
                region_info.owner = new_owner.clone();
                region_info.collateral = auction_info.collateral;
                region_info.next_owner_change = next_owner_change;
                region_info.active_strikes = Default::default();
                RegionDetails::<T>::insert(region_id, region_info);
                Self::deposit_event(Event::<T>::RegionOwnerChanged {
                    region_id,
                    new_owner,
                    next_owner_change,
                });
            }
            Ok(())
        }

        /// Set the default collection configuration for creating a collection.
        fn default_collection_config(
        ) -> CollectionConfig<T::Balance, BlockNumberFor<T>, <T as pallet_nfts::Config>::CollectionId>
        {
            Self::collection_config_with_all_settings_enabled()
        }

        fn collection_config_with_all_settings_enabled(
        ) -> CollectionConfig<T::Balance, BlockNumberFor<T>, <T as pallet_nfts::Config>::CollectionId>
        {
            CollectionConfig {
                settings: CollectionSettings::all_enabled(),
                max_supply: None,
                mint_settings: MintSettings::default(),
            }
        }
    }
}

pub trait LawyerManagement<T: Config> {
    fn increment_active_cases(lawyer: &<T as frame_system::Config>::AccountId) -> DispatchResult;
    fn decrement_active_cases(lawyer: &<T as frame_system::Config>::AccountId) -> DispatchResult;
}

impl<T: Config> LawyerManagement<T> for Pallet<T> {
    fn increment_active_cases(lawyer: &<T as frame_system::Config>::AccountId) -> DispatchResult {
        RealEstateLawyer::<T>::try_mutate(lawyer, |maybe_lawyer_info| {
            let lawyer_info = maybe_lawyer_info
                .as_mut()
                .ok_or(Error::<T>::LawyerNotRegistered)?;
            lawyer_info.active_cases = lawyer_info
                .active_cases
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            Self::deposit_event(Event::LawyerActiveCasesUpdated {
                lawyer_account: lawyer.clone(),
                new_active_cases: lawyer_info.active_cases,
            });
            Ok(())
        })
    }

    fn decrement_active_cases(lawyer: &<T as frame_system::Config>::AccountId) -> DispatchResult {
        RealEstateLawyer::<T>::try_mutate(lawyer, |maybe_lawyer_info| {
            let lawyer_info = maybe_lawyer_info
                .as_mut()
                .ok_or(Error::<T>::LawyerNotRegistered)?;
            lawyer_info.active_cases = lawyer_info
                .active_cases
                .checked_sub(1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            Self::deposit_event(Event::LawyerActiveCasesUpdated {
                lawyer_account: lawyer.clone(),
                new_active_cases: lawyer_info.active_cases,
            });
            Ok(())
        })
    }
}
