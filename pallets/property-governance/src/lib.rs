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
	sp_runtime::{traits::AccountIdConversion, Saturating, Percent},
	traits::{
		tokens::{fungible, fungibles, nonfungibles_v2, WithdrawConsequence},
		fungible::MutateHold,
		fungibles::Mutate as FungiblesMutate,
		fungibles::Inspect as FungiblesInspect,
		fungibles::{InspectFreeze, MutateFreeze},
		tokens::{Fortitude, Precision, Restriction, Preservation},
	},
	PalletId,
};

use codec::Codec;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type RuntimeHoldReasonOf<T> = <T as pallet_property_management::Config>::RuntimeHoldReason;

pub type Balance = u128;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[cfg(feature = "runtime-benchmarks")]
	pub struct AssetHelper;

	#[cfg(feature = "runtime-benchmarks")]
	pub trait BenchmarkHelper<AssetId, T> {
		fn to_asset(i: u32) -> AssetId;
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> BenchmarkHelper<AssetId<T>, T> for AssetHelper {
		fn to_asset(i: u32) -> AssetId<T> {
			i.into()
		}
	}

	pub type ProposalIndex = u32;
	pub type ChallengeIndex = u32;

	/// Proposal with the proposal Details.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Proposal<T: Config> {
		pub proposer: AccountIdOf<T>,
		pub asset_id: u32,
		pub amount: Balance,
		pub created_at: BlockNumberFor<T>,
		pub proposal_info: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
	}

	/// Sell proposal with the proposal Details.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct SaleProposal<T: Config> {
		pub proposer: AccountIdOf<T>,
		pub asset_id: u32,
		pub amount: Balance,
		pub created_at: BlockNumberFor<T>,
	}

	/// Challenge with the challenge Details.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Challenge<BlockNumber, T: Config> {
		pub proposer: AccountIdOf<T>,
		pub asset_id: u32,
		pub created_at: BlockNumber,
		pub state: ChallengeState,
	}

	/// Vote enum.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	pub enum Vote {
		Yes,
		No,
	}

	/// Challenge state of the challenge voting.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	pub enum ChallengeState {
		First,
		Second,
		Third,
		Fourth,
	}

	/// Voting stats.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	pub struct VoteStats {
		pub yes_voting_power: u32,
		pub no_voting_power: u32,
	}

	/// Info for the sales agent.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct SalesAgentInfo<T: Config> {
		pub account: AccountIdOf<T>,
		pub region: u32,
		pub locations: BoundedVec<LocationId<T>, <T as pallet_property_management::Config>::MaxLocations>,
		pub assigned_properties: BoundedVec<u32, <T as pallet_property_management::Config>::MaxProperties>,
		pub deposited: bool,
	}

	/// Voting stats.
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct PropertySaleInfo<T: Config> {
		pub sales_agent: Option<AccountIdOf<T>>,
		pub lawyer: Option<AccountIdOf<T>>,
		pub lawyer_approval: bool,
	}

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_nft_marketplace::Config
		+ pallet_property_management::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type representing the weight of this pallet.
		type WeightInfo: WeightInfo;

		/// The reservable currency type.
		type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
			+ fungible::Mutate<AccountIdOf<Self>>
			+ fungible::InspectHold<AccountIdOf<Self>, Balance = Balance>
			+ fungible::MutateHold<AccountIdOf<Self>, Balance = Balance, Reason = RuntimeHoldReasonOf<Self>>
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

		/// The amount of time given to vote for a proposal.
		type VotingTime: Get<BlockNumberFor<Self>>;

		/// The amount of time give to vote for a sale proposal.
		type SaleVotingTime: Get<BlockNumberFor<Self>>;

		/// The maximum amount of votes per block.
		type MaxVotesForBlock: Get<u32>;

		/// The minimum amount of a letting agent that will be slashed.
		type MinSlashingAmount: Get<Balance>;

		/// The maximum amount of users who can vote on an ongoing voting.
		type MaxVoter: Get<u32>;

		/// Threshold for challenge votes.
		type Threshold: Get<Percent>;

		/// Threshold for high costs challenge votes.
		type HighThreshold: Get<Percent>;

		#[cfg(feature = "runtime-benchmarks")]
		type Helper: crate::BenchmarkHelper<
			<Self as pallet_assets::Config<Instance1>>::AssetId,
			Self,
		>;

		/// Proposal amount to be considered a low proposal.
		type LowProposal: Get<Balance>;

		/// Proposal amount to be considered a high proposal.
		type HighProposal: Get<Balance>;

		/// The property governance's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type MarketplacePalletId: Get<PalletId>;

		/// The minimum amount of a sales agent that has to be deposited.
		type SalesAgentDeposit: Get<Balance>;

		/// Threshold for selling a property.
		type SalesThreshold: Get<Percent>;
	}

	pub type LocationId<T> = BoundedVec<u8, <T as pallet_nft_marketplace::Config>::PostcodeLimit>;

	/// Number of proposals that have been made.
	#[pallet::storage]
	pub(super) type ProposalCount<T> = StorageValue<_, ProposalIndex, ValueQuery>;

	/// Number of Challenges that have been made.
	#[pallet::storage]
	pub(super) type ChallengeCount<T> = StorageValue<_, ProposalIndex, ValueQuery>;

	/// Proposals that have been made.
	#[pallet::storage]
	pub(super) type Proposals<T> =
		StorageMap<_, Blake2_128Concat, ProposalIndex, Proposal<T>, OptionQuery>;

	/// Sell proposals that have been made.
	#[pallet::storage]
	pub(super) type SaleProposals<T> = StorageMap<
		_,
		Blake2_128Concat,
		ProposalIndex,
		SaleProposal<T>,
		OptionQuery,
	>;

	/// Mapping of challenge index to the challenge info.
	#[pallet::storage]
	pub(super) type Challenges<T> =
		StorageMap<_, Blake2_128Concat, ChallengeIndex, Challenge<BlockNumberFor<T>, T>, OptionQuery>;

	/// Mapping of ongoing votes.
	#[pallet::storage]
	pub(super) type OngoingVotes<T> =
		StorageMap<_, Blake2_128Concat, ProposalIndex, VoteStats, OptionQuery>;

	/// Mapping of ongoing sales votes.
	#[pallet::storage]
	pub(super) type OngoingSalesVotes<T> =
		StorageMap<_, Blake2_128Concat, ProposalIndex, VoteStats, OptionQuery>;

	/// Mapping from proposal to vector of users who voted.
	#[pallet::storage]
	pub(super) type ProposalVoter<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ProposalIndex,
		BoundedVec<AccountIdOf<T>, T::MaxVoter>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type UserProposalVote<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProposalIndex,
		Blake2_128Concat,
		AccountIdOf<T>,
		Vote,
		OptionQuery,	
	>;

	/// Mapping of ongoing votes about challenges.
	#[pallet::storage]
	pub(super) type OngoingChallengeVotes<T> =
		StorageDoubleMap<_, Blake2_128Concat, ChallengeIndex, Blake2_128Concat, ChallengeState, VoteStats, OptionQuery>;

	/// Mapping from challenge to vector of users who voted.
	#[pallet::storage]
	pub(super) type ChallengeVoter<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChallengeIndex,
		Blake2_128Concat,
		ChallengeState,
		BoundedVec<AccountIdOf<T>, T::MaxVoter>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type UserChallengeVote<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChallengeIndex,
		Blake2_128Concat,
		AccountIdOf<T>,
		Vote,
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
	pub type SaleProposalRoundExpiring<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<ProposalIndex, T::MaxVotesForBlock>,
		ValueQuery,
	>;

	/// Stores the project keys and round types ending on a given block for challenge votings.
	#[pallet::storage]
	pub type ChallengeRoundsExpiring<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<ChallengeIndex, T::MaxVotesForBlock>,
		ValueQuery,
	>;

	/// Stores the project keys and round types ending on a given block for sell_property votings.
	#[pallet::storage]
	pub type SellPropertyRoundsExpiring<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<ChallengeIndex, T::MaxVotesForBlock>,
		ValueQuery,
	>;

	/// Mapping from sales agent to the region he is active in.
	#[pallet::storage]
	pub type SalesAgentStorage<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		SalesAgentInfo<T>,
		OptionQuery,
	>;

	/// Mapping from asset id to the property sale details.
	#[pallet::storage]
	pub type PropertySale<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		PropertySaleInfo<T>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New proposal has been created.
		Proposed { proposal_id: ProposalIndex, asset_id: u32, proposer: AccountIdOf<T> },
		/// A new challenge has been made.
		Challenge { challenge_id: ChallengeIndex, asset_id: u32, proposer: AccountIdOf<T> },
		/// Voted on proposal.
		VotedOnProposal { proposal_id: ProposalIndex, voter: AccountIdOf<T>, vote: Vote },
		/// Voted on challenge.
		VotedOnChallenge { challenge_id: ChallengeIndex, voter: AccountIdOf<T>, vote: Vote },
		/// The proposal has been executed.
		ProposalExecuted { asset_id: u32, amount: Balance },
		/// The agent got slashed.
		AgentSlashed { challenge_id: ChallengeIndex, amount: Balance },
		/// The agent has been changed.
		AgentChanged { challenge_id: ChallengeIndex, asset_id: u32 },
		/// A proposal got rejected.
		ProposalRejected { proposal_id: ProposalIndex },
		/// A challenge has been rejected/
		ChallengeRejected { challenge_id: ChallengeIndex, challenge_state: ChallengeState },
		/// The threshold could not be reached for a proposal.
		ProposalThresHoldNotReached { proposal_id: ProposalIndex, required_threshold: Percent },
		/// The threshold could not be reached for a challenge.
		ChallengeThresHoldNotReached { challenge_id: ProposalIndex, required_threshold: Percent, challenge_state: ChallengeState },
		/// A Sales agent has been added to a location.
		SalesAgentAdded { region: u32, who: AccountIdOf<T> },
		/// A sales agent deposited the necessary funds.
		Deposited { who: AccountIdOf<T> },
		/// New sale proposal has been created.
		SaleProposed { sale_proposal_id: ProposalIndex, asset_id: u32, proposer: AccountIdOf<T> },
		/// A sale proposal got rejected.
		SaleProposalRejected { proposal_id: ProposalIndex },
		/// The threshold could not be reached for a sale proposal.
		SaleProposalThresHoldNotReached { proposal_id: ProposalIndex, required_threshold: Percent },
		/// Sales agent has been set for a property.
		SalesAgentSet {asset_id: u32, sales_agent: AccountIdOf<T> },
		/// Lawyer for a sale has been set.
		SalesLawyerSet {asset_id: u32, lawyer: AccountIdOf<T> },
		/// The sale got approved by the lawyer.
		LawyerApprovesSale { asset_id: u32, lawyer: AccountIdOf<T> },
		/// The sale got rejected by the lawyer.
		LawyerRejectsSale { asset_id: u32, lawyer: AccountIdOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// There are already too many proposals in the ending block.
		TooManyProposals,
		/// The proposal is not ongoing.
		NotOngoing,
		/// Too many user voted already.
		TooManyVotes,
		/// The assets details could not be found.
		NoAssetFound,
		/// There is no letting agent for this property.
		NoLettingAgentFound,
		/// The pallet has not enough funds.
		NotEnoughFunds,
		/// Error during converting types.
		ConversionError,
		/// The region is not registered.
		RegionUnknown,
		/// The caller is not authorized to call this extrinsic.
		NoPermission,
		/// The sales agent is already registered.
		SalesAgentExists,
		/// The sales agent is already active in too many locations.
		TooManyLocations,
		/// The sales already deposited the necessary amount.
		AlreadyDeposited,
		/// This sales agent has no location.
		NoLoactions,
		/// Real estate asset does not exist.
		AssetNotFound,
		/// This Agent has no authorization in the region.
		NoPermissionInRegion,
		/// This Agent has no authorization in the location.
		NoPermissionInLocation,
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: frame_system::pallet_prelude::BlockNumberFor<T>) -> Weight {
			let mut weight = T::DbWeight::get().reads_writes(1, 1);

			let ended_votings = ProposalRoundsExpiring::<T>::take(n);
			// checks if there is a voting for a proposal ending in this block.
			ended_votings.iter().for_each(|item| {
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));	
				let _ = Self::finish_proposal(*item);					
			});

			let ended_votings = SaleProposalRoundExpiring::<T>::take(n);
			// Checks if there is a voting for a sale porposal in this block;
			ended_votings.iter().for_each(|item| {
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
				let _ = Self::finish_sale_proposal(*item);
			});

			let ended_challenge_votings = ChallengeRoundsExpiring::<T>::take(n);
			// checks if there is a voting for an challenge ending in this block.
			ended_challenge_votings.iter().for_each(|item| {
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
				let _ = Self::finish_challenge(*item);
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
			amount: Balance,
			data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			ensure!(
				pallet_property_management::LettingStorage::<T>::get(asset_id)
					.ok_or(Error::<T>::NoLettingAgentFound)?
					== signer.clone(),
				Error::<T>::NoPermission
			);
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let proposal = Proposal {
				proposer: signer.clone(),
				asset_id,
				amount,
				created_at: current_block_number,
				proposal_info: data,
			};

			// Check if the amount is less than LowProposal
			if amount <= <T as Config>::LowProposal::get() {
				// Execute the proposal immediately
				return Self::execute_proposal(proposal);
			}

			let proposal_id = ProposalCount::<T>::get().saturating_add(1);
			let expiry_block =
				current_block_number.saturating_add(<T as Config>::VotingTime::get());
			ProposalRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
				keys.try_push(proposal_id).map_err(|_| Error::<T>::TooManyProposals)?;
				Ok::<(), DispatchError>(())
			})?;
			let vote_stats = VoteStats { yes_voting_power: 0, no_voting_power: 0 };

			Proposals::<T>::insert(proposal_id, proposal);
			OngoingVotes::<T>::insert(proposal_id, vote_stats);
			ProposalCount::<T>::put(proposal_id);
			Self::deposit_event(Event::Proposed { proposal_id, asset_id, proposer: signer });
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
			let owner_list = pallet_nft_marketplace::PropertyOwner::<T>::get(asset_id);
			ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
			ensure!(pallet_property_management::LettingStorage::<T>::get(asset_id).is_some(), Error::<T>::NoLettingAgentFound);
			let challenge_id = ChallengeCount::<T>::get().saturating_add(1);

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let expiry_block =
				current_block_number.saturating_add(<T as Config>::VotingTime::get());
			let challenge =
				Challenge { proposer: signer.clone(), asset_id, created_at: current_block_number, state: ChallengeState::First };
			ChallengeRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
				keys.try_push(challenge_id).map_err(|_| Error::<T>::TooManyProposals)?;
				Ok::<(), DispatchError>(())
			})?;
			let vote_stats = VoteStats { yes_voting_power: 0, no_voting_power: 0 };
			OngoingChallengeVotes::<T>::insert(challenge_id, challenge.state.clone(), vote_stats);
			Challenges::<T>::insert(challenge_id, challenge);
			ChallengeCount::<T>::put(challenge_id);
			
			Self::deposit_event(Event::Challenge { challenge_id, asset_id, proposer: signer });
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
			let owner_list = pallet_nft_marketplace::PropertyOwner::<T>::get(proposal.asset_id);
			ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
			let voting_power = pallet_nft_marketplace::PropertyOwnerToken::<T>::get(
				proposal.asset_id,
				signer.clone(),
			);
			OngoingVotes::<T>::try_mutate(proposal_id, |maybe_current_vote|{
				let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
				let previous_vote_opt = UserProposalVote::<T>::get(proposal_id, signer.clone());
				if let Some(previous_vote) = previous_vote_opt {
					match previous_vote {
						Vote::Yes => current_vote.yes_voting_power = current_vote.yes_voting_power.saturating_sub(voting_power),
						Vote::No => current_vote.no_voting_power = current_vote.no_voting_power.saturating_sub(voting_power),
					}
				}
				
				match vote {
					Vote::Yes => current_vote.yes_voting_power.saturating_accrue(voting_power),
					Vote::No => current_vote.no_voting_power.saturating_accrue(voting_power),
				}
				Ok::<(), DispatchError>(())
			})?;
			ProposalVoter::<T>::try_mutate(proposal_id, |keys| {
				keys.try_push(signer.clone()).map_err(|_| Error::<T>::TooManyVotes)?;
				Ok::<(), DispatchError>(())
			})?;
			UserProposalVote::<T>::insert(proposal_id, signer.clone(), vote.clone());
			Self::deposit_event(Event::VotedOnProposal { proposal_id, voter: signer, vote });
			Ok(())
		}

		/// Lets owner of the real estate object vote on an challenge.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// Parameters:
		/// - `challenge_id`: The index of the challenge.
		/// - `vote`: Must be either a Yes vote or a No vote.
		///
		/// Emits `VotedOnChallenge` event when succesfful.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_letting_agent_challenge())]
		pub fn vote_on_letting_agent_challenge(
			origin: OriginFor<T>,
			challenge_id: ChallengeIndex,
			vote: Vote,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let challenge = Challenges::<T>::get(challenge_id).ok_or(Error::<T>::NotOngoing)?;
			let owner_list = pallet_nft_marketplace::PropertyOwner::<T>::get(challenge.asset_id);
			ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
			let voting_power = pallet_nft_marketplace::PropertyOwnerToken::<T>::get(
				challenge.asset_id,
				signer.clone(),
			);
			OngoingChallengeVotes::<T>::try_mutate(challenge_id, challenge.state.clone(), |maybe_current_vote|{
				let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
				let previous_vote_opt = UserChallengeVote::<T>::get(challenge_id, signer.clone());
				if let Some(previous_vote) = previous_vote_opt {
					match previous_vote {
						Vote::Yes => current_vote.yes_voting_power = current_vote.yes_voting_power.saturating_sub(voting_power),
						Vote::No => current_vote.no_voting_power = current_vote.no_voting_power.saturating_sub(voting_power),
					}
				}
				
				match vote {
					Vote::Yes => current_vote.yes_voting_power.saturating_accrue(voting_power),
					Vote::No => current_vote.no_voting_power.saturating_accrue(voting_power),
				}
				Ok::<(), DispatchError>(())
			})?;
			ChallengeVoter::<T>::try_mutate(challenge_id, challenge.state, |keys| {
				keys.try_push(signer.clone()).map_err(|_| Error::<T>::TooManyVotes)?;
				Ok::<(), DispatchError>(())
			})?;
			UserChallengeVote::<T>::insert(challenge_id, signer.clone(), vote.clone());
			Self::deposit_event(Event::VotedOnChallenge { challenge_id, voter: signer, vote });
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn add_sales_agent(
			origin: OriginFor<T>,
			region: u32,
			location: LocationId<T>,
			sales_agent: AccountIdOf<T>,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let region_info = pallet_nft_marketplace::Regions::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
			ensure!(region_info.owner == signer, Error::<T>::NoPermission);
			ensure!(SalesAgentStorage::<T>::get(sales_agent.clone()).is_none(), Error::<T>::SalesAgentExists);
			let mut sales_info = SalesAgentInfo {
				account: sales_agent.clone(),
				region,
				locations: Default::default(),
				assigned_properties: Default::default(),
				deposited: Default::default(),
			};
			sales_info
				.locations
				.try_push(location)
				.map_err(|_| Error::<T>::TooManyLocations)?;
			SalesAgentStorage::<T>::insert(sales_agent.clone(), sales_info);
			Self::deposit_event(Event::<T>::SalesAgentAdded { region, who: sales_agent });
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn sales_agent_deposit(origin: OriginFor<T>) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			SalesAgentStorage::<T>::try_mutate(signer.clone(), |maybe_sales_info|{
				let sales_info = maybe_sales_info.as_mut().ok_or(Error::<T>::NoPermission)?;
				ensure!(!sales_info.deposited, Error::<T>::AlreadyDeposited);
				ensure!(!sales_info.locations.is_empty(), Error::<T>::NoLoactions);

				<T as pallet::Config>::NativeCurrency::hold(
					&<T as pallet_property_management::Config>::RuntimeHoldReason::from(pallet_property_management::HoldReason::SalesAgent),
					&signer, 
					<T as Config>::SalesAgentDeposit::get(),
				)?;

				sales_info.deposited = true;
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::<T>::Deposited { who: signer });
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn propose_property_sale(
			origin: OriginFor<T>,
			asset_id: u32,
			amount: Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let owner_list = pallet_nft_marketplace::PropertyOwner::<T>::get(asset_id);
			ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let sale_proposal = SaleProposal {
				proposer: signer.clone(),
				asset_id,
				amount,
				created_at: current_block_number,
			};
			let sale_proposal_id = ProposalCount::<T>::get().saturating_add(1);
			let expiry_block = current_block_number
				.saturating_add(<T as Config>::SaleVotingTime::get());
			SaleProposalRoundExpiring::<T>::try_mutate(expiry_block, |keys| {
				keys.try_push(sale_proposal_id).map_err(|_| Error::<T>::TooManyProposals)?;
				Ok::<(), DispatchError>(())
			})?;
			let vote_stats = VoteStats { yes_voting_power: 0, no_voting_power: 0 };

			SaleProposals::<T>::insert(sale_proposal_id, sale_proposal);
			OngoingSalesVotes::<T>::insert(sale_proposal_id, vote_stats);
			ProposalCount::<T>::set(sale_proposal_id);
			Self::deposit_event(Event::SaleProposed { sale_proposal_id, asset_id, proposer: signer });
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn agent_claim_sale(
			origin: OriginFor<T>,
			asset_id: u32,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let agent_info = SalesAgentStorage::<T>::get(signer.clone()).ok_or(Error::<T>::NoPermission)?;
			let asset_info = pallet_nft_marketplace::AssetIdDetails::<T>::get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
			ensure!(agent_info.region == asset_info.region, Error::<T>::NoPermissionInRegion);
			ensure!(agent_info.locations.contains(&asset_info.location), Error::<T>::NoPermissionInLocation);
			let mut property_sale_info = PropertySale::<T>::get(asset_id).ok_or(Error::<T>::NotForSale)?;
			property_sale_info.sales_agent = Some(signer.clone());
			PropertySale::<T>::insert(asset_id, property_sale_info);
			Self::deposit_event(Event::SalesAgentSet {asset_id, sales_agent: signer });
			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn lawyer_claim_sale(
			origin: OriginFor<T>,
			asset_id: u32,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let lawyer_region = pallet_nft_marketplace::RealEstateLawyer::<T>::get(signer.clone()).ok_or(Error::<T>::NoPermission)?;
			let asset_info = pallet_nft_marketplace::AssetIdDetails::<T>::get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
			ensure!(lawyer_region == asset_info.region, Error::<T>::NoPermissionInRegion);
			let mut property_sale_info = PropertySale::<T>::get(asset_id).ok_or(Error::<T>::NotForSale)?;
			property_sale_info.lawyer = Some(signer.clone());
			PropertySale::<T>::insert(asset_id, property_sale_info);
			Self::deposit_event(Event::SalesLawyerSet {asset_id, lawyer: signer });
			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn lawyer_confirm_sale(
			origin: OriginFor<T>,
			asset_id: u32,
			approve: bool,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			PropertySale::<T>::try_mutate(asset_id, |maybe_sale| -> DispatchResult {
				let property_sale_info = maybe_sale.as_mut().ok_or(Error::<T>::NotForSale)?;
				ensure!(property_sale_info.lawyer == Some(signer.clone()), Error::<T>::NoPermission);
				if approve == true {
					property_sale_info.lawyer_approval = true;
					Self::deposit_event(Event::LawyerApprovesSale{ asset_id, lawyer: signer });
				} else {
					*maybe_sale = None;
					Self::deposit_event(Event::LawyerRejectsSale{ asset_id, lawyer: signer });
				}
				Ok::<(), DispatchError>(())
			})?;
			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
		pub fn finalize_sale(
			origin: OriginFor<T>,
			asset_id: u32,
			amount: Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			PropertySale::<T>::try_mutate_exists(asset_id, |maybe_sale| -> DispatchResult {
				let property_sale_info = maybe_sale.take().ok_or(Error::<T>::NotForSale)?;
				ensure!(property_sale_info.sales_agent == Some(signer.clone()), Error::<T>::NoPermission);
				ensure!(property_sale_info.lawyer_approval, Error::<T>::SaleHasNotBeenApproved);

				let owner_list = pallet_nft_marketplace::PropertyOwner::<T>::get(asset_id);
				let property_info = pallet_nft_marketplace::AssetIdDetails::<T>::get(asset_id)
					.ok_or(Error::<T>::NoObjectFound)?;
				
				let total_token = property_info.token_amount;
				let property_account = pallet_nft_marketplace::Pallet::<T>::property_account_id(asset_id);
				for owner in owner_list {
					let token_amount = pallet_nft_marketplace::PropertyOwnerToken::<T>::get(
						asset_id,
						owner.clone(),
					);
					let amount_for_owner = (token_amount as u128)
						.checked_mul(amount)
						.ok_or(Error::<T>::MultiplyError)?
						.checked_div(total_token as u128)
						.ok_or(Error::<T>::DivisionError)?;
					<T as pallet::Config>::LocalCurrency::transfer(
						asset_id,
						&owner,
						&property_account,
						token_amount.into(),
						Preservation::Expendable,
					)?;	
					<T as pallet::Config>::ForeignCurrency::transfer(
						asset_id,
						&signer,
						&owner,
						amount_for_owner.into(),
						Preservation::Expendable,
					)?;	
				}
				Ok::<(), DispatchError>(())
			})?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
			<T as pallet::Config>::MarketplacePalletId::get().into_sub_account_truncating(("pr", asset_id))
		}

		// Slashes the letting agent.
		fn slash_letting_agent(challenge_id: ChallengeIndex) -> DispatchResult {
			let mut challenge = Challenges::<T>::take(challenge_id).ok_or(Error::<T>::NotOngoing)?;
			let letting_agent =
				pallet_property_management::LettingStorage::<T>::get(challenge.asset_id).ok_or(Error::<T>::NoLettingAgentFound)?;
			let amount = <T as Config>::MinSlashingAmount::get();
			let _slashed_amount = <T as pallet::Config>::NativeCurrency::transfer_on_hold(
				&<T as pallet_property_management::Config>::RuntimeHoldReason::from(pallet_property_management::HoldReason::LettingAgent),
				&letting_agent, 
				&Self::property_account_id(challenge.asset_id),
				amount,
				Precision::Exact,
				Restriction::Free,
				Fortitude::Force,
			)?;
			
			challenge.state = ChallengeState::Fourth;
			let vote_stats = VoteStats { yes_voting_power: 0, no_voting_power: 0 };
			OngoingChallengeVotes::<T>::insert(challenge_id, challenge.state.clone(), vote_stats);
			Challenges::<T>::insert(challenge_id, challenge.clone()); 
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let expiry_block =
				current_block_number.saturating_add(<T as Config>::VotingTime::get());
			ChallengeRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
				keys.try_push(challenge_id).map_err(|_| Error::<T>::TooManyProposals)?;
				Ok::<(), DispatchError>(())
			})?;
			Self::deposit_event(Event::AgentSlashed { challenge_id, amount });
			Ok(())
		}

		/// Changes the letting agent of a given real estate object.
		fn change_letting_agent(challenge_id: ChallengeIndex) -> DispatchResult {
			let challenge = Challenges::<T>::take(challenge_id).ok_or(Error::<T>::NotOngoing)?;
			let _ = pallet_property_management::Pallet::<T>::remove_bad_letting_agent(
				challenge.asset_id
			);
			Self::deposit_event(Event::AgentChanged { challenge_id, asset_id: challenge.asset_id });
			Ok(())
		}

		fn finish_proposal(proposal_id: ProposalIndex) -> DispatchResult {
			let voting_results = <OngoingVotes<T>>::take(proposal_id);
			let proposals = <Proposals<T>>::take(proposal_id);
			if let Some(proposal) = proposals {
				if let Some(voting_result) = voting_results {
					let required_threshold =
						if proposal.amount >= <T as Config>::HighProposal::get() {
							<T as Config>::HighThreshold::get()
						}  else {
							<T as Config>::Threshold::get()
						}; 
					let asset_details = pallet_nft_marketplace::AssetIdDetails::<T>::get(proposal.asset_id);
					if let Some(asset_details) = asset_details {
						let yes_votes_percentage = Percent::from_rational(voting_result.yes_voting_power, asset_details.token_amount);
						let no_votes_percentage = Percent::from_rational(voting_result.no_voting_power, asset_details.token_amount);

						if yes_votes_percentage > no_votes_percentage
							&& required_threshold
								< yes_votes_percentage.saturating_add(no_votes_percentage)
						{
							let _ = Self::execute_proposal(proposal);
						}
						else if yes_votes_percentage <= no_votes_percentage {
							Self::deposit_event(Event::ProposalRejected { proposal_id });
						} else {
							Self::deposit_event(Event::ProposalThresHoldNotReached { proposal_id, required_threshold });
						}								
					}
				}
			}
			Ok(())
		}

		fn finish_sale_proposal(proposal_id: ProposalIndex) -> DispatchResult {
			let voting_results = <OngoingSalesVotes<T>>::take(proposal_id);
			let sale_proposals = <SaleProposals<T>>::take(proposal_id);
			if let Some(sale_proposal) = sale_proposals {
				if let Some(voting_result) = voting_results {
					let required_threshold = T::SalesThreshold::get();
					let asset_details = pallet_nft_marketplace::AssetIdDetails::<T>::get(sale_proposal.asset_id);
					if let Some(asset_details) = asset_details {
						let yes_votes_percentage = Percent::from_rational(voting_result.yes_voting_power, asset_details.token_amount);
						let no_votes_percentage = Percent::from_rational(voting_result.no_voting_power, asset_details.token_amount);

						if yes_votes_percentage > no_votes_percentage 
							&& required_threshold < yes_votes_percentage.saturating_add(no_votes_percentage)
						{
							let _ = Self::execute_sale_proposal(sale_proposal);
						}
						else if yes_votes_percentage <= no_votes_percentage {
							Self::deposit_event(Event::SaleProposalRejected { proposal_id });
						} else {
							Self::deposit_event(Event::SaleProposalThresHoldNotReached { proposal_id, required_threshold });
						}	
					}
				}
			}
			Ok(())
		}

		fn finish_challenge(challenge_id: ChallengeIndex) -> DispatchResult {
			let challenge = Challenges::<T>::get(challenge_id);
			if let Some(mut challenge) = challenge {
				if challenge.state == ChallengeState::Second {
					challenge.state = ChallengeState::Third;
					let vote_stats = VoteStats { yes_voting_power: 0, no_voting_power: 0 };
					OngoingChallengeVotes::<T>::insert(challenge_id, challenge.state.clone(), vote_stats);
					Challenges::<T>::insert(challenge_id, challenge.clone());
					let current_block_number = <frame_system::Pallet<T>>::block_number();
					let expiry_block =
						current_block_number.saturating_add(<T as Config>::VotingTime::get());
					let _ = ChallengeRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
						keys.try_push(challenge_id).map_err(|_| Error::<T>::TooManyProposals)?;
						Ok::<(), DispatchError>(())
					});
				} 
				else {
					let voting_results = <OngoingChallengeVotes<T>>::take(challenge_id, challenge.state.clone());
					if let Some(voting_result) = voting_results {
						let asset_details = pallet_nft_marketplace::AssetIdDetails::<T>::get(challenge.asset_id);
						if let Some(asset_details) = asset_details {
							let yes_votes_percentage = Percent::from_rational(voting_result.yes_voting_power, asset_details.token_amount);
							let no_votes_percentage = Percent::from_rational(voting_result.no_voting_power, asset_details.token_amount);
							let required_threshold = <T as Config>::Threshold::get();
							if yes_votes_percentage > no_votes_percentage
								&& required_threshold
									< yes_votes_percentage.saturating_add(no_votes_percentage)
							{
								if challenge.state == ChallengeState::First {
									challenge.state = ChallengeState::Second;
									Challenges::<T>::insert(challenge_id, challenge.clone());
									let current_block_number = <frame_system::Pallet<T>>::block_number();
									let expiry_block =
										current_block_number.saturating_add(<T as Config>::VotingTime::get());
									let _ = ChallengeRoundsExpiring::<T>::try_mutate(expiry_block, |keys| {
										keys.try_push(challenge_id).map_err(|_| Error::<T>::TooManyProposals)?;
										Ok::<(), DispatchError>(())
									});
								} 
								if challenge.state == ChallengeState::Third {
									let _ = Self::slash_letting_agent(challenge_id);
								} 
								if challenge.state == ChallengeState::Fourth {
									let _ = Self::change_letting_agent(challenge_id);
								} 
							} else {
								Challenges::<T>::take(challenge_id);
								if yes_votes_percentage <= no_votes_percentage {
									Self::deposit_event(Event::ChallengeRejected { challenge_id, challenge_state: challenge.state});
								} else {
									Self::deposit_event(Event::ChallengeThresHoldNotReached { challenge_id, required_threshold, challenge_state: challenge.state });
								}	
							}
						}
					}
				}	
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
		fn execute_sale_proposal(sale_proposal: SaleProposal<T>) -> DispatchResult {
			let asset_id = sale_proposal.asset_id;

			let property_sale_info = PropertySaleInfo{
				sales_agent: None,
				lawyer: None,
				lawyer_approval: false,
			};

			PropertySale::<T>::insert(asset_id, property_sale_info);
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
