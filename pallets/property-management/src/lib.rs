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
    traits::{
        fungible::MutateHold,
        fungibles::Mutate as FungiblesMutate,
        fungibles::MutateFreeze,
        tokens::Preservation,
        tokens::{fungible, fungibles, Balance, Precision},
        EnsureOriginWithArg,
    },
    PalletId,
};

use frame_support::sp_runtime::{
    traits::{AccountIdConversion, Zero},
    Percent, Saturating,
};

use codec::Codec;

use pallet_real_estate_asset::traits::{PropertyTokenInspect, PropertyTokenSpvControl};

use primitives::MarketplaceFreezeReason;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type RuntimeHoldReasonOf<T> = <T as Config>::RuntimeHoldReason;

pub type ForeignAssetIdOf<T> = <<T as Config>::ForeignCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// A reason for the pallet placing a hold on funds.
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Funds are held to register for letting agent.
        #[codec(index = 0)]
        LettingAgent,
        /// Funds are held to register for sales agent.
        #[codec(index = 1)]
        SalesAgent,
    }

    /// Info for the letting agent.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct LettingAgentInfo<T: Config> {
        pub region: u16,
        pub locations: BoundedBTreeMap<LocationId<T>, LocationInfo<T>, T::MaxLocations>,
        pub active_strikes: BoundedBTreeMap<u32, u8, T::MaxProperties>,
    }

    /// Voting stats.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    pub struct VoteStats {
        pub yes_voting_power: u32,
        pub no_voting_power: u32,
    }

    #[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ProposedLettingAgent<T: Config> {
        pub letting_agent: AccountIdOf<T>,
        pub location: LocationId<T>,
        pub expiry_block: BlockNumberFor<T>,
    }

    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct LocationInfo<T: Config> {
        pub assigned_properties: u32,
        pub deposit: <T as pallet::Config>::Balance,
    }

    /// Vote record of a user.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct VoteRecord {
        pub vote: Vote,
        pub asset_id: u32,
        pub power: u32,
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

    #[pallet::config]
    pub trait Config:
        frame_system::Config
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

        /// The overarching hold reason.
        type RuntimeHoldReason: From<HoldReason>;

        /// The reservable currency type.
        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::MutateHold<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                Reason = RuntimeHoldReasonOf<Self>,
            > + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        type ForeignCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        type AssetsFreezer: fungibles::MutateFreeze<
            AccountIdOf<Self>,
            AssetId = u32,
            Balance = <Self as pallet::Config>::Balance,
            Id = MarketplaceFreezeReason,
        >;

        /// The property management's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type MarketplacePalletId: Get<PalletId>;

        /// Origin who can set a new letting agent.
        type AgentOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The minimum amount of a letting agent that has to be deposited.
        #[pallet::constant]
        type LettingAgentDeposit: Get<<Self as pallet::Config>::Balance>;

        /// The maximum amount of properties that can be assigned to a letting agent.
        #[pallet::constant]
        type MaxProperties: Get<u32>;

        /// The maximum amount of letting agents in a location.
        #[pallet::constant]
        type MaxLettingAgents: Get<u32>;

        /// The maximum amount of locations a letting agent can be assigned to.
        #[pallet::constant]
        type MaxLocations: Get<u32>;

        #[pallet::constant]
        type AcceptedAssets: Get<[u32; 2]>;

        type PropertyToken: PropertyTokenInspect<Self> + PropertyTokenSpvControl<Self>;

        /// The amount of time given to vote for a letting agent proposal.
        #[pallet::constant]
        type LettingAgentVotingTime: Get<BlockNumberFor<Self>>;

        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            pallet_xcavate_whitelist::Role,
            Success = Self::AccountId,
        >;

        #[pallet::constant]
        type MinVotingQuorum: Get<Percent>;
    }

    pub type ProposalId = u64;
    pub type LocationId<T> = BoundedVec<u8, <T as pallet_regions::Config>::PostcodeLimit>;

    /// Mapping from the real estate object to the letting agent.
    #[pallet::storage]
    pub type LettingStorage<T> = StorageMap<_, Blake2_128Concat, u32, AccountIdOf<T>, OptionQuery>;

    /// Mapping from account to currently stored balance.
    #[pallet::storage]
    pub type InvestorFunds<T> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, AccountIdOf<T>>,
            NMapKey<Blake2_128Concat, u32>,
            NMapKey<Blake2_128Concat, u32>,
        ),
        <T as pallet::Config>::Balance,
        ValueQuery,
    >;

    /// Mapping from account to letting agent info
    #[pallet::storage]
    pub type LettingInfo<T: Config> =
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, LettingAgentInfo<T>, OptionQuery>;

    /// Mapping of asset id to the ongoing letting agent proposal.
    #[pallet::storage]
    pub type LettingAgentProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, ProposedLettingAgent<T>, OptionQuery>;

    /// Mapping of ongoing letting agent vote.
    #[pallet::storage]
    pub type OngoingLettingAgentVoting<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats, OptionQuery>;

    /// Mapping of a asset id and account id to the vote of a user.
    #[pallet::storage]
    pub(super) type UserLettingAgentVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        AccountIdOf<T>,
        VoteRecord,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type AssetLettingProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ProposalId, OptionQuery>;

    /// Counter of proposal ids.
    #[pallet::storage]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new letting agent got set.
        LettingAgentAdded { region: u16, who: T::AccountId },
        /// A new letting has been removed from a location.
        LettingAgentRemoved {
            location: LocationId<T>,
            who: T::AccountId,
        },
        /// A letting agent has been added to a property.
        LettingAgentSet { asset_id: u32, who: T::AccountId },
        /// The rental income has been distributed.
        IncomeDistributed {
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
        },
        /// A user withdrew funds.
        WithdrawFunds {
            who: T::AccountId,
            amount: <T as pallet::Config>::Balance,
        },
        /// A letting agent has been proposed for a property.
        LettingAgentProposed {
            asset_id: u32,
            who: T::AccountId,
            proposal_id: ProposalId,
        },
        /// Someone has voted on a letting agent.
        VotedOnLettingAgent {
            asset_id: u32,
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
        },
        /// A letting agent has been rejected.
        LettingAgentRejected {
            asset_id: u32,
            letting_agent: T::AccountId,
        },
        /// A user has unfrozen his token.
        TokenUnfrozen {
            proposal_id: ProposalId,
            asset_id: u32,
            voter: AccountIdOf<T>,
            amount: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Error by convertion to balance type.
        ConversionError,
        /// Error by dividing a number.
        DivisionError,
        /// Error by multiplying a number.
        MultiplyError,
        ArithmeticOverflow,
        ArithmeticUnderflow,
        /// The caller has no funds stored.
        UserHasNoFundsStored,
        /// The pallet has not enough funds.
        NotEnoughFunds,
        /// The letting agent has already too many assigned properties.
        TooManyAssignedProperties,
        /// No letting agent could be selected.
        NoLettingAgentFound,
        /// The region is not registered.
        RegionUnknown,
        /// The location has already the maximum amount of letting agents.
        TooManyLettingAgents,
        /// The letting agent is already active in too many locations.
        TooManyLocations,
        /// The caller is not authorized to call this extrinsic.
        NoPermission,
        /// The letting agent of this property is already set.
        LettingAgentAlreadySet,
        /// The real estate object could not be found.
        NoObjectFound,
        /// The account is not a letting agent of this location.
        AgentNotFound,
        /// The location is not registered.
        LocationUnknown,
        /// The letting agent is already assigned to this location.
        LettingAgentInLocation,
        /// The letting agent is already registered.
        LettingAgentExists,
        /// This asset has no token.
        AssetNotFound,
        /// This Asset is not supported for payment.
        PaymentAssetNotSupported,
        /// No letting agent has been proposed for this property.
        NoLettingAgentProposed,
        /// The propal has expired.
        VotingExpired,
        /// User did not pass the kyc.
        UserNotWhitelisted,
        /// The voting is still ongoing.
        VotingStillOngoing,
        /// There is already a letting agent proposal ongoing.
        LettingAgentProposalOngoing,
        /// There are already too many voters for this voting.
        TooManyVoters,
        /// The account has not the role of a letting agent.
        AccountIsNotLettingAgent,
        /// The letting agent has is not responsible for this location.
        LocationNotFound,
        /// The letting agent is not active in this location.
        LettingAgentNotActiveInLocation,
        /// Letting agent still has active properties in location.
        LettingAgentActive,
        /// The user has no token amount frozen.
        NoFrozenAmount,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Adds an account as a letting agent.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region`: The region number where the letting agent should be added to.
        /// - `location`: The location number where the letting agent should be added to.
        /// - `letting_agent`: The account of the letting_agent.
        ///
        /// Emits `LettingAgentAdded` event when succesfful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::add_letting_agent())]
        pub fn add_letting_agent(
            origin: OriginFor<T>,
            region: u16,
            location: LocationId<T>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::LettingAgent,
            )?;
            ensure!(
                pallet_regions::RegionDetails::<T>::contains_key(region),
                Error::<T>::RegionUnknown
            );
            ensure!(
                pallet_regions::LocationRegistration::<T>::get(region, &location),
                Error::<T>::LocationUnknown
            );
            let deposit_amount = <T as Config>::LettingAgentDeposit::get();
            if let Some(mut letting_info) = LettingInfo::<T>::get(&signer) {
                ensure!(
                    !letting_info.locations.contains_key(&location),
                    Error::<T>::LettingAgentInLocation
                );
                <T as pallet::Config>::NativeCurrency::hold(
                    &HoldReason::LettingAgent.into(),
                    &signer,
                    deposit_amount,
                )?;
                letting_info
                    .locations
                    .try_insert(
                        location,
                        LocationInfo {
                            assigned_properties: 0,
                            deposit: deposit_amount,
                        },
                    )
                    .map_err(|_| Error::<T>::TooManyLocations)?;
                LettingInfo::<T>::insert(&signer, letting_info);
            } else {
                <T as pallet::Config>::NativeCurrency::hold(
                    &HoldReason::LettingAgent.into(),
                    &signer,
                    deposit_amount,
                )?;
                let mut letting_info = LettingAgentInfo {
                    region,
                    locations: Default::default(),
                    active_strikes: Default::default(),
                };
                letting_info
                    .locations
                    .try_insert(
                        location,
                        LocationInfo {
                            assigned_properties: 0,
                            deposit: deposit_amount,
                        },
                    )
                    .map_err(|_| Error::<T>::TooManyLocations)?;
                LettingInfo::<T>::insert(&signer, letting_info);
            }
            Self::deposit_event(Event::<T>::LettingAgentAdded {
                region,
                who: signer,
            });
            Ok(())
        }

        /// Removes a letting agent from a location.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `location`: The location where the letting agent should be removed from.
        ///
        /// Emits `LettingAgentRemoved` event when succesfful.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn remove_letting_agent(
            origin: OriginFor<T>,
            location: LocationId<T>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::LettingAgent,
            )?;
            let mut letting_info =
                LettingInfo::<T>::get(&signer).ok_or(Error::<T>::AgentNotFound)?;
            let location_info = letting_info
                .locations
                .get(&location)
                .ok_or(Error::<T>::LettingAgentNotActiveInLocation)?;
            ensure!(
                location_info.assigned_properties.is_zero(),
                Error::<T>::LettingAgentActive
            );
            let deposit_amount = location_info.deposit;
            letting_info
                .locations
                .remove(&location)
                .ok_or(Error::<T>::LocationNotFound)?;
            <T as pallet::Config>::NativeCurrency::release(
                &HoldReason::LettingAgent.into(),
                &signer,
                deposit_amount,
                Precision::Exact,
            )?;
            LettingInfo::<T>::insert(signer.clone(), letting_info);
            Self::deposit_event(Event::<T>::LettingAgentRemoved {
                location,
                who: signer,
            });
            Ok(())
        }

        /// Propose a letting agent for a property.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `LettingAgentProposed` event when succesfful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::letting_agent_propose())]
        pub fn letting_agent_propose(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::LettingAgent,
            )?;
            let property_info = T::PropertyToken::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;
            let letting_info = LettingInfo::<T>::get(&signer).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                letting_info.locations.contains_key(&property_info.location),
                Error::<T>::NoPermission
            );
            T::PropertyToken::ensure_property_finalized(asset_id)?;
            ensure!(
                LettingStorage::<T>::get(asset_id).is_none(),
                Error::<T>::LettingAgentAlreadySet
            );
            ensure!(
                !AssetLettingProposal::<T>::contains_key(asset_id),
                Error::<T>::LettingAgentProposalOngoing
            );
            let proposal_id = ProposalCounter::<T>::get();
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let expiry_block =
                current_block_number.saturating_add(T::LettingAgentVotingTime::get());
            AssetLettingProposal::<T>::insert(asset_id, proposal_id);
            LettingAgentProposal::<T>::insert(
                proposal_id,
                ProposedLettingAgent {
                    letting_agent: signer.clone(),
                    location: property_info.location,
                    expiry_block,
                },
            );
            OngoingLettingAgentVoting::<T>::insert(
                proposal_id,
                VoteStats {
                    yes_voting_power: 0,
                    no_voting_power: 0,
                },
            );
            let next_proposal_id = proposal_id
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCounter::<T>::put(next_proposal_id);
            Self::deposit_event(Event::<T>::LettingAgentProposed {
                asset_id,
                who: signer,
                proposal_id,
            });
            Ok(())
        }

        /// Vote for a letting agent.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `vote`: Must be either a Yes vote or a No vote.
        ///
        /// Emits `VotedOnLettingAgent` event when succesfful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_letting_agent())]
        pub fn vote_on_letting_agent(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let proposal_id = AssetLettingProposal::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let proposal_details = LettingAgentProposal::<T>::get(proposal_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                proposal_details.expiry_block > current_block_number,
                Error::<T>::VotingExpired
            );
            let voting_power = T::PropertyToken::get_token_balance(asset_id, &signer);
            ensure!(voting_power >= amount, Error::<T>::NoPermission);
            OngoingLettingAgentVoting::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote
                    .as_mut()
                    .ok_or(Error::<T>::NoLettingAgentProposed)?;
                UserLettingAgentVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    if let Some(previous_vote) = maybe_vote_record.take() {
                        T::AssetsFreezer::decrease_frozen(
                            asset_id,
                            &MarketplaceFreezeReason::LettingAgentVoting,
                            &signer,
                            previous_vote.power.into(),
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

                    T::AssetsFreezer::increase_frozen(
                        asset_id,
                        &MarketplaceFreezeReason::LettingAgentVoting,
                        &signer,
                        amount.into(),
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

                    *maybe_vote_record = Some(VoteRecord {
                        vote: vote.clone(),
                        asset_id,
                        power: amount,
                    });
                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnLettingAgent {
                asset_id,
                proposal_id,
                voter: signer,
                vote,
            });
            Ok(())
        }

        /// Lets someone finalize the letting agent process.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `LettingAgentSet` event when vote successful.
        /// Emits `LettingAgentRejected` event when vote unsuccessful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_letting_agent())]
        pub fn finalize_letting_agent(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let _ = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;

            let proposal_id = AssetLettingProposal::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let proposal = LettingAgentProposal::<T>::get(proposal_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                proposal.expiry_block <= current_block_number,
                Error::<T>::VotingStillOngoing
            );

            let voting_result = OngoingLettingAgentVoting::<T>::get(proposal_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;

            let asset_details =
                <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;
            let total_votes = voting_result
                .yes_voting_power
                .saturating_add(voting_result.no_voting_power);
            let total_supply = asset_details.token_amount;

            ensure!(total_supply > Zero::zero(), Error::<T>::NoObjectFound);

            let quorum_percent: u32 = T::MinVotingQuorum::get().deconstruct().into();

            let meets_quorum =
                total_votes.saturating_mul(100u32) > total_supply.saturating_mul(quorum_percent);
            if voting_result.yes_voting_power > voting_result.no_voting_power && meets_quorum {
                LettingInfo::<T>::try_mutate(
                    proposal.letting_agent.clone(),
                    |maybe_letting_info| {
                        let letting_info = maybe_letting_info
                            .as_mut()
                            .ok_or(Error::<T>::AgentNotFound)?;
                        ensure!(
                            LettingStorage::<T>::get(asset_id).is_none(),
                            Error::<T>::LettingAgentAlreadySet
                        );
                        if let Some(location_info) =
                            letting_info.locations.get_mut(&proposal.location)
                        {
                            location_info.assigned_properties = location_info
                                .assigned_properties
                                .checked_add(1)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        } else {
                            return Err(Error::<T>::LocationNotFound.into());
                        }
                        LettingStorage::<T>::insert(asset_id, proposal.letting_agent.clone());
                        Self::deposit_event(Event::<T>::LettingAgentSet {
                            asset_id,
                            who: proposal.letting_agent,
                        });
                        Ok::<(), DispatchError>(())
                    },
                )?;
            } else {
                Self::deposit_event(Event::LettingAgentRejected {
                    asset_id,
                    letting_agent: proposal.letting_agent,
                });
            }
            AssetLettingProposal::<T>::remove(asset_id);
            LettingAgentProposal::<T>::remove(proposal_id);
            OngoingLettingAgentVoting::<T>::remove(proposal_id);

            Ok(())
        }

        #[pallet::call_index(27)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn unfreeze_letting_voting_token(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record = UserLettingAgentVote::<T>::get(proposal_id, &signer)
                .ok_or(Error::<T>::NoFrozenAmount)?;

            if let Some(proposal) = LettingAgentProposal::<T>::get(proposal_id) {
                let current_block_number = frame_system::Pallet::<T>::block_number();
                ensure!(
                    proposal.expiry_block <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            T::AssetsFreezer::decrease_frozen(
                vote_record.asset_id,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &signer,
                vote_record.power.into(),
            )?;

            UserLettingAgentVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::TokenUnfrozen {
                proposal_id,
                asset_id: vote_record.asset_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets the letting agent distribute the income for a property.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `amount`: The amount of funds that should be distributed.
        ///
        /// Emits `IncomeDistributed` event when succesfful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::distribute_income())]
        pub fn distribute_income(
            origin: OriginFor<T>,
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::LettingAgent,
            )?;
            let letting_agent =
                LettingStorage::<T>::get(asset_id).ok_or(Error::<T>::NoLettingAgentFound)?;
            ensure!(letting_agent == signer, Error::<T>::NoPermission);
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );

            let scaled_amount = amount
                .checked_mul(&1u128.into()) // Modify the scale factor if needed
                .ok_or(Error::<T>::MultiplyError)?;

            <T as pallet::Config>::ForeignCurrency::transfer(
                payment_asset,
                &signer,
                &Self::property_account_id(asset_id),
                scaled_amount,
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughFunds)?;

            let owner_list = T::PropertyToken::get_property_owner(asset_id);
            let property_info = T::PropertyToken::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;

            let total_token = property_info.token_amount;
            for owner in owner_list {
                let token_amount = T::PropertyToken::get_token_balance(asset_id, &owner);
                let amount_for_owner = Self::u64_to_balance_option(token_amount as u64)?
                    .checked_mul(&amount)
                    .ok_or(Error::<T>::MultiplyError)?
                    .checked_div(&Self::u64_to_balance_option(total_token.into())?)
                    .ok_or(Error::<T>::DivisionError)?;
                InvestorFunds::<T>::try_mutate((&owner, asset_id, payment_asset), |stored| {
                    *stored = stored
                        .checked_add(&amount_for_owner)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    Ok::<(), DispatchError>(())
                })?;
            }

            Self::deposit_event(Event::<T>::IncomeDistributed { asset_id, amount });
            Ok(())
        }

        /// Lets a property owner withdraw the distributed funds.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Emits `WithdrawFunds` event when succesfful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_funds())]
        pub fn claim_income(
            origin: OriginFor<T>,
            asset_id: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );
            let amount = InvestorFunds::<T>::take((&signer, asset_id, payment_asset));
            ensure!(!amount.is_zero(), Error::<T>::UserHasNoFundsStored);
            <T as pallet::Config>::ForeignCurrency::transfer(
                payment_asset,
                &Self::property_account_id(asset_id),
                &signer,
                amount,
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughFunds)?;
            Self::deposit_event(Event::<T>::WithdrawFunds {
                who: signer,
                amount,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
            <T as pallet::Config>::MarketplacePalletId::get()
                .into_sub_account_truncating(("pr", asset_id))
        }

        /// Converts a u64 to a balance.
        pub fn u64_to_balance_option(
            input: u64,
        ) -> Result<<T as pallet::Config>::Balance, Error<T>> {
            input.try_into().map_err(|_| Error::<T>::ConversionError)
        }

        /// Removes bad letting agents.
        pub fn remove_bad_letting_agent(asset_id: u32) -> DispatchResult {
            let letting_agent =
                LettingStorage::<T>::take(asset_id).ok_or(Error::<T>::NoLettingAgentFound)?;
            let property_info = T::PropertyToken::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;
            LettingInfo::<T>::try_mutate(&letting_agent, |maybe_info| {
                let letting_info = maybe_info.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                if let Some(location_info) = letting_info.locations.get_mut(&property_info.location)
                {
                    location_info.assigned_properties = location_info
                        .assigned_properties
                        .checked_sub(1)
                        .ok_or(Error::<T>::ArithmeticUnderflow)?;
                } else {
                    return Err(Error::<T>::LocationNotFound.into());
                }
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }
    }
}

sp_api::decl_runtime_apis! {
    pub trait PropertyManagementApi<AccountId>
    where
        AccountId: Codec
    {
        fn get_management_account_id() -> AccountId;
    }
}
