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
        tokens::Preservation,
        tokens::{fungible, fungibles, Balance},
        EnsureOriginWithArg,
    },
    PalletId,
};

use frame_support::sp_runtime::{
    traits::{AccountIdConversion, Zero},
    Saturating,
};

use codec::Codec;

use pallet_real_estate_asset::traits::PropertyTokenInspect;

use pallet_xcavate_whitelist::HasRole;

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
        pub locations: BoundedVec<LocationId<T>, T::MaxLocations>,
        pub assigned_properties: BoundedVec<u32, T::MaxProperties>,
        pub deposited: bool,
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
        pub expiry_block: BlockNumberFor<T>,
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

        type PropertyToken: PropertyTokenInspect<Self>;

        /// The amount of time given to vote for a lawyer proposal.
        #[pallet::constant]
        type LettingAgentVotingTime: Get<BlockNumberFor<Self>>;

        type Whitelist: pallet_xcavate_whitelist::HasRole<Self::AccountId>;

        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            pallet_xcavate_whitelist::Role,
            Success = Self::AccountId,
        >;
    }

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
        StorageMap<_, Blake2_128Concat, u32, ProposedLettingAgent<T>, OptionQuery>;

    /// Mapping of ongoing letting agent vote.
    #[pallet::storage]
    pub type OngoingLettingAgentVoting<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, VoteStats, OptionQuery>;

    /// Mapping of a asset id and account id to the vote of a user.
    #[pallet::storage]
    pub(super) type UserLettingAgentVote<T: Config> = StorageMap<
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new letting agent got set.
        LettingAgentAdded { region: u16, who: T::AccountId },
        /// A letting agent deposited the necessary funds.
        Deposited { who: T::AccountId },
        /// A letting agent has been added to a location.
        LettingAgentAddedToLocation {
            who: T::AccountId,
            location: LocationId<T>,
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
        LettingAgentProposed { asset_id: u32, who: T::AccountId },
        /// Someone has voted on a letting agent.
        VotedOnLettingAgent {
            asset_id: u32,
            voter: T::AccountId,
            vote: Vote,
        },
        /// A letting agent has been rejected.
        LettingAgentRejected {
            asset_id: u32,
            letting_agent: T::AccountId,
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
        /// The letting already deposited the necessary amount.
        AlreadyDeposited,
        /// The location is not registered.
        LocationUnknown,
        /// The letting agent is already assigned to this location.
        LettingAgentInLocation,
        /// The letting agent has no funds deposited.
        NotDeposited,
        /// The letting agent is already registered.
        LettingAgentExists,
        /// This asset has no token.
        AssetNotFound,
        /// This letting agent has no location.
        NoLoactions,
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
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Adds an account as a letting agent.
        ///
        /// The origin must be the AgentOrigin.
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
            letting_agent: AccountIdOf<T>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            let region_info =
                pallet_regions::RegionDetails::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
            ensure!(region_info.owner == signer, Error::<T>::NoPermission);
            ensure!(
                pallet_regions::LocationRegistration::<T>::get(region, &location),
                Error::<T>::LocationUnknown
            );
            ensure!(<T as pallet::Config>::Whitelist::has_role(
                &letting_agent,
                pallet_xcavate_whitelist::Role::LettingAgent
            ), Error::<T>::AccountIsNotLettingAgent);
            ensure!(
                !LettingInfo::<T>::contains_key(&letting_agent),
                Error::<T>::LettingAgentExists
            );
            let mut letting_info = LettingAgentInfo {
                region,
                locations: Default::default(),
                assigned_properties: Default::default(),
                deposited: Default::default(),
                active_strikes: Default::default(),
            };
            letting_info
                .locations
                .try_push(location)
                .map_err(|_| Error::<T>::TooManyLocations)?;
            LettingInfo::<T>::insert(&letting_agent, letting_info);
            Self::deposit_event(Event::<T>::LettingAgentAdded {
                region,
                who: letting_agent,
            });
            Ok(())
        }

        /// Lets the letting agent deposit the required amount, to be able to operate as a letting agent.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Emits `Deposited` event when succesfful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::letting_agent_deposit())]
        pub fn letting_agent_deposit(origin: OriginFor<T>) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::LettingAgent,
            )?;
            LettingInfo::<T>::try_mutate(&signer, |maybe_letting_info| {
                let letting_info = maybe_letting_info
                    .as_mut()
                    .ok_or(Error::<T>::NoPermission)?;
                ensure!(!letting_info.deposited, Error::<T>::AlreadyDeposited);
                ensure!(!letting_info.locations.is_empty(), Error::<T>::NoLoactions);

                <T as pallet::Config>::NativeCurrency::hold(
                    &HoldReason::LettingAgent.into(),
                    &signer,
                    <T as Config>::LettingAgentDeposit::get(),
                )?;

                letting_info.deposited = true;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::<T>::Deposited { who: signer });
            Ok(())
        }

        /// Adds a letting agent to a location.
        ///
        /// The origin must be the AgentOrigin.
        ///
        /// Parameters:
        /// - `location`: The location number where the letting agent should be added to.
        /// - `letting_agent`: The account of the letting_agent.
        ///
        /// Emits `LettingAgentAddedToLocation` event when succesfful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::add_letting_agent_to_location())]
        pub fn add_letting_agent_to_location(
            origin: OriginFor<T>,
            location: LocationId<T>,
            letting_agent: AccountIdOf<T>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RegionalOperator,
            )?;
            LettingInfo::<T>::try_mutate(&letting_agent, |maybe_letting_info| {
                let letting_info = maybe_letting_info
                    .as_mut()
                    .ok_or(Error::<T>::NoLettingAgentFound)?;
                let region_info = pallet_regions::RegionDetails::<T>::get(letting_info.region)
                    .ok_or(Error::<T>::RegionUnknown)?;
                ensure!(region_info.owner == signer, Error::<T>::NoPermission);
                ensure!(letting_info.deposited, Error::<T>::NotDeposited);
                ensure!(
                    pallet_regions::LocationRegistration::<T>::get(letting_info.region, &location),
                    Error::<T>::LocationUnknown
                );
                ensure!(
                    !letting_info.locations.contains(&location),
                    Error::<T>::LettingAgentInLocation
                );
                letting_info
                    .locations
                    .try_push(location.clone())
                    .map_err(|_| Error::<T>::TooManyLocations)?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::<T>::LettingAgentAddedToLocation {
                who: letting_agent,
                location,
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
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::letting_agent_propose())]
        pub fn letting_agent_propose(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::LettingAgent,
            )?;
            ensure!(
                T::PropertyToken::get_property_asset_info(asset_id).is_some(),
                Error::<T>::NoObjectFound
            );
            ensure!(
                LettingStorage::<T>::get(asset_id).is_none(),
                Error::<T>::LettingAgentAlreadySet
            );
            ensure!(
                !LettingAgentProposal::<T>::contains_key(asset_id),
                Error::<T>::LettingAgentProposalOngoing
            );
            let letting_info = LettingInfo::<T>::get(&signer).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(letting_info.deposited, Error::<T>::NotDeposited);
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let expiry_block =
                current_block_number.saturating_add(T::LettingAgentVotingTime::get());
            LettingAgentProposal::<T>::insert(
                asset_id,
                ProposedLettingAgent {
                    letting_agent: signer.clone(),
                    expiry_block,
                },
            );
            OngoingLettingAgentVoting::<T>::insert(
                asset_id,
                VoteStats {
                    yes_voting_power: 0,
                    no_voting_power: 0,
                },
            );
            Self::deposit_event(Event::<T>::LettingAgentProposed {
                asset_id,
                who: signer,
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
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_letting_agent())]
        pub fn vote_on_letting_agent(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let proposal_details = LettingAgentProposal::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                proposal_details.expiry_block > current_block_number,
                Error::<T>::VotingExpired
            );
            let voting_power = T::PropertyToken::get_token_balance(asset_id, &signer);
            ensure!(!voting_power.is_zero(), Error::<T>::NoPermission);
            OngoingLettingAgentVoting::<T>::try_mutate(asset_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote
                    .as_mut()
                    .ok_or(Error::<T>::NoLettingAgentProposed)?;
                UserLettingAgentVote::<T>::try_mutate(asset_id, |maybe_map| {
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
            Self::deposit_event(Event::VotedOnLettingAgent {
                asset_id,
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
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_letting_agent())]
        pub fn finalize_letting_agent(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let _ = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;

            let proposal = LettingAgentProposal::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                proposal.expiry_block <= current_block_number,
                Error::<T>::VotingStillOngoing
            );

            let voting_result = OngoingLettingAgentVoting::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;

            if voting_result.yes_voting_power > voting_result.no_voting_power {
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
                        match letting_info.assigned_properties.try_push(asset_id) {
                            Ok(()) => {
                                LettingStorage::<T>::insert(
                                    asset_id,
                                    proposal.letting_agent.clone(),
                                );
                                Self::deposit_event(Event::<T>::LettingAgentSet {
                                    asset_id,
                                    who: proposal.letting_agent,
                                });
                            }
                            Err(_) => {
                                Self::deposit_event(Event::LettingAgentRejected {
                                    asset_id,
                                    letting_agent: proposal.letting_agent,
                                });
                            }
                        }
                        Ok::<(), DispatchError>(())
                    },
                )?;
            } else {
                Self::deposit_event(Event::LettingAgentRejected {
                    asset_id,
                    letting_agent: proposal.letting_agent,
                });
            }
            UserLettingAgentVote::<T>::remove(asset_id);
            LettingAgentProposal::<T>::remove(asset_id);
            OngoingLettingAgentVoting::<T>::remove(asset_id);

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
        #[pallet::call_index(6)]
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
        #[pallet::call_index(7)]
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
            LettingStorage::<T>::remove(asset_id);
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
