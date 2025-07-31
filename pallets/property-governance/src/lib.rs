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

use frame_support::{
    sp_runtime::{traits::AccountIdConversion, Percent, Saturating},
    traits::{
        fungible::{BalancedHold, Credit},
        fungibles::{Mutate as FungiblesMutate, MutateHold as FungiblesMutateHold},
        tokens::{fungible, fungibles},
        tokens::{imbalance::OnUnbalanced, Balance, Precision, Preservation},
    },
    PalletId,
};

use codec::{Codec, DecodeWithMemTracking};

use primitives::MarketplaceHoldReason;

use pallet_real_estate_asset::traits::{
    PropertyTokenInspect, PropertyTokenManage, PropertyTokenOwnership, PropertyTokenSpvControl,
};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type RuntimeHoldReasonOf<T> = <T as pallet_property_management::Config>::RuntimeHoldReason;

pub type NegativeImbalanceOf<T> =
    Credit<<T as frame_system::Config>::AccountId, <T as Config>::NativeCurrency>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    pub type ProposalIndex = u32;

    /// Proposal with the proposal Details.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub proposer: AccountIdOf<T>,
        pub asset_id: u32,
        pub amount: <T as pallet::Config>::Balance,
        pub created_at: BlockNumberFor<T>,
        pub metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
    }

    /// Sale proposal with the proposal Details.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct PropertySaleProposal<T: Config> {
        pub proposer: AccountIdOf<T>,
        pub created_at: BlockNumberFor<T>,
    }

    /// Challenge with the challenge Details.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Challenge<T: Config> {
        pub proposer: AccountIdOf<T>,
        pub created_at: BlockNumberFor<T>,
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

    /// Current status of the sale process.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    pub enum DocumentStatus {
        Pending,
        Approved,
        Rejected,
    }

    /// Legal sites for lawyers to represent in a sale.
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
    pub enum LegalSale {
        SpvSide,
        BuyerSide,
    }

    /// Voting stats.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    pub struct VoteStats {
        pub yes_voting_power: u32,
        pub no_voting_power: u32,
    }

    /// Voting stats.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct PropertySaleInfo<T: Config> {
        pub spv_lawyer: Option<AccountIdOf<T>>,
        pub buyer_lawyer: Option<AccountIdOf<T>>,
        pub buyer: Option<AccountIdOf<T>>,
        pub spv_status: DocumentStatus,
        pub buyer_status: DocumentStatus,
        pub spv_lawyer_costs: <T as pallet::Config>::Balance,
        pub buyer_lawyer_costs: <T as pallet::Config>::Balance,
        pub price: Option<<T as pallet::Config>::Balance>,
        pub second_attempt: bool,
        pub lawyer_approved: bool,
        pub finalized: bool,
        pub property_token_amount: u32,
        pub reserve: Option<Reserve<T>>,
    }

    /// Info for sale auctions.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct SaleAuction<T: Config> {
        pub highest_bidder: Option<AccountIdOf<T>>,
        pub price: <T as pallet::Config>::Balance,
        pub reserve: Option<Reserve<T>>,
    }

    /// Reserve of an auction.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Reserve<T: Config> {
        pub payment_asset: u32,
        pub amount: <T as pallet::Config>::Balance,
    }

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_property_management::Config
        + pallet_xcavate_whitelist::Config
        + pallet_regions::Config
        + pallet_real_estate_asset::Config
    {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;

        type Balance: Balance
            + TypeInfo
            + From<u128>
            + Into<<Self as pallet_real_estate_asset::Config>::Balance>
            + Default;

        /// The reservable currency type.
        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::MutateHold<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                Reason = RuntimeHoldReasonOf<Self>,
            > + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        type LocalCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        type ForeignCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        type ForeignAssetsHolder: fungibles::MutateHold<
                AccountIdOf<Self>,
                AssetId = u32,
                Balance = <Self as pallet::Config>::Balance,
                Reason = MarketplaceHoldReason,
            > + fungibles::InspectHold<AccountIdOf<Self>, AssetId = u32>;

        /// The amount of time given to vote for a proposal.
        #[pallet::constant]
        type VotingTime: Get<BlockNumberFor<Self>>;

        /// The amount of time give to vote for a sale proposal.
        #[pallet::constant]
        type SaleVotingTime: Get<BlockNumberFor<Self>>;

        /// The maximum amount of votes per block.
        #[pallet::constant]
        type MaxVotesForBlock: Get<u32>;

        /// The minimum amount of a letting agent that will be slashed.
        #[pallet::constant]
        type MinSlashingAmount: Get<<Self as pallet::Config>::Balance>;

        /// Threshold for challenge votes.
        #[pallet::constant]
        type Threshold: Get<Percent>;

        /// Threshold for high costs challenge votes.
        #[pallet::constant]
        type HighThreshold: Get<Percent>;

        /// Proposal amount to be considered a low proposal.
        #[pallet::constant]
        type LowProposal: Get<<Self as pallet::Config>::Balance>;

        /// Proposal amount to be considered a high proposal.
        #[pallet::constant]
        type HighProposal: Get<<Self as pallet::Config>::Balance>;

        /// The property governance's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type MarketplacePalletId: Get<PalletId>;

        /// Threshold for selling a property.
        #[pallet::constant]
        type SaleApprovalYesThreshold: Get<Percent>;

        /// Time of auctions of a property sale.
        #[pallet::constant]
        type AuctionTime: Get<BlockNumberFor<Self>>;

        /// Handler for the unbalanced reduction when slashing a letting agent.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        #[pallet::constant]
        type AcceptedAssets: Get<[u32; 2]>;

        /// The Trasury's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryId: Get<PalletId>;

        type PropertyToken: PropertyTokenManage<Self>
            + PropertyTokenOwnership<Self>
            + PropertyTokenSpvControl<Self>
            + PropertyTokenInspect<Self>;
    }

    pub type LocationId<T> = BoundedVec<u8, <T as pallet_regions::Config>::PostcodeLimit>;

    /// Number of proposals that have been made.
    #[pallet::storage]
    pub(super) type ProposalCount<T> = StorageValue<_, ProposalIndex, ValueQuery>;

    /// Proposals that have been made.
    #[pallet::storage]
    pub(super) type Proposals<T> =
        StorageMap<_, Blake2_128Concat, ProposalIndex, Proposal<T>, OptionQuery>;

    /// Sell proposals that have been made.
    #[pallet::storage]
    pub(super) type SaleProposals<T> =
        StorageMap<_, Blake2_128Concat, u32, PropertySaleProposal<T>, OptionQuery>;

    /// Mapping of challenge index to the challenge info.
    #[pallet::storage]
    pub(super) type Challenges<T> = StorageMap<_, Blake2_128Concat, u32, Challenge<T>, OptionQuery>;

    /// Mapping of ongoing votes.
    #[pallet::storage]
    pub(super) type OngoingProposalVotes<T> =
        StorageMap<_, Blake2_128Concat, ProposalIndex, VoteStats, OptionQuery>;

    /// Mapping of a proposal id and account id to the vote of a user.
    #[pallet::storage]
    pub(super) type UserProposalVote<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ProposalIndex,
        BoundedBTreeMap<
            AccountIdOf<T>,
            Vote,
            <T as pallet_real_estate_asset::Config>::MaxPropertyToken,
        >,
        OptionQuery,
    >;

    /// Mapping of ongoing sales votes.
    #[pallet::storage]
    pub(super) type OngoingSaleProposalVotes<T> =
        StorageMap<_, Blake2_128Concat, u32, VoteStats, OptionQuery>;

    /// Mapping of a proposal id and account id to the vote of the user.
    #[pallet::storage]
    pub(super) type UserSaleProposalVote<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        BoundedBTreeMap<
            AccountIdOf<T>,
            Vote,
            <T as pallet_real_estate_asset::Config>::MaxPropertyToken,
        >,
        OptionQuery,
    >;

    /// Mapping of ongoing votes about challenges.
    #[pallet::storage]
    pub(super) type OngoingChallengeVotes<T> =
        StorageMap<_, Blake2_128Concat, u32, VoteStats, OptionQuery>;

    /// Mapping of a proposal id and account id to the vote of the user.
    #[pallet::storage]
    pub(super) type UserChallengeVote<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        BoundedBTreeMap<
            AccountIdOf<T>,
            Vote,
            <T as pallet_real_estate_asset::Config>::MaxPropertyToken,
        >,
        OptionQuery,
    >;

    /// Stores the project keys and round types ending on a given block for proposal votings.
    #[pallet::storage]
    pub type ProposalRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<ProposalIndex, T::MaxVotesForBlock>,
        ValueQuery,
    >;

    /// Stores the project keys and round types ending on a give block for sale proposal votings.
    #[pallet::storage]
    pub type SaleProposalRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxVotesForBlock>,
        ValueQuery,
    >;

    /// Stores the project keys and round types ending on a given block for challenge votings.
    #[pallet::storage]
    pub type ChallengeRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxVotesForBlock>,
        ValueQuery,
    >;

    /// Mapping from asset id to the property sale details.
    #[pallet::storage]
    pub type PropertySale<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, PropertySaleInfo<T>, OptionQuery>;

    /// Mapping of asset id to infos about an auction.
    #[pallet::storage]
    pub type SaleAuctions<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, SaleAuction<T>, OptionQuery>;

    /// Stores the project keys and round types ending on a given block for auctions.
    #[pallet::storage]
    pub type AuctionRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxVotesForBlock>,
        ValueQuery,
    >;

    /// Stores the funds from a property sale.
    #[pallet::storage]
    pub type PropertySaleFunds<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,
        Blake2_128Concat,
        u32,
        <T as pallet::Config>::Balance,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New proposal has been created.
        Proposed {
            proposal_id: ProposalIndex,
            asset_id: u32,
            proposer: AccountIdOf<T>,
        },
        /// A new challenge has been made.
        Challenge {
            asset_id: u32,
            proposer: AccountIdOf<T>,
        },
        /// Voted on proposal.
        VotedOnProposal {
            proposal_id: ProposalIndex,
            voter: AccountIdOf<T>,
            vote: Vote,
        },
        /// Voted on sale proposal.
        VotedOnPropertySaleProposal {
            asset_id: u32,
            voter: AccountIdOf<T>,
            vote: Vote,
        },
        /// Voted on challenge.
        VotedOnChallenge {
            asset_id: u32,
            voter: AccountIdOf<T>,
            vote: Vote,
        },
        /// The proposal has been executed.
        ProposalExecuted {
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
        },
        /// The agent got slashed.
        AgentSlashed {
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
        },
        /// The agent has been changed.
        AgentChanged { asset_id: u32 },
        /// A proposal got rejected.
        ProposalRejected { proposal_id: ProposalIndex },
        /// A challenge has been rejected/
        ChallengeRejected { asset_id: u32 },
        /// The threshold could not be reached for a proposal.
        ProposalThresHoldNotReached {
            proposal_id: ProposalIndex,
            required_threshold: Percent,
        },
        /// The threshold could not be reached for a challenge.
        ChallengeThresHoldNotReached {
            asset_id: u32,
            required_threshold: Percent,
        },
        /// New sale proposal has been created.
        PropertySaleProposed {
            asset_id: u32,
            proposer: AccountIdOf<T>,
        },
        /// A sale proposal got rejected.
        PropertySaleProposalRejected { asset_id: u32 },
        /// Lawyer for a sale has been set.
        SalesLawyerSet {
            asset_id: u32,
            lawyer: AccountIdOf<T>,
            legal_side: LegalSale,
        },
        /// The sale got approved by the lawyer.
        LawyerApprovesSale {
            asset_id: u32,
            lawyer: AccountIdOf<T>,
            legal_side: LegalSale,
        },
        /// The sale got rejected by the lawyer.
        LawyerRejectsSale {
            asset_id: u32,
            lawyer: AccountIdOf<T>,
            legal_side: LegalSale,
        },
        /// A sale has been finalized.
        SaleFinalized {
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
            payment_asset: u32,
        },
        /// A token owner claimed his sale funds.
        SaleFundsClaimed {
            claimer: AccountIdOf<T>,
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
            payment_asset: u32,
        },
        /// A bid has ben placed.
        BidSuccessfullyPlaced {
            asset_id: u32,
            bidder: AccountIdOf<T>,
            new_leading_bid: <T as pallet::Config>::Balance,
        },
        /// An auction has been won.
        AuctionWon {
            asset_id: u32,
            winner: AccountIdOf<T>,
            highest_bid: <T as pallet::Config>::Balance,
        },
        /// A sale has been approved.
        SaleApproved { asset_id: u32 },
        /// A sale has been rejected.
        SaleRejected { asset_id: u32 },
        /// Processing of a proposal failed.
        ProposalProcessingFailed {
            proposal_id: ProposalIndex,
            error: DispatchResult,
        },
        /// Processing of a sale proposal failed.
        SaleProposalProcessingFailed {
            asset_id: u32,
            error: DispatchResult,
        },
        /// Processing of a challenge failed.
        ChallengeProcessingFailed {
            asset_id: u32,
            error: DispatchResult,
        },
        /// Processing of an auction failed.
        AuctionProcessingFailed {
            asset_id: u32,
            error: DispatchResult,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// There are already too many proposals in the ending block.
        TooManyProposals,
        /// The proposal is not ongoing.
        NotOngoing,
        /// There is no letting agent for this property.
        NoLettingAgentFound,
        /// The pallet has not enough funds.
        NotEnoughFunds,
        /// The region is not registered.
        RegionUnknown,
        /// The caller is not authorized to call this extrinsic.
        NoPermission,
        /// Real estate asset does not exist.
        AssetNotFound,
        /// This Agent has no authorization in the region.
        NoPermissionInRegion,
        /// The property is not for sale.
        NotForSale,
        /// The sale has not been approved yet by a lawyer.
        SaleHasNotBeenApproved,
        /// The real estate object could not be found.
        NoObjectFound,
        /// Error by dividing a number.
        DivisionError,
        /// Error by multiplying a number.
        MultiplyError,
        /// This Asset is not supported for payment.
        PaymentAssetNotSupported,
        /// The property sale has already been finalized.
        AlreadyFinalized,
        /// Sale has not been finalized.
        SaleNotFinalized,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        /// The lawyer already confirmed the sale.
        SaleAlreadyConfirmed,
        /// There are no funds to claim for the caller.
        NoFundsToClaim,
        /// Costs for the lawyer are too high.
        CostsTooHigh,
        /// The lawyer job has already been taken.
        LawyerJobTaken,
        /// Price for a property sale has not been set yet.
        PriceNotSet,
        /// The Spv lawyer is not set.
        SpvLawyerNotSet,
        /// No price has been set.
        NoPriceSet,
        /// There is no auction to bid on.
        NoOngoingAuction,
        /// User did not pass the kyc.
        UserNotWhitelisted,
        /// The bid is lower than the current highest bid.
        BidTooLow,
        /// There is already a sale ongoing.
        SaleOngoing,
        /// There is already a sale proposal ongoing.
        PropertySaleProposalOngoing,
        /// No buyer has been set.
        BuyerNotSet,
        /// No reserve has been set for the sale.
        NoReserve,
        /// Token amount is zero.
        ZeroTokenAmount,
        /// The letting agent has already too many assigned properties.
        TooManyAssignedProperties,
        /// A challenge against a letting agent is already ongoing.
        ChallengeAlreadyOngoing,
        /// There are already too many voters for this voting.
        TooManyVoters,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: frame_system::pallet_prelude::BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads_writes(1, 1);

            let ended_votings = ProposalRoundsExpiring::<T>::take(n);
            // checks if there is a voting for a proposal ending in this block.
            ended_votings.iter().for_each(|item| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(4, 3));
                UserProposalVote::<T>::remove(item);
                if let Err(e) = Self::finish_proposal(*item) {
                    Self::deposit_event(Event::ProposalProcessingFailed {
                        proposal_id: *item,
                        error: Err(e),
                    });
                };
            });

            let ended_votings = SaleProposalRoundsExpiring::<T>::take(n);
            // Checks if there is a voting for a sale porposal in this block;
            ended_votings.iter().for_each(|item| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(6, 4));
                UserSaleProposalVote::<T>::remove(item);
                if let Err(e) = Self::finish_sale_proposal(*item) {
                    Self::deposit_event(Event::SaleProposalProcessingFailed {
                        asset_id: *item,
                        error: Err(e),
                    });
                };
            });

            let ended_votings = AuctionRoundsExpiring::<T>::take(n);
            // Checks if there is a voting for a sale porposal in this block;
            ended_votings.iter().for_each(|item| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));
                UserSaleProposalVote::<T>::remove(item);
                if let Err(e) = Self::finish_auction(*item) {
                    Self::deposit_event(Event::AuctionProcessingFailed {
                        asset_id: *item,
                        error: Err(e),
                    });
                };
            });

            let ended_challenge_votings = ChallengeRoundsExpiring::<T>::take(n);
            // checks if there is a voting for an challenge ending in this block.
            ended_challenge_votings.iter().for_each(|item| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(7, 9));
                UserChallengeVote::<T>::remove(item);
                if let Err(e) = Self::finish_challenge(*item) {
                    Self::deposit_event(Event::ChallengeProcessingFailed {
                        asset_id: *item,
                        error: Err(e),
                    });
                };
            });
            weight
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a proposal for a real estate object.
        /// Only the letting agent can propose.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `amount`: The amount the letting agent is asking for.
        /// - `data`: The data regarding this proposal.
        ///
        /// Emits `Proposed` event when succesfful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose())]
        pub fn propose(
            origin: OriginFor<T>,
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
            data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                pallet_property_management::LettingStorage::<T>::get(asset_id)
                    .ok_or(Error::<T>::NoLettingAgentFound)?
                    == signer,
                Error::<T>::NoPermission
            );
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let proposal = Proposal {
                proposer: signer.clone(),
                asset_id,
                amount,
                created_at: current_block_number,
                metadata: data,
            };

            // Check if the amount is less than LowProposal
            if amount <= <T as Config>::LowProposal::get() {
                // Execute the proposal immediately
                Self::execute_proposal(proposal)?;
                return Ok(());
            }

            let proposal_id = ProposalCount::<T>::get();
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::VotingTime::get());
            ProposalRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
                keys.try_push(proposal_id)
                    .map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;
            let vote_stats = VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0,
            };
            let next_proposal_id = proposal_id.saturating_add(1);

            Proposals::<T>::insert(proposal_id, proposal);
            OngoingProposalVotes::<T>::insert(proposal_id, vote_stats);
            ProposalCount::<T>::put(next_proposal_id);
            Self::deposit_event(Event::Proposed {
                proposal_id,
                asset_id,
                proposer: signer,
            });
            Ok(())
        }

        /// Creates an challenge against the letting agent of the real estate object.
        /// Only one of the owner of the property can propose.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `Challenge` event when succesfful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::challenge_against_letting_agent())]
        pub fn challenge_against_letting_agent(
            origin: OriginFor<T>,
            asset_id: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
            ensure!(
                pallet_property_management::LettingStorage::<T>::get(asset_id).is_some(),
                Error::<T>::NoLettingAgentFound
            );
            ensure!(
                Challenges::<T>::get(asset_id).is_none(),
                Error::<T>::ChallengeAlreadyOngoing
            );

            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::VotingTime::get());
            let challenge = Challenge {
                proposer: signer.clone(),
                created_at: current_block_number,
            };
            ChallengeRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
                keys.try_push(asset_id)
                    .map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;
            let vote_stats = VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0,
            };
            OngoingChallengeVotes::<T>::insert(asset_id, vote_stats);
            Challenges::<T>::insert(asset_id, challenge);

            Self::deposit_event(Event::Challenge {
                asset_id,
                proposer: signer,
            });
            Ok(())
        }

        /// Lets owner of the real estate object vote on a proposal.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `proposal_id`: The index of the proposal.
        /// - `vote`: Must be either a Yes vote or a No vote.
        ///
        /// Emits `VotedOnProposal` event when succesfful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_proposal())]
        pub fn vote_on_proposal(
            origin: OriginFor<T>,
            proposal_id: ProposalIndex,
            vote: Vote,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let owner_list =
                <T as pallet::Config>::PropertyToken::get_property_owner(proposal.asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
            let voting_power =
                <T as pallet::Config>::PropertyToken::get_token_balance(proposal.asset_id, &signer);
            OngoingProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserProposalVote::<T>::try_mutate(proposal_id, |maybe_map| {
                    let map = maybe_map.get_or_insert_with(BoundedBTreeMap::new);
                    if let Some(previous_vote) = map.get(&signer) {
                        match previous_vote {
                            Vote::Yes => {
                                current_vote.yes_voting_power =
                                    current_vote.yes_voting_power.saturating_sub(voting_power)
                            }
                            Vote::No => {
                                current_vote.no_voting_power =
                                    current_vote.no_voting_power.saturating_sub(voting_power)
                            }
                        }
                    }

                    match vote {
                        Vote::Yes => {
                            current_vote.yes_voting_power =
                                current_vote.yes_voting_power.saturating_add(voting_power)
                        }
                        Vote::No => {
                            current_vote.no_voting_power =
                                current_vote.no_voting_power.saturating_add(voting_power)
                        }
                    }

                    map.try_insert(signer.clone(), vote.clone())
                        .map_err(|_| Error::<T>::TooManyVoters)?;
                    Ok::<(), DispatchError>(())
                })?; 
                Ok::<(), DispatchError>(())  
            })?;
            Self::deposit_event(Event::VotedOnProposal {
                proposal_id,
                voter: signer,
                vote,
            });
            Ok(())
        }

        /// Lets owner of the real estate object vote on an challenge.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id: u32`: The index of the challenge.
        /// - `vote`: Must be either a Yes vote or a No vote.
        ///
        /// Emits `VotedOnChallenge` event when succesfful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_letting_agent_challenge())]
        pub fn vote_on_letting_agent_challenge(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                Challenges::<T>::get(asset_id).is_some(),
                Error::<T>::NotOngoing
            );
            let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
            let voting_power =
                <T as pallet::Config>::PropertyToken::get_token_balance(asset_id, &signer);
            OngoingChallengeVotes::<T>::try_mutate(asset_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserChallengeVote::<T>::try_mutate(asset_id, |maybe_map| {
                    let map = maybe_map.get_or_insert_with(BoundedBTreeMap::new);
                    if let Some(previous_vote) = map.get(&signer) {
                        match previous_vote {
                            Vote::Yes => {
                                current_vote.yes_voting_power =
                                    current_vote.yes_voting_power.saturating_sub(voting_power)
                            }
                            Vote::No => {
                                current_vote.no_voting_power =
                                    current_vote.no_voting_power.saturating_sub(voting_power)
                            }
                        }
                    }

                    match vote {
                        Vote::Yes => {
                            current_vote.yes_voting_power =
                                current_vote.yes_voting_power.saturating_add(voting_power)
                        }
                        Vote::No => {
                            current_vote.no_voting_power =
                                current_vote.no_voting_power.saturating_add(voting_power)
                        }
                    }

                    map.try_insert(signer.clone(), vote.clone())
                        .map_err(|_| Error::<T>::TooManyVoters)?;
                    Ok::<(), DispatchError>(())
                })?; 
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnChallenge {
                asset_id,
                voter: signer,
                vote,
            });
            Ok(())
        }

        /// Creates a proposal to sell a real estate object as a whole.
        /// Only a token holder can propose.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `PropertySaleProposed` event when succesfful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose_property_sale())]
        pub fn propose_property_sale(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            <T as pallet::Config>::PropertyToken::ensure_spv_created(asset_id)?;

            ensure!(
                PropertySale::<T>::get(asset_id).is_none(),
                Error::<T>::SaleOngoing
            );
            ensure!(
                SaleProposals::<T>::get(asset_id).is_none(),
                Error::<T>::PropertySaleProposalOngoing
            );
            let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let sale_proposal = PropertySaleProposal {
                proposer: signer.clone(),
                created_at: current_block_number,
            };
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::SaleVotingTime::get());
            SaleProposalRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
                keys.try_push(asset_id)
                    .map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;
            let vote_stats = VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0,
            };

            SaleProposals::<T>::insert(asset_id, sale_proposal);
            OngoingSaleProposalVotes::<T>::insert(asset_id, vote_stats);
            Self::deposit_event(Event::PropertySaleProposed {
                asset_id,
                proposer: signer,
            });
            Ok(())
        }

        /// Lets owner of the real estate object vote on a sale proposal.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `vote`: Must be either a Yes vote or a No vote.
        ///
        /// Emits `VotedOnPropertySaleProposal` event when succesfful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_property_sale())]
        pub fn vote_on_property_sale(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                SaleProposals::<T>::get(asset_id).is_some(),
                Error::<T>::NotOngoing
            );
            let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
            let voting_power =
                <T as pallet::Config>::PropertyToken::get_token_balance(asset_id, &signer);
            OngoingSaleProposalVotes::<T>::try_mutate(asset_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserSaleProposalVote::<T>::try_mutate(asset_id, |maybe_map| {
                    let map = maybe_map.get_or_insert_with(BoundedBTreeMap::new);
                    if let Some(previous_vote) = map.get(&signer) {
                        match previous_vote {
                            Vote::Yes => {
                                current_vote.yes_voting_power =
                                    current_vote.yes_voting_power.saturating_sub(voting_power)
                            }
                            Vote::No => {
                                current_vote.no_voting_power =
                                    current_vote.no_voting_power.saturating_sub(voting_power)
                            }
                        }
                    }

                    match vote {
                        Vote::Yes => {
                            current_vote.yes_voting_power =
                                current_vote.yes_voting_power.saturating_add(voting_power)
                        }
                        Vote::No => {
                            current_vote.no_voting_power =
                                current_vote.no_voting_power.saturating_add(voting_power)
                        }
                    }

                    map.try_insert(signer.clone(), vote.clone())
                        .map_err(|_| Error::<T>::TooManyVoters)?;
                    Ok::<(), DispatchError>(())
                })?; 
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnPropertySaleProposal {
                asset_id,
                voter: signer,
                vote,
            });
            Ok(())
        }

        /// Lets someone bid to buy the property that is on sale.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `price`: Price that the buyer wants to pay.
        /// - `payment_asset`: Asset in which the caller wants to pay.
        ///
        /// Emits `BidSuccessfullyPlaced` event when succesfful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::bid_on_sale())]
        pub fn bid_on_sale(
            origin: OriginFor<T>,
            asset_id: u32,
            price: <T as pallet::Config>::Balance,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                pallet_xcavate_whitelist::WhitelistedAccounts::<T>::get(&signer),
                Error::<T>::UserNotWhitelisted
            );
            ensure!(
                <T as pallet::Config>::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );
            let reserve_amount = price
                .checked_div(&10u128.into())
                .ok_or(Error::<T>::DivisionError)?;
            <T as pallet::Config>::ForeignAssetsHolder::hold(
                payment_asset,
                &MarketplaceHoldReason::Auction,
                &signer,
                reserve_amount,
            )?;

            SaleAuctions::<T>::try_mutate(asset_id, |maybe_auction| -> DispatchResult {
                let auction = maybe_auction.as_mut().ok_or(Error::<T>::NoOngoingAuction)?;
                if let Some(old_reserve) = &auction.reserve {
                    if let Some(ref old_bidder) = auction.highest_bidder {
                        <T as pallet::Config>::ForeignAssetsHolder::release(
                            old_reserve.payment_asset,
                            &MarketplaceHoldReason::Auction,
                            old_bidder,
                            old_reserve.amount,
                            Precision::Exact,
                        )?;
                    }
                };
                ensure!(price > auction.price, Error::<T>::BidTooLow);
                auction.highest_bidder = Some(signer.clone());
                auction.price = price;
                auction.reserve = Some(Reserve {
                    payment_asset,
                    amount: reserve_amount,
                });
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::BidSuccessfullyPlaced {
                asset_id,
                bidder: signer,
                new_leading_bid: price,
            });
            Ok(())
        }

        /// Lets a lawyer claim a sale to handle the legal work.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `legal_side`: The side that the lawyer wants to represent.
        /// - `costs`: The costs thats the lawyer demands for his work.
        ///
        /// Emits `SalesLawyerSet` event when succesfful.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::lawyer_claim_sale())]
        pub fn lawyer_claim_sale(
            origin: OriginFor<T>,
            asset_id: u32,
            legal_side: LegalSale,
            costs: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let lawyer_region = pallet_regions::RealEstateLawyer::<T>::get(&signer)
                .ok_or(Error::<T>::NoPermission)?;
            let asset_info =
                <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::AssetNotFound)?;
            ensure!(
                lawyer_region == asset_info.region,
                Error::<T>::NoPermissionInRegion
            );
            let mut property_sale_info =
                PropertySale::<T>::get(asset_id).ok_or(Error::<T>::NotForSale)?;
            let price = property_sale_info.price.ok_or(Error::<T>::PriceNotSet)?;
            let max_costs = price
                .checked_div(&100u128.into())
                .ok_or(Error::<T>::DivisionError)?;
            ensure!(max_costs >= costs, Error::<T>::CostsTooHigh);
            match legal_side {
                LegalSale::SpvSide => {
                    ensure!(
                        property_sale_info.spv_lawyer.is_none(),
                        Error::<T>::LawyerJobTaken
                    );
                    ensure!(
                        property_sale_info.buyer_lawyer.as_ref() != Some(&signer),
                        Error::<T>::NoPermission
                    );
                    property_sale_info.spv_lawyer = Some(signer.clone());
                    property_sale_info.spv_lawyer_costs = costs;
                    PropertySale::<T>::insert(asset_id, property_sale_info);
                }
                LegalSale::BuyerSide => {
                    ensure!(
                        property_sale_info.buyer_lawyer.is_none(),
                        Error::<T>::LawyerJobTaken
                    );
                    ensure!(
                        property_sale_info.spv_lawyer.as_ref() != Some(&signer),
                        Error::<T>::NoPermission
                    );
                    property_sale_info.buyer_lawyer = Some(signer.clone());
                    property_sale_info.buyer_lawyer_costs = costs;
                    PropertySale::<T>::insert(asset_id, property_sale_info);
                }
            }
            Self::deposit_event(Event::SalesLawyerSet {
                asset_id,
                lawyer: signer,
                legal_side,
            });
            Ok(())
        }

        /// Lets a lawyer confirm a legal case.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `approve`: Approves or Rejects the case.
        ///
        /// Emits `LawyerApprovesSale` event when approved successfully.
        /// Emits `LawyerRejectsSale` event when rejected successfully.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::lawyer_confirm_sale())]
        pub fn lawyer_confirm_sale(
            origin: OriginFor<T>,
            asset_id: u32,
            approve: bool,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let mut property_sale_info =
                PropertySale::<T>::take(asset_id).ok_or(Error::<T>::NotForSale)?;
            if property_sale_info.spv_lawyer.as_ref() == Some(&signer) {
                ensure!(
                    property_sale_info.spv_status == DocumentStatus::Pending,
                    Error::<T>::SaleAlreadyConfirmed
                );
                property_sale_info.spv_status = if approve {
                    Self::deposit_event(Event::LawyerApprovesSale {
                        asset_id,
                        lawyer: signer,
                        legal_side: LegalSale::SpvSide,
                    });
                    DocumentStatus::Approved
                } else {
                    Self::deposit_event(Event::LawyerRejectsSale {
                        asset_id,
                        lawyer: signer,
                        legal_side: LegalSale::SpvSide,
                    });
                    DocumentStatus::Rejected
                };
            } else if property_sale_info.buyer_lawyer.as_ref() == Some(&signer) {
                ensure!(
                    property_sale_info.buyer_status == DocumentStatus::Pending,
                    Error::<T>::SaleAlreadyConfirmed
                );
                property_sale_info.buyer_status = if approve {
                    Self::deposit_event(Event::LawyerApprovesSale {
                        asset_id,
                        lawyer: signer,
                        legal_side: LegalSale::BuyerSide,
                    });
                    DocumentStatus::Approved
                } else {
                    Self::deposit_event(Event::LawyerRejectsSale {
                        asset_id,
                        lawyer: signer,
                        legal_side: LegalSale::BuyerSide,
                    });
                    DocumentStatus::Rejected
                };
            } else {
                return Err(Error::<T>::NoPermission.into());
            }

            let spv_status = property_sale_info.spv_status.clone();
            let buyer_status = property_sale_info.buyer_status.clone();

            match (spv_status, buyer_status) {
                (DocumentStatus::Approved, DocumentStatus::Approved) => {
                    property_sale_info.lawyer_approved = true;
                    PropertySale::<T>::insert(asset_id, property_sale_info);
                    Self::deposit_event(Event::SaleApproved { asset_id });
                }
                (DocumentStatus::Rejected, DocumentStatus::Rejected) => {
                    Self::release_token(property_sale_info)?;
                    Self::deposit_event(Event::SaleRejected { asset_id });
                }
                (DocumentStatus::Approved, DocumentStatus::Rejected) => {
                    if !property_sale_info.second_attempt {
                        property_sale_info.spv_status = DocumentStatus::Pending;
                        property_sale_info.buyer_status = DocumentStatus::Pending;
                        property_sale_info.second_attempt = true;
                        PropertySale::<T>::insert(asset_id, property_sale_info);
                    } else {
                        Self::release_token(property_sale_info)?;
                        Self::deposit_event(Event::SaleRejected { asset_id });
                    }
                }
                (DocumentStatus::Rejected, DocumentStatus::Approved) => {
                    if !property_sale_info.second_attempt {
                        property_sale_info.spv_status = DocumentStatus::Pending;
                        property_sale_info.buyer_status = DocumentStatus::Pending;
                        property_sale_info.second_attempt = true;
                        PropertySale::<T>::insert(asset_id, property_sale_info);
                    } else {
                        Self::release_token(property_sale_info)?;
                        Self::deposit_event(Event::SaleRejected { asset_id });
                    }
                }
                _ => {
                    PropertySale::<T>::insert(asset_id, property_sale_info);
                }
            }
            Ok(())
        }

        /// Lets a the lawyer that represents the buyer finalize the sale and sending the funds.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `payment_asset`: Asset in which the lawyer wants to pay.
        ///
        /// Emits `SaleFinalized` event when succesfful.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_sale())]
        pub fn finalize_sale(
            origin: OriginFor<T>,
            asset_id: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            PropertySale::<T>::try_mutate_exists(asset_id, |maybe_sale| -> DispatchResult {
                let sale_info = maybe_sale.as_mut().ok_or(Error::<T>::NotForSale)?;
                ensure!(
                    sale_info.buyer_lawyer.as_ref() == Some(&signer),
                    Error::<T>::NoPermission
                );
                ensure!(
                    sale_info.lawyer_approved,
                    Error::<T>::SaleHasNotBeenApproved
                );
                ensure!(!sale_info.finalized, Error::<T>::AlreadyFinalized);
                ensure!(
                    <T as pallet::Config>::AcceptedAssets::get().contains(&payment_asset),
                    Error::<T>::PaymentAssetNotSupported
                );

                let sales_price = sale_info.price.ok_or(Error::<T>::NoPriceSet)?;
                let spv_lawyer_fees = sale_info.spv_lawyer_costs;
                let buyer_lawyer_fees = sale_info.buyer_lawyer_costs;
                let spv_lawyer_account = sale_info
                    .spv_lawyer
                    .clone()
                    .ok_or(Error::<T>::SpvLawyerNotSet)?;
                let property_account = Self::property_account_id(asset_id);
                let treasury_account = Self::treasury_account_id();

                let property_info =
                    <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id)
                        .ok_or(Error::<T>::NoObjectFound)?;
                let region_info = pallet_regions::RegionDetails::<T>::get(property_info.region)
                    .ok_or(Error::<T>::RegionUnknown)?;

                let total_fees = sales_price
                    .checked_mul(&2u128.into())
                    .ok_or(Error::<T>::MultiplyError)?
                    .checked_div(&100u128.into())
                    .ok_or(Error::<T>::DivisionError)?;
                let protocol_fees = total_fees
                    .checked_sub(&spv_lawyer_fees)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?
                    .checked_sub(&buyer_lawyer_fees)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                let region_owner_share = protocol_fees
                    .checked_div(&2u128.into())
                    .ok_or(Error::<T>::DivisionError)?;
                let treasury_share = protocol_fees.saturating_sub(region_owner_share);
                let net_amount = sales_price
                    .checked_sub(&total_fees)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;

                let reserve = sale_info.reserve.clone().ok_or(Error::<T>::NoReserve)?;
                let buyer = sale_info.buyer.clone().ok_or(Error::<T>::BuyerNotSet)?;
                <T as pallet::Config>::ForeignAssetsHolder::release(
                    reserve.payment_asset,
                    &MarketplaceHoldReason::Auction,
                    &buyer,
                    reserve.amount,
                    Precision::Exact,
                )?;
                let reserve_released = reserve.amount;
                let reserve_asset = reserve.payment_asset;
                Self::transfer_funds(&buyer, &property_account, reserve.amount, reserve_asset)?;

                let expected_buyer_amount = net_amount
                    .checked_sub(&reserve_released)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;

                Self::transfer_funds(
                    &signer,
                    &property_account,
                    expected_buyer_amount,
                    payment_asset,
                )?;
                Self::transfer_funds(&signer, &spv_lawyer_account, spv_lawyer_fees, payment_asset)?;
                Self::transfer_funds(&signer, &treasury_account, treasury_share, payment_asset)?;
                Self::transfer_funds(
                    &signer,
                    &region_info.owner,
                    region_owner_share,
                    payment_asset,
                )?;

                PropertySaleFunds::<T>::insert(asset_id, payment_asset, expected_buyer_amount);
                PropertySaleFunds::<T>::insert(asset_id, reserve.payment_asset, reserve_released);

                sale_info.finalized = true;
                Self::deposit_event(Event::SaleFinalized {
                    asset_id,
                    amount: sales_price,
                    payment_asset,
                });
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }

        /// Lets a token holder withdraw his stored funds from a sale.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `payment_asset`: Asset id the caller wants to withdraw funds in.
        ///
        /// Emits `SaleFundsClaimed` event when succesfful.
        #[pallet::call_index(10)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::claim_sale_funds())]
        pub fn claim_sale_funds(
            origin: OriginFor<T>,
            asset_id: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let mut property_sale_info =
                PropertySale::<T>::take(asset_id).ok_or(Error::<T>::NotForSale)?;
            ensure!(property_sale_info.finalized, Error::<T>::SaleNotFinalized);
            let mut property_sale_amount = PropertySaleFunds::<T>::get(asset_id, payment_asset);
            let property_info =
                <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;

            let sales_price = property_sale_info.price.ok_or(Error::<T>::NoPriceSet)?;
            let total_fees = sales_price
                .checked_mul(&2u128.into())
                .ok_or(Error::<T>::MultiplyError)?
                .checked_div(&100u128.into())
                .ok_or(Error::<T>::DivisionError)?;

            let net_amount = sales_price
                .checked_sub(&total_fees)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            let total_token = property_info.token_amount;
            let property_token_amount =
                <T as pallet::Config>::PropertyToken::take_property_token(asset_id, &signer);
            ensure!(!property_token_amount.is_zero(), Error::<T>::NoFundsToClaim);

            let mut owner_share = net_amount
                .checked_mul(&(property_token_amount as u128).into())
                .ok_or(Error::<T>::MultiplyError)?
                .checked_div(&(total_token as u128).into())
                .ok_or(Error::<T>::DivisionError)?;
            let property_account = Self::property_account_id(asset_id);
            if property_sale_amount >= owner_share {
                Self::transfer_funds(&property_account, &signer, owner_share, payment_asset)?;
                property_sale_amount = property_sale_amount
                    .checked_sub(&owner_share)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                if property_sale_amount.is_zero() {
                    PropertySaleFunds::<T>::remove(asset_id, payment_asset);
                } else {
                    PropertySaleFunds::<T>::insert(asset_id, payment_asset, property_sale_amount);
                }
            } else {
                for (payment_asset, mut funds_amount) in
                    PropertySaleFunds::<T>::iter_prefix(asset_id)
                {
                    if funds_amount >= owner_share {
                        Self::transfer_funds(
                            &property_account,
                            &signer,
                            owner_share,
                            payment_asset,
                        )?;
                        funds_amount = funds_amount
                            .checked_sub(&owner_share)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                        if funds_amount.is_zero() {
                            PropertySaleFunds::<T>::remove(asset_id, payment_asset);
                        } else {
                            PropertySaleFunds::<T>::insert(asset_id, payment_asset, funds_amount);
                        }
                        break;
                    } else {
                        Self::transfer_funds(
                            &property_account,
                            &signer,
                            funds_amount,
                            payment_asset,
                        )?;
                        owner_share = owner_share
                            .checked_sub(&funds_amount)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                        PropertySaleFunds::<T>::remove(asset_id, payment_asset);
                    }
                }
            }
            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                &signer,
                &property_account,
                property_token_amount.into(),
                Preservation::Expendable,
            )?;
            property_sale_info.property_token_amount = property_sale_info
                .property_token_amount
                .checked_sub(property_token_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            if property_sale_info.property_token_amount == 0 {
                <T as pallet::Config>::PropertyToken::burn_property_token(asset_id)?;
                <T as pallet::Config>::PropertyToken::clear_token_owners(asset_id)?;
            } else {
                PropertySale::<T>::insert(asset_id, property_sale_info);
            }
            Self::deposit_event(Event::SaleFundsClaimed {
                claimer: signer,
                asset_id,
                amount: owner_share,
                payment_asset,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
            <T as pallet::Config>::MarketplacePalletId::get()
                .into_sub_account_truncating(("pr", asset_id))
        }

        /// Get the account id of the treasury pallet
        pub fn treasury_account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::TreasuryId::get().into_account_truncating()
        }

        // Slashes the letting agent.
        fn slash_letting_agent(asset_id: u32, letting_agent: AccountIdOf<T>) -> DispatchResult {
            let amount = <T as Config>::MinSlashingAmount::get();

            let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                &<T as pallet_property_management::Config>::RuntimeHoldReason::from(
                    pallet_property_management::HoldReason::LettingAgent,
                ),
                &letting_agent,
                amount,
            );

            <T as pallet::Config>::Slash::on_unbalanced(imbalance);

            Self::deposit_event(Event::AgentSlashed { asset_id, amount });
            Ok(())
        }

        /// Changes the letting agent of a given real estate object.
        fn change_letting_agent(asset_id: u32) -> DispatchResult {
            let _ = pallet_property_management::Pallet::<T>::remove_bad_letting_agent(asset_id);
            Self::deposit_event(Event::AgentChanged { asset_id });
            Ok(())
        }

        fn finish_proposal(proposal_id: ProposalIndex) -> DispatchResult {
            let voting_results = OngoingProposalVotes::<T>::take(proposal_id);
            let proposals = Proposals::<T>::take(proposal_id);
            if let Some(proposal) = proposals {
                if let Some(voting_result) = voting_results {
                    let required_threshold =
                        if proposal.amount >= <T as Config>::HighProposal::get() {
                            <T as Config>::HighThreshold::get()
                        } else {
                            <T as Config>::Threshold::get()
                        };
                    let asset_details =
                        <T as pallet::Config>::PropertyToken::get_property_asset_info(
                            proposal.asset_id,
                        );
                    if let Some(asset_details) = asset_details {
                        ensure!(asset_details.token_amount > 0, Error::<T>::ZeroTokenAmount);
                        let yes_votes_percentage = Percent::from_rational(
                            voting_result.yes_voting_power,
                            asset_details.token_amount,
                        );
                        let no_votes_percentage = Percent::from_rational(
                            voting_result.no_voting_power,
                            asset_details.token_amount,
                        );

                        if yes_votes_percentage > no_votes_percentage
                            && required_threshold
                                < yes_votes_percentage.saturating_add(no_votes_percentage)
                        {
                            let _ = Self::execute_proposal(proposal);
                        } else if yes_votes_percentage <= no_votes_percentage {
                            Self::deposit_event(Event::ProposalRejected { proposal_id });
                        } else {
                            Self::deposit_event(Event::ProposalThresHoldNotReached {
                                proposal_id,
                                required_threshold,
                            });
                        }
                    }
                }
            }
            Ok(())
        }

        fn finish_sale_proposal(asset_id: u32) -> DispatchResult {
            let voting_results = OngoingSaleProposalVotes::<T>::take(asset_id);
            SaleProposals::<T>::remove(asset_id);
            if let Some(voting_result) = voting_results {
                let asset_details =
                    <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id);
                if let Some(asset_details) = asset_details {
                    ensure!(asset_details.token_amount > 0, Error::<T>::ZeroTokenAmount);
                    let yes_votes_percentage = Percent::from_rational(
                        voting_result.yes_voting_power,
                        asset_details.token_amount,
                    );
                    let required_threshold = T::SaleApprovalYesThreshold::get();
                    if yes_votes_percentage >= required_threshold {
                        let _ = Self::execute_sale_proposal(asset_id, asset_details.token_amount);
                    } else {
                        Self::deposit_event(Event::PropertySaleProposalRejected { asset_id });
                    }
                }
            }
            Ok(())
        }

        fn finish_auction(asset_id: u32) -> DispatchResult {
            if let Some(auction) = SaleAuctions::<T>::take(asset_id) {
                if auction.price > 0u128.into() {
                    if let Some(buyer) = auction.highest_bidder {
                        if let Some(mut sale) = PropertySale::<T>::get(asset_id) {
                            sale.price = Some(auction.price);
                            sale.buyer = Some(buyer.clone());
                            sale.reserve = auction.reserve;
                            PropertySale::<T>::insert(asset_id, sale);

                            Self::deposit_event(Event::AuctionWon {
                                asset_id,
                                winner: buyer,
                                highest_bid: auction.price,
                            });
                        }
                    }
                }
            }
            Ok(())
        }

        fn finish_challenge(asset_id: u32) -> DispatchResult {
            let _ = Challenges::<T>::take(asset_id).ok_or(Error::<T>::NotOngoing)?;
            let voting_result =
                OngoingChallengeVotes::<T>::take(asset_id).ok_or(Error::<T>::NotOngoing)?;
            let asset_details =
                <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::AssetNotFound)?;
            ensure!(asset_details.token_amount > 0, Error::<T>::ZeroTokenAmount);
            let yes_votes_percentage =
                Percent::from_rational(voting_result.yes_voting_power, asset_details.token_amount);
            let no_votes_percentage =
                Percent::from_rational(voting_result.no_voting_power, asset_details.token_amount);
            let required_threshold = <T as Config>::Threshold::get();
            if yes_votes_percentage > no_votes_percentage
                && required_threshold < yes_votes_percentage.saturating_add(no_votes_percentage)
            {
                let letting_agent = pallet_property_management::LettingStorage::<T>::get(asset_id)
                    .ok_or(Error::<T>::NoLettingAgentFound)?;
                let mut letting_info =
                    pallet_property_management::LettingInfo::<T>::get(letting_agent.clone())
                        .ok_or(Error::<T>::NoLettingAgentFound)?;
                Self::slash_letting_agent(asset_id, letting_agent.clone())?;
                let active_strikes = letting_info
                    .active_strikes
                    .get(&asset_id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1);

                if let Some(entry) = letting_info.active_strikes.get_mut(&asset_id) {
                    *entry = active_strikes;
                } else {
                    letting_info
                        .active_strikes
                        .try_insert(asset_id, active_strikes)
                        .map_err(|_| Error::<T>::TooManyAssignedProperties)?;
                }

                if active_strikes >= 3 {
                    let _ = Self::change_letting_agent(asset_id);
                    letting_info.active_strikes.remove(&asset_id);
                }
                pallet_property_management::LettingInfo::<T>::insert(letting_agent, letting_info);
            } else if yes_votes_percentage <= no_votes_percentage {
                Self::deposit_event(Event::ChallengeRejected { asset_id });
            } else {
                Self::deposit_event(Event::ChallengeThresHoldNotReached {
                    asset_id,
                    required_threshold,
                });
            }

            Ok(())
        }

        /// Executes a proposal once it passes.
        fn execute_proposal(proposal: Proposal<T>) -> DispatchResult {
            let asset_id = proposal.asset_id;
            let proposal_amount = proposal.amount;

            Self::deposit_event(Event::ProposalExecuted {
                asset_id,
                amount: proposal_amount,
            });

            Ok(())
        }

        /// Executes a sale proposal once it passes.
        fn execute_sale_proposal(asset_id: u32, property_token_amount: u32) -> DispatchResult {
            let property_sale_info = PropertySaleInfo {
                spv_lawyer: None,
                buyer_lawyer: None,
                buyer: None,
                spv_status: DocumentStatus::Pending,
                buyer_status: DocumentStatus::Pending,
                spv_lawyer_costs: Default::default(),
                buyer_lawyer_costs: Default::default(),
                price: None,
                second_attempt: false,
                lawyer_approved: false,
                finalized: false,
                property_token_amount,
                reserve: None,
            };

            let auction = SaleAuction {
                highest_bidder: None,
                price: Default::default(),
                reserve: None,
            };
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::AuctionTime::get());
            AuctionRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
                keys.try_push(asset_id)
                    .map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;

            PropertySale::<T>::insert(asset_id, property_sale_info);
            SaleAuctions::<T>::insert(asset_id, auction);
            Ok(())
        }

        fn release_token(property_sale_info: PropertySaleInfo<T>) -> DispatchResult {
            let reserve = property_sale_info.reserve.ok_or(Error::<T>::NoReserve)?;
            let buyer = property_sale_info
                .buyer
                .clone()
                .ok_or(Error::<T>::BuyerNotSet)?;
            <T as pallet::Config>::ForeignAssetsHolder::release(
                reserve.payment_asset,
                &MarketplaceHoldReason::Auction,
                &buyer,
                reserve.amount,
                Precision::Exact,
            )?;
            Ok(())
        }

        fn transfer_funds(
            from: &AccountIdOf<T>,
            to: &AccountIdOf<T>,
            amount: <T as pallet::Config>::Balance,
            asset: u32,
        ) -> DispatchResult {
            if !amount.is_zero() {
                <T as pallet::Config>::ForeignCurrency::transfer(
                    asset,
                    from,
                    to,
                    amount,
                    Preservation::Expendable,
                )
                .map_err(|_| Error::<T>::NotEnoughFunds)?;
            }
            Ok(())
        }
    }
}

sp_api::decl_runtime_apis! {
    pub trait PropertyGovernanceApi<AccountId>
    where
        AccountId: Codec
    {
        fn get_governance_account_id() -> AccountId;
    }
}
