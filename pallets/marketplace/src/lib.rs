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
    storage::bounded_btree_map::BoundedBTreeMap,
    traits::{
        fungible::{Inspect, Mutate, MutateHold},
        fungibles::Mutate as FungiblesMutate,
        fungibles::MutateFreeze,
        fungibles::MutateHold as FungiblesHold,
        tokens::Preservation,
        tokens::{fungible, fungibles, Balance, Precision, WithdrawConsequence},
        EnsureOriginWithArg,
    },
    PalletId,
};

use frame_support::sp_runtime::{
    traits::{AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero},
    Percent, Permill, Saturating,
};

use codec::Codec;

use primitives::{MarketplaceFreezeReason, MarketplaceHoldReason};

use types::*;

use pallet_real_estate_asset::traits::{
    PropertyTokenInspect, PropertyTokenManage, PropertyTokenOwnership, PropertyTokenSpvControl,
};

use pallet_xcavate_whitelist::RolePermission;

use pallet_regions::{LawyerManagement, Pallet as PalletRegions};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type LocalAssetIdOf<T> = <<T as Config>::LocalCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

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

    #[pallet::composite_enum]
    pub enum HoldReason {
        #[codec(index = 0)]
        ListingDepositReserve,
    }

    /// The module configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_nfts::Config
        + pallet_regions::Config
        + pallet_nft_fractionalization::Config
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

        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<
                Self::AccountId,
                Reason = <Self as pallet::Config>::RuntimeHoldReason,
            >;

        /// The overarching hold reason.
        type RuntimeHoldReason: From<HoldReason>;

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

        type AssetsFreezer: fungibles::MutateFreeze<
            AccountIdOf<Self>,
            AssetId = u32,
            Balance = <Self as pallet::Config>::Balance,
            Id = MarketplaceFreezeReason,
        >;

        /// The marketplace's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The minimum amount of token of a property.
        #[pallet::constant]
        type MinPropertyToken: Get<u32>;

        /// The maximum amount of token of a property.
        #[pallet::constant]
        type MaxPropertyToken: Get<u32>;

        /// Asset id type from pallet nft fractionalization.
        type AssetId: IsType<<Self as pallet_nft_fractionalization::Config>::AssetId>
            + Parameter
            + From<u32>
            + Ord
            + Copy;

        /// The Trasury's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryId: Get<PalletId>;

        /// A deposit for listing a property.
        #[pallet::constant]
        type ListingDeposit: Get<<Self as pallet::Config>::Balance>;

        /// The fee percentage charged by the marketplace (e.g., 1 for 1%).
        #[pallet::constant]
        type MarketplaceFeePercentage: Get<<Self as pallet::Config>::Balance>;

        #[pallet::constant]
        type AcceptedAssets: Get<[u32; 2]>;

        /// The maximum amount of accepted assets.
        #[pallet::constant]
        type MaxAcceptedAssets: Get<u32>;

        type PropertyToken: PropertyTokenManage<Self>
            + PropertyTokenOwnership<Self>
            + PropertyTokenSpvControl<Self>
            + PropertyTokenInspect<Self>;

        /// The amount of time given to vote for a lawyer proposal.
        #[pallet::constant]
        type LawyerVotingTime: Get<BlockNumberFor<Self>>;

        /// The amount of time given for the lawyer to handle the legal process.
        #[pallet::constant]
        type LegalProcessTime: Get<BlockNumberFor<Self>>;

        type Whitelist: pallet_xcavate_whitelist::RolePermission<Self::AccountId>;

        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            pallet_xcavate_whitelist::Role,
            Success = Self::AccountId,
        >;

        type CompliantOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            pallet_xcavate_whitelist::Role,
            Success = Self::AccountId,
        >;

        #[pallet::constant]
        type MinVotingQuorum: Get<Percent>;

        #[pallet::constant]
        type ClaimWindow: Get<BlockNumberFor<Self>>;
        #[pallet::constant]
        type MaxRelistAttempts: Get<u8>;
    }

    pub type RegionId = u16;
    pub type ListingId = u32;
    pub type ProposalId = u64;
    pub type LocationId<T> = BoundedVec<u8, <T as pallet_regions::Config>::PostcodeLimit>;

    pub(super) type PropertyListingDetailsType<T> = PropertyListingDetails<
        <T as pallet_real_estate_asset::Config>::NftId,
        <T as pallet_regions::Config>::NftCollectionId,
        T,
    >;

    pub(super) type ListingDetailsType<T> = TokenListingDetails<
        <T as pallet_real_estate_asset::Config>::NftId,
        <T as pallet_regions::Config>::NftCollectionId,
        T,
    >;

    /// The Id for the next token listing.
    #[pallet::storage]
    pub(super) type NextListingId<T: Config> = StorageValue<_, ListingId, ValueQuery>;

    /// Mapping of the listing id to the ongoing property listing details.
    #[pallet::storage]
    pub(super) type OngoingObjectListing<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, PropertyListingDetailsType<T>, OptionQuery>;

    /// Double mapping of the account id of the token owner
    /// and the listing to the amount of token.
    #[pallet::storage]
    pub(super) type TokenOwner<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        ListingId,
        TokenOwnerDetails<T>,
        OptionQuery,
    >;

    /// Mapping of the listing id to the listing details of a token listing.
    #[pallet::storage]
    pub(super) type TokenListings<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ListingDetailsType<T>, OptionQuery>;

    /// Mapping from listing and offeror account id to the offer details.
    #[pallet::storage]
    pub(super) type OngoingOffers<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ListingId,
        Blake2_128Concat,
        AccountIdOf<T>,
        OfferDetails<T>,
        OptionQuery,
    >;

    /// Stores the lawyer related infos of a listing.
    #[pallet::storage]
    pub type PropertyLawyer<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, PropertyLawyerDetails<T>, OptionQuery>;

    /// Stores required infos in case of a refund.
    #[pallet::storage]
    pub type RefundToken<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, RefundInfos<T>, OptionQuery>;

    /// Stores required infos in case of a refund is a legal process expired.
    #[pallet::storage]
    pub type RefundLegalExpired<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, u32, OptionQuery>;

    /// Stores the deposit information of a listing.
    #[pallet::storage]
    pub type ListingDeposits<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ListingId,
        (AccountIdOf<T>, <T as pallet::Config>::Balance),
    >;

    /// Mapping of the listing to the real estate developer lawyer proposals.
    #[pallet::storage]
    pub type ProposedLawyers<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ProposedDeveloperLawyer<T>, OptionQuery>;

    /// Mapping of listing to the ongoing spv lawyer proposal.
    #[pallet::storage]
    pub type SpvLawyerProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, ProposedSpvLawyer<T>, OptionQuery>;

    /// Mapping of ongoing lawyer voted.
    #[pallet::storage]
    pub type OngoingLawyerVoting<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats, OptionQuery>;

    /// Mapping of a listing id and account id to the vote of a user.
    #[pallet::storage]
    pub(super) type UserLawyerVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        AccountIdOf<T>,
        VoteRecord,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type ListingSpvProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ProposalId, OptionQuery>;

    /// Counter of proposal ids.
    #[pallet::storage]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new object has been listed on the marketplace.
        ObjectListed {
            listing_index: ListingId,
            collection_index: <T as pallet_regions::Config>::NftCollectionId,
            item_index: <T as pallet_real_estate_asset::Config>::NftId,
            asset_id: u32,
            token_price: <T as pallet::Config>::Balance,
            token_amount: u32,
            total_valuation: <T as pallet::Config>::Balance,
            seller: AccountIdOf<T>,
            tax_paid_by_developer: bool,
            listing_expiry: BlockNumberFor<T>,
            metadata_blob: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
        },
        /// A token has been bought.
        RelistedTokenBought {
            listing_index: ListingId,
            asset_id: u32,
            buyer: AccountIdOf<T>,
            seller: AccountIdOf<T>,
            price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
            new_amount_remaining: u32,
        },
        /// Token from listed object have been bought.
        PropertyTokenBought {
            listing_index: ListingId,
            asset_id: u32,
            buyer: AccountIdOf<T>,
            amount_purchased: u32,
            price_paid: <T as pallet::Config>::Balance,
            tax_paid: <T as pallet::Config>::Balance,
            payment_asset: u32,
            new_tokens_remaining: u32,
            new_total_funds_for_asset: <T as pallet::Config>::Balance,
            new_total_tax_for_asset: <T as pallet::Config>::Balance,
        },
        /// Token have been listed.
        TokenRelisted {
            listing_index: ListingId,
            asset_id: u32,
            price: <T as pallet::Config>::Balance,
            token_amount: u32,
            seller: AccountIdOf<T>,
        },
        /// The property has been delisted.
        ListingDelisted { listing_index: ListingId },
        /// The price of the listed object has been updated.
        ObjectUpdated {
            listing_index: ListingId,
            new_price: <T as pallet::Config>::Balance,
        },
        /// A new offer has been made.
        OfferCreated {
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
        },
        /// An offer has been cancelled.
        OfferCancelled {
            listing_id: ListingId,
            account_id: AccountIdOf<T>,
        },
        /// A real estate developer lawyer claimed a property.
        DeveloperLawyerProposed {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            proposed_cost: <T as pallet::Config>::Balance,
        },
        /// A spv lawyer claimed a property.
        SpvLawyerProposed {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            proposed_cost: <T as pallet::Config>::Balance,
            expiry_block: BlockNumberFor<T>,
        },
        /// A lawyer stepped back from a legal case.
        LawyerRemovedFromCase {
            lawyer: AccountIdOf<T>,
            listing_id: ListingId,
        },
        /// Documents have been approved or rejected.
        DocumentsConfirmed {
            signer: AccountIdOf<T>,
            listing_id: ListingId,
            legal_side: LegalProperty,
            approve: bool,
        },
        /// The property deal has been successfully sold.
        PrimarySaleCompleted {
            listing_id: ListingId,
            asset_id: u32,
            payouts: FinalSettlementPayouts<T>,
        },
        /// Funds has been withdrawn.
        RejectedFundsWithdrawn {
            signer: AccountIdOf<T>,
            listing_id: ListingId,
        },
        /// Funds have been refunded after expired listing.
        ExpiredFundsWithdrawn {
            signer: AccountIdOf<T>,
            listing_id: ListingId,
        },
        /// An offer has been accepted.
        OfferAccepted {
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            amount: u32,
            price: <T as pallet::Config>::Balance,
        },
        /// An offer has been Rejected.
        OfferRejected {
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            amount: u32,
            price: <T as pallet::Config>::Balance,
        },
        /// A buy has been cancelled.
        InvestmentCancelled {
            listing_id: ListingId,
            investor: AccountIdOf<T>,
            amount_returned: u32,
            new_tokens_remaining: u32,
            principal_refunded_usdc: <T as pallet::Config>::Balance,
            tax_refunded_usdc: <T as pallet::Config>::Balance,
            principal_refunded_usdt: <T as pallet::Config>::Balance,
            tax_refunded_usdt: <T as pallet::Config>::Balance,
        },
        /// Property token have been sent to another account.
        PropertyTokenSend {
            asset_id: u32,
            sender: AccountIdOf<T>,
            receiver: AccountIdOf<T>,
            amount: u32,
        },
        /// The deposit of the real estate developer has been released.
        DeveloperDepositReturned {
            listing_id: ListingId,
            developer: AccountIdOf<T>,
            amount: <T as pallet::Config>::Balance,
        },
        /// Someone has voted on a lawyer.
        VotedOnLawyer {
            listing_id: ListingId,
            voter: AccountIdOf<T>,
            vote: Vote,
            voting_power: u32,
            new_yes_power: u32,
            new_no_power: u32,
            proposal_id: ProposalId,
        },
        /// The real estate developer lawyer has been finalized.
        RealEstateLawyerProposalFinalized {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            is_approved: bool,
        },
        /// The spv lawyer has been finalized.
        SpvLawyerVoteFinalized {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            is_approved: bool,
            final_yes_power: u32,
            final_no_power: u32,
        },
        PropertyTokenClaimed {
            listing_id: ListingId,
            asset_id: u32,
            owner: AccountIdOf<T>,
            amount: u32,
        },
        SpvCreated {
            listing_id: ListingId,
            asset_id: u32,
        },
        /// All token of a property have been sold.
        PrimarySaleSoldOut {
            listing_id: ListingId,
            asset_id: u32,
            claim_deadline: BlockNumberFor<T>,
        },
        /// All token of a property have been claimed.
        AllPropertyTokenClaimed {
            listing_id: ListingId,
            asset_id: u32,
            legal_process_expiry_block: BlockNumberFor<T>,
        },
        /// A user has unfrozen his token.
        TokenUnfrozen {
            proposal_id: ProposalId,
            asset_id: u32,
            voter: AccountIdOf<T>,
            amount: u32,
        },
        LawyerCostsAllocated {
            listing_id: ListingId,
            lawyer_account: AccountIdOf<T>,
            lawyer_type: LegalProperty,
            cost_in_usdc: <T as pallet::Config>::Balance,
            cost_in_usdt: <T as pallet::Config>::Balance,
        },
        UnclaimedRelisted {
            listing_id: ListingId,
            amount: u32,
            relist_count: u8,
        },
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
        /// Error by dividing a number.
        DivisionError,
        /// Error by multiplying a number.
        MultiplyError,
        /// No sufficient permission.
        NoPermission,
        /// User did not pass the kyc.
        UserNotWhitelisted,
        ArithmeticUnderflow,
        ArithmeticOverflow,
        /// The token is not for sale.
        TokenNotForSale,
        /// This Region is not known.
        RegionUnknown,
        /// The location is not registered.
        LocationUnknown,
        /// The object can not be divided in so many token.
        TooManyToken,
        /// The object needs more token.
        TokenAmountTooLow,
        /// A user can only make one offer per listing.
        OnlyOneOfferPerUser,
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
        /// This Asset is not supported for payment.
        PaymentAssetNotSupported,
        ExceedsMaxEntries,
        /// The property is not refunded.
        TokenNotRefunded,
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
        /// The real estate object could not be found.
        NoObjectFound,
        /// The lawyer has no permission for this region.
        WrongRegion,
        /// TokenOwnerHasNotBeenFound.
        TokenOwnerNotFound,
        /// No lawyer has been proposed to vote on.
        NoLawyerProposed,
        /// There is already a lawyer proposal ongoing.
        LawyerProposalOngoing,
        /// The propal has expired.
        VotingExpired,
        /// The voting is still ongoing.
        VotingStillOngoing,
        /// Property has not been sold yet.
        PropertyHasNotBeenSoldYet,
        /// The legal process was not finished on time.
        LegalProcessFailed,
        /// The legal process is currently ongoing.
        LegalProcessOngoing,
        /// The user has no token amount frozen.
        NoFrozenAmount,
        NoClaimWindow,
        ClaimWindowExpired,
        ClaimWindowNotExpired,
        StillHasUnclaimedToken,
        NoValidTokenToClaim,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
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
        /// - `tax_paid_by_developer`: Bool if the tax is paid by the real estate developer or not.
        ///
        /// Emits `ObjectListed` event when successful
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::list_object(
            <T as pallet_nfts::Config>::StringLimit::get()
        ))]
        pub fn list_property(
            origin: OriginFor<T>,
            region: RegionId,
            location: LocationId<T>,
            token_price: <T as pallet::Config>::Balance,
            token_amount: u32,
            data: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>,
            tax_paid_by_developer: bool,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateDeveloper,
            )?;
            ensure!(token_amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(
                token_amount <= <T as pallet::Config>::MaxPropertyToken::get(),
                Error::<T>::TooManyToken
            );
            ensure!(
                token_amount >= T::MinPropertyToken::get(),
                Error::<T>::TokenAmountTooLow
            );
            ensure!(!token_price.is_zero(), Error::<T>::InvalidTokenPrice);

            let region_info =
                pallet_regions::RegionDetails::<T>::get(region).ok_or(Error::<T>::RegionUnknown)?;
            ensure!(
                pallet_regions::LocationRegistration::<T>::get(region, &location),
                Error::<T>::LocationUnknown
            );
            let listing_id = NextListingId::<T>::get();
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            let listing_duration = region_info.listing_duration;
            let listing_expiry = current_block_number.saturating_add(listing_duration);

            let mut collected_funds = BoundedBTreeMap::default();
            for &asset_id in T::AcceptedAssets::get().iter() {
                collected_funds
                    .try_insert(asset_id, Default::default())
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }

            // Calculate listing deposit
            let property_price = token_price
                .checked_mul(&((token_amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;
            let deposit_amount = T::ListingDeposit::get();

            // Check signer balance before doing anything
            match <T as pallet::Config>::NativeCurrency::can_withdraw(&signer, deposit_amount) {
                WithdrawConsequence::Success => {}
                _ => return Err(Error::<T>::NotEnoughFunds.into()),
            }

            let (item_id, asset_number) = T::PropertyToken::create_property_token(
                &signer,
                region,
                location,
                token_amount,
                property_price.into(),
                data.clone(),
            )?;

            let property_details = PropertyListingDetails {
                real_estate_developer: signer.clone(),
                token_price,
                collected_funds: collected_funds.clone(),
                collected_tax: collected_funds.clone(),
                collected_fees: collected_funds,
                asset_id: asset_number,
                item_id,
                collection_id: region_info.collection_id,
                token_amount,
                listed_token_amount: token_amount,
                tax_paid_by_developer,
                tax: region_info.tax,
                listing_expiry,
                investor_funds: Default::default(),
                buyers: Default::default(),
                claim_expiry: None,
                relist_count: Zero::zero(),
                unclaimed_token_amount: Zero::zero(),
            };
            OngoingObjectListing::<T>::insert(listing_id, property_details);

            <T as pallet::Config>::NativeCurrency::hold(
                &HoldReason::ListingDepositReserve.into(),
                &signer,
                deposit_amount,
            )?;

            ListingDeposits::<T>::insert(listing_id, (&signer, deposit_amount));

            let next_listing_id = Self::next_listing_id(listing_id)?;

            NextListingId::<T>::put(next_listing_id);

            Self::deposit_event(Event::<T>::ObjectListed {
                listing_index: listing_id,
                collection_index: region_info.collection_id,
                item_index: item_id,
                asset_id: asset_number,
                token_price,
                token_amount,
                total_valuation: property_price,
                seller: signer,
                tax_paid_by_developer,
                listing_expiry,
                metadata_blob: data,
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
        /// Emits `PropertyTokenBought` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::buy_property_token_all_token(
            <T as pallet::Config>::MaxPropertyToken::get(),
            <T as pallet::Config>::AcceptedAssets::get().len() as u32
        ))]
        pub fn buy_property_token(
            origin: OriginFor<T>,
            listing_id: ListingId,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            let accepted_payment_assets = T::AcceptedAssets::get();
            ensure!(
                accepted_payment_assets.contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );

            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
            ensure!(
                property_details.listed_token_amount >= amount,
                Error::<T>::NotEnoughTokenAvailable
            );
            ensure!(
                property_details.listing_expiry > <frame_system::Pallet<T>>::block_number(),
                Error::<T>::ListingExpired
            );
            let asset_details =
                T::PropertyToken::get_if_spv_not_created(property_details.asset_id)?;
            let region_info = pallet_regions::RegionDetails::<T>::get(asset_details.region)
                .ok_or(Error::<T>::RegionUnknown)?;

            let fee_percent = T::MarketplaceFeePercentage::get();
            ensure!(
                fee_percent < 100u128.into(),
                Error::<T>::InvalidFeePercentage
            );
            let tax_percent = region_info.tax;
            ensure!(
                tax_percent < Permill::from_percent(100),
                Error::<T>::InvalidTaxPercentage
            );

            let transfer_price = property_details
                .token_price
                .checked_mul(&((amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;
            let fee = transfer_price
                .checked_mul(&fee_percent)
                .ok_or(Error::<T>::MultiplyError)?
                .checked_div(&100u128.into())
                .ok_or(Error::<T>::DivisionError)?;
            let tax = tax_percent.mul_floor(transfer_price);

            let base_price = transfer_price
                .checked_add(&fee)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let total_transfer_price = if property_details.tax_paid_by_developer {
                base_price
            } else {
                base_price
                    .checked_add(&tax)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
            };

            T::ForeignAssetsHolder::hold(
                payment_asset,
                &MarketplaceHoldReason::Marketplace,
                &signer,
                total_transfer_price,
            )?;

            property_details.listed_token_amount = property_details
                .listed_token_amount
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            property_details.unclaimed_token_amount = property_details
                .unclaimed_token_amount
                .checked_add(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            TokenOwner::<T>::try_mutate_exists(&signer, listing_id, |maybe_token_owner_details| {
                if maybe_token_owner_details.is_none() {
                    let initial_funds = Self::create_initial_funds()?;
                    *maybe_token_owner_details = Some(TokenOwnerDetails {
                        token_amount: 0,
                        paid_funds: initial_funds.clone(),
                        paid_tax: initial_funds,
                        relist_count: property_details.relist_count,
                    });
                }

                let token_owner_details = maybe_token_owner_details
                    .as_mut()
                    .ok_or(Error::<T>::TokenOwnerNotFound)?;
                ensure!(token_owner_details.relist_count == property_details.relist_count, Error::<T>::StillHasUnclaimedToken);

                token_owner_details.token_amount = token_owner_details
                    .token_amount
                    .checked_add(amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                Self::update_map(
                    &mut token_owner_details.paid_funds,
                    payment_asset,
                    transfer_price,
                )?;

                if !property_details.tax_paid_by_developer {
                    Self::update_map(&mut token_owner_details.paid_tax, payment_asset, tax)?;
                }

                Ok::<(), DispatchError>(())
            })?;
            if !property_details.buyers.contains(&signer) {
                property_details
                    .buyers
                    .try_insert(signer.clone())
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }
            
            let asset_id = property_details.asset_id;
            let tax_paid_by_developer = property_details.tax_paid_by_developer;
            let listed_token = property_details.listed_token_amount;
            if listed_token == 0 {
                let current_block_number = <frame_system::Pallet<T>>::block_number();
                let expiry_block = current_block_number.saturating_add(T::ClaimWindow::get());
                property_details.claim_expiry = Some(expiry_block);
                Self::deposit_event(Event::<T>::PrimarySaleSoldOut {
                    listing_id,
                    asset_id,
                    claim_deadline: expiry_block,
                });
            }
            OngoingObjectListing::<T>::insert(listing_id, &property_details);
            let new_total_funds_for_asset = *property_details
                .collected_funds
                .get(&payment_asset)
                .unwrap_or(&0u128.into());

            let new_total_tax_for_asset = *property_details
                .collected_tax
                .get(&payment_asset)
                .unwrap_or(&0u128.into());
            Self::deposit_event(Event::<T>::PropertyTokenBought {
                listing_index: listing_id,
                asset_id,
                buyer: signer,
                amount_purchased: amount,
                price_paid: transfer_price,
                tax_paid: if !tax_paid_by_developer {
                    tax
                } else {
                    0u128.into()
                },
                payment_asset,
                new_tokens_remaining: listed_token,
                new_total_funds_for_asset,
                new_total_tax_for_asset,
            });
            Ok(())
        }

        /// Claim purchased property token once all token are sold.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to claim token from.
        ///
        /// Emits `PropertyTokenClaimed` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::claim_property_token())]
        pub fn claim_property_token(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_some(),
                Error::<T>::PropertyHasNotBeenSoldYet
            );
            let claim_expiry = property_details.claim_expiry.ok_or(Error::<T>::NoClaimWindow)?;
            ensure!(
                <frame_system::Pallet<T>>::block_number() < claim_expiry,
                Error::<T>::ClaimWindowExpired
            );
            let token_details =
                TokenOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::TokenOwnerNotFound)?;
            ensure!(token_details.relist_count == property_details.relist_count, Error::<T>::NoValidTokenToClaim);
            let property_account = Self::property_account_id(property_details.asset_id);
            let fee_percent = T::MarketplaceFeePercentage::get();
            ensure!(
                fee_percent < 100u128.into(),
                Error::<T>::InvalidFeePercentage
            );

            // Process each payment asset
            for (asset, paid_funds) in token_details
                .paid_funds
                .iter()
                .filter(|(_, funds)| !funds.is_zero())
            {
                let default = Default::default();
                let paid_tax = token_details
                    .paid_tax
                    .get(asset)
                    .copied()
                    .unwrap_or(default);
                // Calculate investor's fee (1% of paid_funds)
                let investor_fee = paid_funds
                    .checked_mul(&fee_percent)
                    .ok_or(Error::<T>::MultiplyError)?
                    .checked_div(&100u128.into())
                    .ok_or(Error::<T>::DivisionError)?;

                Self::update_map(
                    &mut property_details.collected_funds,
                    *asset,
                    *paid_funds,
                )?;
                Self::update_map(&mut property_details.collected_fees, *asset, investor_fee)?;
                if !property_details.tax_paid_by_developer {
                    Self::update_map(&mut property_details.collected_tax, *asset, paid_tax)?;
                }

                // Total amount to unfreeze (paid_funds + fee + tax)
                let total_investor_amount = paid_funds
                    .checked_add(&investor_fee)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
                    .checked_add(&paid_tax)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                T::ForeignAssetsHolder::release(
                    *asset,
                    &MarketplaceHoldReason::Marketplace,
                    &signer,
                    total_investor_amount,
                    Precision::Exact,
                )?;

                // Transfer funds to property account
                Self::transfer_funds(&signer, &property_account, total_investor_amount, *asset)?;

                let investor_net_contribution = paid_funds
                    .checked_add(&paid_tax)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                match property_details.investor_funds.get_mut(&signer) {
                    Some(token_funds) => {
                        let paid_funds = &mut token_funds.paid_funds;
                        if let Some(existing) = paid_funds.get_mut(asset) {
                            *existing = existing
                                .checked_add(&investor_net_contribution)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        } else {
                            paid_funds
                                .try_insert(*asset, investor_net_contribution)
                                .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                        }
                        let paid_fee = &mut token_funds.paid_fee;
                        if let Some(existing) = paid_fee.get_mut(asset) {
                            *existing = existing
                                .checked_add(&investor_fee)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        } else {
                            paid_fee
                                .try_insert(*asset, investor_fee)
                                .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                        }
                    }
                    None => {
                        let mut paid_funds = BoundedBTreeMap::new();
                        paid_funds
                            .try_insert(*asset, investor_net_contribution)
                            .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                        let mut paid_fee = BoundedBTreeMap::new();
                        paid_fee
                            .try_insert(*asset, investor_fee)
                            .map_err(|_| Error::<T>::ExceedsMaxEntries)?;

                        let new_entry = TokenOwnerFunds {
                            paid_funds,
                            paid_fee,
                        };
                        property_details
                            .investor_funds
                            .try_insert(signer.clone(), new_entry)
                            .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                    }
                }
            }

            // Distribute property tokens
            let token_amount = token_details.token_amount;
            let asset_id = property_details.asset_id;

            T::PropertyToken::distribute_property_token_to_owner(asset_id, &signer, token_amount)?;
            property_details.buyers.remove(&signer);
            property_details.unclaimed_token_amount = property_details
                .unclaimed_token_amount
                .checked_sub(token_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            if property_details.buyers.is_empty() {
                ensure!(PropertyLawyer::<T>::get(listing_id).is_none(), Error::<T>::LegalProcessOngoing);
                let initial_funds = Self::create_initial_funds()?;
                let current_block_number = <frame_system::Pallet<T>>::block_number();
                let expiry_block = current_block_number.saturating_add(T::LegalProcessTime::get());
                let property_lawyer_details = PropertyLawyerDetails {
                    real_estate_developer_lawyer: None,
                    spv_lawyer: None,
                    real_estate_developer_status: DocumentStatus::Pending,
                    spv_status: DocumentStatus::Pending,
                    real_estate_developer_lawyer_costs: initial_funds.clone(),
                    spv_lawyer_costs: initial_funds,
                    legal_process_expiry: expiry_block,
                    second_attempt: false,
                };
                property_details.claim_expiry = None;
                PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                Self::deposit_event(Event::<T>::AllPropertyTokenClaimed {
                    listing_id,
                    asset_id,
                    legal_process_expiry_block: expiry_block,
                });
            }

            OngoingObjectListing::<T>::insert(listing_id, property_details);
            Self::deposit_event(Event::<T>::PropertyTokenClaimed {
                listing_id,
                asset_id,
                owner: signer,
                amount: token_amount,
            });
            Ok(())
        }

        #[pallet::call_index(30)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn finalize_claim_window(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let _ = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::NoObjectFound)?;
            let claim_expiry = property_details.claim_expiry.ok_or(Error::<T>::NoClaimWindow)?;
            let current_block = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block > claim_expiry, Error::<T>::ClaimWindowNotExpired);
            
            let unclaimed_amount = property_details.unclaimed_token_amount;
            if unclaimed_amount == 0 {
                ensure!(
                    PropertyLawyer::<T>::get(listing_id).is_some(),
                    Error::<T>::TokenNotForSale
                );
                property_details.claim_expiry = None;
                OngoingObjectListing::<T>::insert(listing_id, &property_details);
            } else {
                if property_details.relist_count >= T::MaxRelistAttempts::get() {
                
                } else {
                    property_details.listed_token_amount = property_details
                        .listed_token_amount
                        .checked_add(unclaimed_amount)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    property_details.relist_count = property_details
                        .relist_count
                        .checked_add(1)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    property_details.unclaimed_token_amount = 0;
                    property_details.buyers.clear();
                    property_details.claim_expiry = None;
                    OngoingObjectListing::<T>::insert(listing_id, &property_details);
                    Self::deposit_event(Event::<T>::UnclaimedRelisted {
                        listing_id,
                        amount: unclaimed_amount,
                        relist_count: property_details.relist_count,
                    });
                }
            }
            Ok(())
        }

        /// Confirm that a spv has been created.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the spv has been created for.
        ///
        /// Emits `SpvCreated` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn create_spv(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let _ = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::SpvConfirmation,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::NoObjectFound)?;
            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_some(),
                Error::<T>::PropertyHasNotBeenSoldYet
            );
            T::PropertyToken::ensure_spv_not_created(property_details.asset_id)?;
            T::PropertyToken::register_spv(property_details.asset_id)?;
            Self::deposit_event(Event::<T>::SpvCreated {
                listing_id,
                asset_id: property_details.asset_id,
            });
            Ok(())
        }

        /// Relist token on the marketplace.
        /// The property must be registered on the marketplace.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `region`: The region where the object is located.
        /// - `item_id`: The item id of the nft.
        /// - `token_price`: The price of a single token.
        /// - `amount`: The amount of token of the real estate object that should be listed.
        ///
        /// Emits `TokenRelisted` event when successful
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::relist_token())]
        pub fn relist_token(
            origin: OriginFor<T>,
            asset_id: u32,
            token_price: <T as pallet::Config>::Balance,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(!token_price.is_zero(), Error::<T>::InvalidTokenPrice);

            let asset_details = T::PropertyToken::get_if_property_finalized(asset_id)?;

            let property_account = Self::property_account_id(asset_id);
            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                &signer,
                &property_account,
                amount.into(),
                Preservation::Expendable,
            )?;
            let listing_id = NextListingId::<T>::get();

            let token_listing = TokenListingDetails {
                seller: signer.clone(),
                token_price,
                asset_id,
                item_id: asset_details.item_id,
                collection_id: asset_details.collection_id,
                amount,
            };
            TokenListings::<T>::insert(listing_id, token_listing);
            let next_listing_id = Self::next_listing_id(listing_id)?;
            NextListingId::<T>::put(next_listing_id);

            Self::deposit_event(Event::<T>::TokenRelisted {
                listing_index: listing_id,
                asset_id,
                price: token_price,
                token_amount: amount,
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
        /// Emits `RelistedTokenBought` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::buy_relisted_token())]
        pub fn buy_relisted_token(
            origin: OriginFor<T>,
            listing_id: ListingId,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let buyer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            let listing_details =
                TokenListings::<T>::take(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
            ensure!(
                listing_details.amount >= amount,
                Error::<T>::NotEnoughTokenAvailable
            );
            let price = listing_details
                .token_price
                .checked_mul(&((amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;
            Self::buying_token_process(
                listing_id,
                &buyer,
                &buyer,
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
        /// Emits `InvestmentCancelled` event when successful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_property_purchase())]
        pub fn cancel_property_purchase(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            ensure!(
                property_details.listing_expiry > <frame_system::Pallet<T>>::block_number(),
                Error::<T>::ListingExpired
            );
            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_none(),
                Error::<T>::PropertyAlreadySold
            );

            let token_details: TokenOwnerDetails<T> =
                TokenOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::TokenOwnerNotFound)?;
            ensure!(
                !token_details.token_amount.is_zero(),
                Error::<T>::NoTokenBought
            );

            // Process refunds
            let (
                principal_refunded_usdc,
                tax_refunded_usdc,
                principal_refunded_usdt,
                tax_refunded_usdt,
            ) = Self::unfreeze_token_with_refunds(&mut property_details, &token_details, &signer)?;
            property_details.listed_token_amount = property_details
                .listed_token_amount
                .checked_add(token_details.token_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            OngoingObjectListing::<T>::insert(listing_id, &property_details);

            Self::deposit_event(Event::<T>::InvestmentCancelled {
                listing_id,
                investor: signer,
                amount_returned: token_details.token_amount,
                new_tokens_remaining: property_details.listed_token_amount,
                principal_refunded_usdc,
                tax_refunded_usdc,
                principal_refunded_usdt,
                tax_refunded_usdt,
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
        /// Emits `OfferCreated` event when successful.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::make_offer())]
        pub fn make_offer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            offer_price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );
            ensure!(
                OngoingOffers::<T>::get(listing_id, &signer).is_none(),
                Error::<T>::OnlyOneOfferPerUser
            );
            let listing_details =
                TokenListings::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
            ensure!(
                listing_details.amount >= amount,
                Error::<T>::NotEnoughTokenAvailable
            );
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(!offer_price.is_zero(), Error::<T>::InvalidTokenPrice);
            let price = offer_price
                .checked_mul(&((amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;

            T::ForeignAssetsHolder::hold(
                payment_asset,
                &MarketplaceHoldReason::Marketplace,
                &signer,
                price,
            )?;
            let offer_details = OfferDetails {
                token_price: offer_price,
                amount,
                payment_assets: payment_asset,
            };
            OngoingOffers::<T>::insert(listing_id, &signer, offer_details);
            Self::deposit_event(Event::<T>::OfferCreated {
                listing_id,
                offeror: signer,
                price: offer_price,
                amount,
                payment_asset,
            });
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
        /// Emits `OfferAccepted` event when offer gets accepted successfully.
        /// Emits `OfferRejected` event when offer gets rejected successfully.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::handle_offer())]
        pub fn handle_offer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            offer: Offer,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let listing_details =
                TokenListings::<T>::get(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
            ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
            let offer_details = OngoingOffers::<T>::take(listing_id, offeror.clone())
                .ok_or(Error::<T>::InvalidIndex)?;
            ensure!(
                listing_details.amount >= offer_details.amount,
                Error::<T>::NotEnoughTokenAvailable
            );
            let price = offer_details.get_total_amount()?;
            T::ForeignAssetsHolder::release(
                offer_details.payment_assets,
                &MarketplaceHoldReason::Marketplace,
                &offeror,
                price,
                Precision::Exact,
            )?;
            match offer {
                Offer::Accept => {
                    Self::buying_token_process(
                        listing_id,
                        &offeror,
                        &offeror,
                        listing_details,
                        price,
                        offer_details.amount,
                        offer_details.payment_assets,
                    )?;
                    Self::deposit_event(Event::<T>::OfferAccepted {
                        listing_id,
                        offeror,
                        amount: offer_details.amount,
                        price,
                    });
                }
                Offer::Reject => {
                    Self::deposit_event(Event::<T>::OfferRejected {
                        listing_id,
                        offeror,
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
        /// Emits `OfferCancelled` event when successful.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_offer())]
        pub fn cancel_offer(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let offer_details =
                OngoingOffers::<T>::take(listing_id, &signer).ok_or(Error::<T>::InvalidIndex)?;
            let price = offer_details.get_total_amount()?;
            T::ForeignAssetsHolder::release(
                offer_details.payment_assets,
                &MarketplaceHoldReason::Marketplace,
                &signer,
                price,
                Precision::Exact,
            )?;
            Self::deposit_event(Event::<T>::OfferCancelled {
                listing_id,
                account_id: signer,
            });
            Ok(())
        }

        /// Lets the investor withdraw his funds after a property deal was unsuccessful.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to withdraw from.
        ///
        /// Emits `RejectedFundsWithdrawn` event when successful.
        #[pallet::call_index(10)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_rejected())]
        pub fn withdraw_rejected(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let property_account = Self::property_account_id(property_details.asset_id);
            let token_amount = <T as pallet::Config>::PropertyToken::get_token_balance(
                property_details.asset_id,
                &signer,
            );
            ensure!(!token_amount.is_zero(), Error::<T>::NoPermission);
            let mut refund_infos =
                RefundToken::<T>::take(listing_id).ok_or(Error::<T>::TokenNotRefunded)?;
            refund_infos.refund_amount = refund_infos
                .refund_amount
                .checked_sub(token_amount)
                .ok_or(Error::<T>::NotEnoughTokenAvailable)?;

            for &asset in T::AcceptedAssets::get().iter() {
                if let Some(investor_funds) = property_details.investor_funds.get(&signer).cloned()
                {
                    if let Some(paid_funds) = investor_funds.paid_funds.get(&asset).copied() {
                        // Transfer USDT funds to owner account
                        Self::transfer_funds(&property_account, &signer, paid_funds, asset)?;
                    }
                }
            }
            <T as pallet::Config>::LocalCurrency::transfer(
                property_details.asset_id,
                &signer,
                &property_account,
                token_amount.into(),
                Preservation::Expendable,
            )?;
            if refund_infos.refund_amount == 0 {
                T::PropertyToken::burn_property_token(property_details.asset_id)?;
                Self::refund_investors_with_fees(listing_id, refund_infos.property_lawyer_details)?;
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
            } else {
                RefundToken::<T>::insert(listing_id, refund_infos);
            }
            T::PropertyToken::remove_property_token_ownership(property_details.asset_id, &signer)?;
            Self::deposit_event(Event::<T>::RejectedFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the investor withdraw his funds after a property deal expired.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to withdraw from.
        ///
        /// Emits `ExpiredFundsWithdrawn` event when successful.
        #[pallet::call_index(11)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn withdraw_legal_process_expired(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let property_account = Self::property_account_id(property_details.asset_id);
            let token_amount = <T as pallet::Config>::PropertyToken::get_token_balance(
                property_details.asset_id,
                &signer,
            );
            ensure!(!token_amount.is_zero(), Error::<T>::NoPermission);

            let mut refund_infos = match RefundLegalExpired::<T>::get(listing_id) {
                Some(refund_infos) => refund_infos,
                None => {
                    let property_lawyer_details =
                        PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::TokenNotRefunded)?;
                    let current_block_number = <frame_system::Pallet<T>>::block_number();
                    ensure!(
                        property_lawyer_details.legal_process_expiry < current_block_number,
                        Error::<T>::LegalProcessOngoing
                    );
                    let real_estate_developer_lawyer_id = property_lawyer_details
                        .real_estate_developer_lawyer
                        .ok_or(Error::<T>::LawyerNotFound)?;
                    let spv_lawyer_id = property_lawyer_details
                        .spv_lawyer
                        .ok_or(Error::<T>::LawyerNotFound)?;
                    PalletRegions::<T>::decrement_active_cases(&real_estate_developer_lawyer_id)?;
                    PalletRegions::<T>::decrement_active_cases(&spv_lawyer_id)?;
                    PropertyLawyer::<T>::remove(listing_id);
                    RefundLegalExpired::<T>::insert(listing_id, property_details.token_amount);
                    property_details.token_amount
                }
            };

            refund_infos = refund_infos
                .checked_sub(token_amount)
                .ok_or(Error::<T>::NotEnoughTokenAvailable)?;

            for &asset in T::AcceptedAssets::get().iter() {
                if let Some(investor_funds) = property_details.investor_funds.get(&signer).cloned()
                {
                    if let Some(paid_funds) = investor_funds.paid_funds.get(&asset).copied() {
                        if let Some(paid_fee) = investor_funds.paid_fee.get(&asset).copied() {
                            let transfer_amount = paid_funds
                                .checked_add(&paid_fee)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                            Self::transfer_funds(
                                &property_account,
                                &signer,
                                transfer_amount,
                                asset,
                            )?;
                        } else {
                            Self::transfer_funds(&property_account, &signer, paid_funds, asset)?;
                        }
                    }
                }
            }
            <T as pallet::Config>::LocalCurrency::transfer(
                property_details.asset_id,
                &signer,
                &property_account,
                token_amount.into(),
                Preservation::Expendable,
            )?;
            if refund_infos == 0 {
                T::PropertyToken::burn_property_token(property_details.asset_id)?;
                T::PropertyToken::clear_token_owners(property_details.asset_id)?;
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
                RefundLegalExpired::<T>::remove(listing_id);
            } else {
                RefundLegalExpired::<T>::insert(listing_id, refund_infos);
            }
            T::PropertyToken::remove_property_token_ownership(property_details.asset_id, &signer)?;
            Self::deposit_event(Event::<T>::ExpiredFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the investor unfreeze his funds after a property listing expired.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        ///
        /// Emits `ExpiredFundsWithdrawn` event when successful.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_expired())]
        pub fn withdraw_expired(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            ensure!(
                property_details.listing_expiry < <frame_system::Pallet<T>>::block_number(),
                Error::<T>::ListingNotExpired
            );

            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_none(),
                Error::<T>::PropertyAlreadySold
            );

            let token_details =
                TokenOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::TokenOwnerNotFound)?;
            ensure!(
                !token_details.token_amount.is_zero(),
                Error::<T>::NoTokenBought,
            );

            // Process refunds for supported assets (USDT and USDC)
            Self::unfreeze_token(&mut property_details, &token_details, &signer)?;

            property_details.listed_token_amount = property_details
                .listed_token_amount
                .checked_add(token_details.token_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Check if all tokens are returned
            if property_details.listed_token_amount >= property_details.token_amount {
                // Listing is over, burn and clean everything
                T::PropertyToken::burn_property_token(property_details.asset_id)?;
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                let property_account = Self::property_account_id(property_details.asset_id);
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
            } else {
                OngoingObjectListing::<T>::insert(listing_id, &property_details);
            }
            Self::deposit_event(Event::<T>::ExpiredFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the real estate developer withdraw his deposit in case no token have been sold.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the caller wants to withdraw the deposit from.
        ///
        /// Emits `DeveloperDepositReturned` event when successful.
        #[pallet::call_index(13)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_deposit_unsold())]
        pub fn withdraw_deposit_unsold(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            ensure!(
                property_details.real_estate_developer == signer,
                Error::<T>::NoPermission
            );
            ensure!(
                property_details.listing_expiry < <frame_system::Pallet<T>>::block_number(),
                Error::<T>::ListingNotExpired
            );

            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_none(),
                Error::<T>::PropertyAlreadySold
            );
            // Check if all tokens are returned
            ensure!(
                property_details.listed_token_amount >= property_details.token_amount,
                Error::<T>::TokenNotReturned
            );
            // Listing is over, burn and clean everything
            T::PropertyToken::burn_property_token(property_details.asset_id)?;
            let (depositor, deposit_amount) =
                ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            <T as pallet::Config>::NativeCurrency::release(
                &HoldReason::ListingDepositReserve.into(),
                &depositor,
                deposit_amount,
                Precision::Exact,
            )?;
            let property_account = Self::property_account_id(property_details.asset_id);
            let native_balance = <T as pallet::Config>::NativeCurrency::balance(&property_account);
            if !native_balance.is_zero() {
                <T as pallet::Config>::NativeCurrency::transfer(
                    &property_account,
                    &property_details.real_estate_developer,
                    native_balance,
                    Preservation::Expendable,
                )?;
            }
            OngoingObjectListing::<T>::remove(listing_id);
            Self::deposit_event(Event::<T>::DeveloperDepositReturned {
                listing_id,
                developer: signer,
                amount: deposit_amount,
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
        /// Emits `ObjectUpdated` event when successful.
        #[pallet::call_index(14)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::upgrade_object())]
        pub fn upgrade_object(
            origin: OriginFor<T>,
            listing_id: ListingId,
            new_price: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateDeveloper,
            )?;
            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_none(),
                Error::<T>::PropertyAlreadySold
            );
            OngoingObjectListing::<T>::try_mutate(listing_id, |maybe_property_details| {
                let property_details = maybe_property_details
                    .as_mut()
                    .ok_or(Error::<T>::TokenNotForSale)?;
                ensure!(
                    property_details.listing_expiry > <frame_system::Pallet<T>>::block_number(),
                    Error::<T>::ListingExpired
                );
                ensure!(
                    property_details.real_estate_developer == signer,
                    Error::<T>::NoPermission
                );
                ensure!(
                    !property_details.listed_token_amount.is_zero(),
                    Error::<T>::PropertyAlreadySold
                );
                property_details.token_price = new_price;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::<T>::ObjectUpdated {
                listing_index: listing_id,
                new_price,
            });
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
        /// Emits `ListingDelisted` event when successful.
        #[pallet::call_index(15)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::delist_token())]
        pub fn delist_token(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let listing_details =
                TokenListings::<T>::take(listing_id).ok_or(Error::<T>::TokenNotForSale)?;
            ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
            let token_amount = listing_details.amount.into();
            let property_account = Self::property_account_id(listing_details.asset_id);
            <T as pallet::Config>::LocalCurrency::transfer(
                listing_details.asset_id,
                &property_account,
                &signer,
                token_amount,
                Preservation::Expendable,
            )?;
            Self::deposit_event(Event::<T>::ListingDelisted {
                listing_index: listing_id,
            });
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
        /// Emits `DeveloperLawyerProposed` event or `SpvLawyerProposed` event when successful.
        #[pallet::call_index(16)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::lawyer_claim_property())]
        pub fn lawyer_claim_property(
            origin: OriginFor<T>,
            listing_id: ListingId,
            legal_side: LegalProperty,
            costs: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::Lawyer,
            )?;
            let lawyer_region = pallet_regions::RealEstateLawyer::<T>::get(&signer)
                .ok_or(Error::<T>::NoPermission)?;
            let property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let asset_details =
                T::PropertyToken::get_property_asset_info(property_details.asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;
            ensure!(
                lawyer_region.region == asset_details.region,
                Error::<T>::WrongRegion
            );

            let [asset_id_usdc, asset_id_usdt] = T::AcceptedAssets::get();
            let collected_fee_usdt = property_details
                .collected_fees
                .get(&asset_id_usdt)
                .ok_or(Error::<T>::AssetNotSupported)?;
            let collected_fee_usdc = property_details
                .collected_fees
                .get(&asset_id_usdc)
                .ok_or(Error::<T>::AssetNotSupported)?;
            let collected_fees = collected_fee_usdt
                .checked_add(collected_fee_usdc)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            ensure!(collected_fees >= costs, Error::<T>::CostsTooHigh);
            match legal_side {
                LegalProperty::RealEstateDeveloperSide => {
                    ensure!(
                        !ProposedLawyers::<T>::contains_key(listing_id),
                        Error::<T>::LawyerProposalOngoing
                    );
                    ensure!(
                        property_lawyer_details
                            .real_estate_developer_lawyer
                            .is_none(),
                        Error::<T>::LawyerJobTaken
                    );
                    ensure!(
                        property_lawyer_details.spv_lawyer.as_ref() != Some(&signer),
                        Error::<T>::NoPermission
                    );
                    if let Some(proposal_id) = ListingSpvProposal::<T>::get(listing_id) {
                        if let Some(lawyer_proposal) = SpvLawyerProposal::<T>::get(proposal_id) {
                            ensure!(lawyer_proposal.lawyer != signer, Error::<T>::NoPermission);
                        }
                    }
                    ProposedLawyers::<T>::insert(
                        listing_id,
                        ProposedDeveloperLawyer {
                            lawyer: signer.clone(),
                            costs,
                        },
                    );
                    Self::deposit_event(Event::<T>::DeveloperLawyerProposed {
                        listing_id,
                        lawyer: signer,
                        proposed_cost: costs,
                    });
                }
                LegalProperty::SpvSide => {
                    T::PropertyToken::ensure_spv_created(property_details.asset_id)?;
                    ensure!(
                        !ListingSpvProposal::<T>::contains_key(listing_id),
                        Error::<T>::LawyerProposalOngoing
                    );
                    ensure!(
                        property_lawyer_details.spv_lawyer.is_none(),
                        Error::<T>::LawyerJobTaken
                    );
                    ensure!(
                        property_lawyer_details
                            .real_estate_developer_lawyer
                            .as_ref()
                            != Some(&signer),
                        Error::<T>::NoPermission
                    );
                    if let Some(lawyer_proposal) = ProposedLawyers::<T>::get(listing_id) {
                        ensure!(lawyer_proposal.lawyer != signer, Error::<T>::NoPermission);
                    }
                    let proposal_id = ProposalCounter::<T>::get();
                    let current_block_number = <frame_system::Pallet<T>>::block_number();
                    let expiry_block =
                        current_block_number.saturating_add(T::LawyerVotingTime::get());
                    ListingSpvProposal::<T>::insert(listing_id, proposal_id);
                    SpvLawyerProposal::<T>::insert(
                        proposal_id,
                        ProposedSpvLawyer {
                            lawyer: signer.clone(),
                            asset_id: property_details.asset_id,
                            costs,
                            expiry_block,
                        },
                    );
                    OngoingLawyerVoting::<T>::insert(
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
                    Self::deposit_event(Event::<T>::SpvLawyerProposed {
                        listing_id,
                        lawyer: signer,
                        proposed_cost: costs,
                        expiry_block,
                    });
                }
            }
            Ok(())
        }

        /// Lets token buyer vote for a lawyer to represent the spv.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount of property token that the investor is using for voting.
        ///
        /// Emits `VotedOnLawyer` event when successful.
        #[pallet::call_index(17)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_spv_lawyer())]
        pub fn vote_on_spv_lawyer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            vote: Vote,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            let proposal_id =
                ListingSpvProposal::<T>::get(listing_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let proposal_details =
                SpvLawyerProposal::<T>::get(proposal_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                proposal_details.expiry_block > current_block_number,
                Error::<T>::VotingExpired
            );
            let voting_power =
                T::PropertyToken::get_token_balance(proposal_details.asset_id, &signer);
            ensure!(voting_power >= amount, Error::<T>::NotEnoughToken);

            let mut new_yes_power = 0u32;
            let mut new_no_power = 0u32;

            OngoingLawyerVoting::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote
                    .as_mut()
                    .ok_or(Error::<T>::NoLawyerProposed)?;

                UserLawyerVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    if let Some(previous_vote) = maybe_vote_record.take() {
                        T::AssetsFreezer::decrease_frozen(
                            proposal_details.asset_id,
                            &MarketplaceFreezeReason::SpvLawyerVoting,
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
                        proposal_details.asset_id,
                        &MarketplaceFreezeReason::SpvLawyerVoting,
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
                        asset_id: proposal_details.asset_id,
                        power: amount,
                    });

                    new_yes_power = current_vote.yes_voting_power;
                    new_no_power = current_vote.no_voting_power;

                    Ok::<(), DispatchError>(())
                })?;

                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnLawyer {
                listing_id,
                voter: signer,
                vote,
                voting_power,
                new_yes_power,
                new_no_power,
                proposal_id,
            });
            Ok(())
        }

        /// Lets the real estate developer approve or reject a lawyer.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        /// - `approve`: Approves or rejects the lawyer.
        ///
        /// Emits `RealEstateLawyerProposalFinalized` event when successful.
        #[pallet::call_index(18)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::approve_developer_lawyer())]
        pub fn approve_developer_lawyer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            approve: bool,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateDeveloper,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            ensure!(
                signer == property_details.real_estate_developer,
                Error::<T>::NoPermission
            );

            let mut property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let proposal =
                ProposedLawyers::<T>::get(listing_id).ok_or(Error::<T>::NoLawyerProposed)?;

            if approve {
                property_lawyer_details.real_estate_developer_lawyer =
                    Some(proposal.lawyer.clone());
                let [asset_id_usdc, asset_id_usdt] = T::AcceptedAssets::get();
                let collected_fee_usdt = property_details
                    .collected_fees
                    .get(&asset_id_usdt)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let collected_fee_usdc = property_details
                    .collected_fees
                    .get(&asset_id_usdc)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let collected_fees = collected_fee_usdt
                    .checked_add(&collected_fee_usdc)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                ensure!(collected_fees >= proposal.costs, Error::<T>::CostsTooHigh);

                let (usdc_cost, usdt_cost) = Self::allocate_fees(
                    &mut property_lawyer_details.real_estate_developer_lawyer_costs,
                    asset_id_usdt,
                    collected_fee_usdt,
                    asset_id_usdc,
                    collected_fee_usdc,
                    proposal.costs,
                )?;
                Self::deposit_event(Event::<T>::LawyerCostsAllocated {
                    listing_id,
                    lawyer_account: proposal.lawyer.clone(),
                    lawyer_type: LegalProperty::RealEstateDeveloperSide,
                    cost_in_usdc: usdc_cost,
                    cost_in_usdt: usdt_cost,
                });
                PalletRegions::<T>::increment_active_cases(&proposal.lawyer)?;
                PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
            }
            ProposedLawyers::<T>::remove(listing_id);
            Self::deposit_event(Event::RealEstateLawyerProposalFinalized {
                listing_id,
                lawyer: proposal.lawyer,
                is_approved: approve,
            });
            Ok(())
        }

        /// Finalizes the spv lawyer voting.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        ///
        /// Emits `SpvLawyerVoteFinalized` event when successful.
        #[pallet::call_index(19)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_spv_lawyer())]
        pub fn finalize_spv_lawyer(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let _ = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;

            let proposal_id =
                ListingSpvProposal::<T>::get(listing_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let proposal =
                SpvLawyerProposal::<T>::get(proposal_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                proposal.expiry_block <= current_block_number,
                Error::<T>::VotingStillOngoing
            );

            let voting_result =
                OngoingLawyerVoting::<T>::get(proposal_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let mut property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let asset_details = <T as pallet::Config>::PropertyToken::get_property_asset_info(
                property_details.asset_id,
            )
            .ok_or(Error::<T>::NoObjectFound)?;
            let total_votes = voting_result
                .yes_voting_power
                .saturating_add(voting_result.no_voting_power);
            let total_supply = asset_details.token_amount;

            ensure!(total_supply > Zero::zero(), Error::<T>::NoObjectFound);

            let quorum_percent: u32 = T::MinVotingQuorum::get().deconstruct().into();

            let meets_quorum =
                total_votes.saturating_mul(100u32) > total_supply.saturating_mul(quorum_percent);
            let is_approved =
                voting_result.yes_voting_power > voting_result.no_voting_power && meets_quorum;
            if is_approved {
                property_lawyer_details.spv_lawyer = Some(proposal.lawyer.clone());
                let [asset_id_usdc, asset_id_usdt] = T::AcceptedAssets::get();

                let collected_fee_usdt = property_details
                    .collected_fees
                    .get(&asset_id_usdt)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let collected_fee_usdc = property_details
                    .collected_fees
                    .get(&asset_id_usdc)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let collected_fees = collected_fee_usdt
                    .checked_add(&collected_fee_usdc)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                ensure!(collected_fees >= proposal.costs, Error::<T>::CostsTooHigh);

                let (usdc_cost, usdt_cost) = Self::allocate_fees(
                    &mut property_lawyer_details.spv_lawyer_costs,
                    asset_id_usdt,
                    collected_fee_usdt,
                    asset_id_usdc,
                    collected_fee_usdc,
                    proposal.costs,
                )?;
                Self::deposit_event(Event::<T>::LawyerCostsAllocated {
                    listing_id,
                    lawyer_account: proposal.lawyer.clone(),
                    lawyer_type: LegalProperty::SpvSide,
                    cost_in_usdc: usdc_cost,
                    cost_in_usdt: usdt_cost,
                });
                PalletRegions::<T>::increment_active_cases(&proposal.lawyer)?;
                PropertyLawyer::<T>::insert(listing_id, property_lawyer_details.clone());
            }
            SpvLawyerProposal::<T>::remove(proposal_id);
            OngoingLawyerVoting::<T>::remove(proposal_id);
            ListingSpvProposal::<T>::remove(listing_id);

            Self::deposit_event(Event::SpvLawyerVoteFinalized {
                listing_id,
                lawyer: proposal.lawyer,
                is_approved,
                final_yes_power: voting_result.yes_voting_power,
                final_no_power: voting_result.no_voting_power,
            });
            Ok(())
        }

        /// Lets a voter unlock his locked token after voting on a spv lawyer.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the spv lawyer proposal.
        ///
        /// Emits `TokenUnfrozen` event when successful.
        #[pallet::call_index(20)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn unfreeze_spv_lawyer_token(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record =
                UserLawyerVote::<T>::get(proposal_id, &signer).ok_or(Error::<T>::NoFrozenAmount)?;

            if let Some(proposal) = SpvLawyerProposal::<T>::get(proposal_id) {
                let current_block_number = frame_system::Pallet::<T>::block_number();
                ensure!(
                    proposal.expiry_block <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            T::AssetsFreezer::decrease_frozen(
                vote_record.asset_id,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &signer,
                vote_record.power.into(),
            )?;

            UserLawyerVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::TokenUnfrozen {
                proposal_id,
                asset_id: vote_record.asset_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets a lawyer step back from a case.
        ///
        /// The origin must be Signed and the sender must have sufficient funds free.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        ///
        /// Emits `LawyerRemovedFromCase` event when successful.
        #[pallet::call_index(21)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::remove_from_case())]
        pub fn remove_lawyer_claim(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::Lawyer,
            )?;
            ensure!(
                pallet_regions::RealEstateLawyer::<T>::get(&signer).is_some(),
                Error::<T>::NoPermission
            );
            let mut property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            if property_lawyer_details
                .real_estate_developer_lawyer
                .as_ref()
                == Some(&signer)
            {
                ensure!(
                    property_lawyer_details.real_estate_developer_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                property_lawyer_details.real_estate_developer_lawyer = None;
            } else if property_lawyer_details.spv_lawyer.as_ref() == Some(&signer) {
                ensure!(
                    property_lawyer_details.spv_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                property_lawyer_details.spv_lawyer = None;
            } else {
                return Err(Error::<T>::NoPermission.into());
            }
            PalletRegions::<T>::decrement_active_cases(&signer)?;
            PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
            Self::deposit_event(Event::<T>::LawyerRemovedFromCase {
                lawyer: signer,
                listing_id,
            });
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
        /// Emits `DocumentsConfirmed` event when successful.
        #[pallet::call_index(22)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::lawyer_confirm_documents(
            <T as pallet::Config>::MaxPropertyToken::get(),
        ))]
        pub fn lawyer_confirm_documents(
            origin: OriginFor<T>,
            listing_id: ListingId,
            approve: bool,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::Lawyer,
            )?;
            let mut property_lawyer_details =
                PropertyLawyer::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                property_lawyer_details.legal_process_expiry >= current_block_number,
                Error::<T>::LegalProcessFailed
            );
            if property_lawyer_details
                .real_estate_developer_lawyer
                .as_ref()
                == Some(&signer)
            {
                ensure!(
                    property_lawyer_details.real_estate_developer_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                property_lawyer_details.real_estate_developer_status = if approve {
                    DocumentStatus::Approved
                } else {
                    DocumentStatus::Rejected
                };
                Self::deposit_event(Event::<T>::DocumentsConfirmed {
                    signer,
                    listing_id,
                    legal_side: LegalProperty::RealEstateDeveloperSide,
                    approve,
                });
            } else if property_lawyer_details.spv_lawyer.as_ref() == Some(&signer) {
                ensure!(
                    property_lawyer_details.spv_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                property_lawyer_details.spv_status = if approve {
                    DocumentStatus::Approved
                } else {
                    DocumentStatus::Rejected
                };
                Self::deposit_event(Event::<T>::DocumentsConfirmed {
                    signer,
                    listing_id,
                    legal_side: LegalProperty::SpvSide,
                    approve,
                });
            } else {
                return Err(Error::<T>::NoPermission.into());
            }

            let developer_status = property_lawyer_details.real_estate_developer_status.clone();
            let spv_status = property_lawyer_details.spv_status.clone();

            match (developer_status, spv_status) {
                (DocumentStatus::Approved, DocumentStatus::Approved) => {
                    Self::execute_deal(listing_id, property_lawyer_details.clone())?;
                }
                (DocumentStatus::Rejected, DocumentStatus::Rejected) => {
                    Self::reject_and_refund(listing_id, &property_lawyer_details)?;
                }
                (DocumentStatus::Approved, DocumentStatus::Rejected) => {
                    if !property_lawyer_details.second_attempt {
                        property_lawyer_details.spv_status = DocumentStatus::Pending;
                        property_lawyer_details.real_estate_developer_status =
                            DocumentStatus::Pending;
                        property_lawyer_details.second_attempt = true;
                        PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                    } else {
                        Self::reject_and_refund(listing_id, &property_lawyer_details)?;
                    }
                }
                (DocumentStatus::Rejected, DocumentStatus::Approved) => {
                    if !property_lawyer_details.second_attempt {
                        property_lawyer_details.spv_status = DocumentStatus::Pending;
                        property_lawyer_details.real_estate_developer_status =
                            DocumentStatus::Pending;
                        property_lawyer_details.second_attempt = true;
                        PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                    } else {
                        Self::reject_and_refund(listing_id, &property_lawyer_details)?;
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
        /// Emits `DocumentsConfirmed` event when successful.
        #[pallet::call_index(23)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::send_property_token())]
        pub fn send_property_token(
            origin: OriginFor<T>,
            asset_id: u32,
            receiver: AccountIdOf<T>,
            token_amount: u32,
        ) -> DispatchResult {
            let sender = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &pallet_xcavate_whitelist::Role::RealEstateInvestor,
            )?;
            ensure!(
                <T as pallet::Config>::Whitelist::has_role(
                    &receiver,
                    pallet_xcavate_whitelist::Role::RealEstateInvestor
                ),
                Error::<T>::UserNotWhitelisted
            );
            T::PropertyToken::transfer_property_token(
                asset_id,
                &sender,
                &sender,
                &receiver,
                token_amount,
            )?;

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
            <T as pallet::Config>::TreasuryId::get().into_account_truncating()
        }

        pub fn next_listing_id(listing_id: ListingId) -> Result<ListingId, Error<T>> {
            listing_id
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)
        }

        /// Sends the token to the new owners and the funds to the real estate developer once all 100 token
        /// of a collection are sold.
        fn execute_deal(
            listing_id: u32,
            property_lawyer_details: PropertyLawyerDetails<T>,
        ) -> DispatchResult {
            let property_details =
                OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let asset_details =
                T::PropertyToken::get_property_asset_info(property_details.asset_id)
                    .ok_or(Error::<T>::InvalidIndex)?;
            let treasury_id = Self::treasury_account_id();
            let property_account = Self::property_account_id(property_details.asset_id);
            let region = pallet_regions::RegionDetails::<T>::get(asset_details.region)
                .ok_or(Error::<T>::RegionUnknown)?;

            // Get lawyer accounts
            let real_estate_developer_lawyer_id = property_lawyer_details
                .real_estate_developer_lawyer
                .ok_or(Error::<T>::LawyerNotFound)?;
            let spv_lawyer_id = property_lawyer_details
                .spv_lawyer
                .ok_or(Error::<T>::LawyerNotFound)?;
            PalletRegions::<T>::decrement_active_cases(&real_estate_developer_lawyer_id)?;
            PalletRegions::<T>::decrement_active_cases(&spv_lawyer_id)?;

            let mut developer_payout = Payout::default();
            let mut spv_lawyer_payout = Payout::default();
            let mut treasury_payout = Payout::default();
            let mut region_owner_payout = Payout::default();

            // Distribute funds from property account for each asset
            for &asset in T::AcceptedAssets::get().iter() {
                // Get total collected amounts and lawyer costs
                let total_collected_funds = property_details
                    .collected_funds
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let spv_lawyer_costs = property_lawyer_details
                    .spv_lawyer_costs
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let tax = property_details
                    .collected_tax
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let collected_fees = property_details
                    .collected_fees
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;

                let fee_percentage = T::MarketplaceFeePercentage::get();
                ensure!(
                    fee_percentage <= 100u128.into(),
                    Error::<T>::InvalidFeePercentage
                );

                let developer_percentage = <T as pallet::Config>::Balance::from(100u128)
                    .checked_sub(&fee_percentage)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;

                // Calculate amounts to distribute
                let mut developer_amount = total_collected_funds
                    .checked_mul(&developer_percentage)
                    .ok_or(Error::<T>::MultiplyError)?
                    .checked_div(&(100u128.into()))
                    .ok_or(Error::<T>::DivisionError)?;

                let (developer_lawyer_tax, spv_lawyer_tax) = if property_details.tax_paid_by_developer {
                    developer_amount = developer_amount
                        .checked_sub(&tax)
                        .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    (tax, Zero::zero())
                } else {
                    (Zero::zero(), tax)
                };

                let spv_lawyer_amount = spv_lawyer_costs
                    .checked_add(&spv_lawyer_tax)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                let protocol_fees = total_collected_funds
                    .checked_div(&(100u128.into()))
                    .ok_or(Error::<T>::DivisionError)?
                    .checked_add(&collected_fees)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
                    .saturating_sub(spv_lawyer_costs);

                let region_owner_amount = protocol_fees
                    .checked_div(&(2u128.into()))
                    .ok_or(Error::<T>::DivisionError)?;
                let treasury_amount = protocol_fees.saturating_sub(region_owner_amount);

                // Transfer funds from property account
                Self::transfer_funds(
                    &property_account,
                    &property_details.real_estate_developer,
                    developer_amount,
                    asset,
                )?;
                if !developer_lawyer_tax.is_zero() {
                    Self::transfer_funds(
                        &property_account,
                        &real_estate_developer_lawyer_id,
                        developer_lawyer_tax,
                        asset,
                    )?;
                }
                Self::transfer_funds(&property_account, &spv_lawyer_id, spv_lawyer_amount, asset)?;
                Self::transfer_funds(&property_account, &treasury_id, treasury_amount, asset)?;
                Self::transfer_funds(&property_account, &region.owner, region_owner_amount, asset)?;

                match asset {
                    1337 => {
                        // USDC ID
                        developer_payout.amount_in_usdc = developer_amount;
                        spv_lawyer_payout.amount_in_usdc = spv_lawyer_costs;
                        treasury_payout.amount_in_usdc = treasury_amount;
                        region_owner_payout.amount_in_usdc = region_owner_amount;
                    }
                    1984 => {
                        // USDT ID
                        developer_payout.amount_in_usdt = developer_amount;
                        spv_lawyer_payout.amount_in_usdt = spv_lawyer_costs;
                        treasury_payout.amount_in_usdt = treasury_amount;
                        region_owner_payout.amount_in_usdt = region_owner_amount;
                    }
                    _ => {}
                }
            }
            T::PropertyToken::finalize_property(property_details.asset_id)?;
            // Release deposit
            if let Some((depositor, deposit_amount)) = ListingDeposits::<T>::take(listing_id) {
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
            }

            let payouts = FinalSettlementPayouts {
                developer_payout,
                developer_account: property_details.real_estate_developer.clone(),
                spv_lawyer_payout,
                spv_lawyer_account: spv_lawyer_id.clone(),
                treasury_payout,
                treasury_account: treasury_id.clone(),
                region_owner_payout,
                region_owner_account: region.owner.clone(),
            };

            Self::deposit_event(Event::<T>::PrimarySaleCompleted {
                listing_id,
                asset_id: property_details.asset_id,
                payouts,
            });
            Ok(())
        }

        fn reject_and_refund(
            listing_id: u32,
            property_lawyer_details: &PropertyLawyerDetails<T>,
        ) -> DispatchResult {
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let real_estate_developer_lawyer_id = property_lawyer_details
                .real_estate_developer_lawyer
                .clone()
                .ok_or(Error::<T>::LawyerNotFound)?;
            let spv_lawyer_id = property_lawyer_details
                .spv_lawyer
                .clone()
                .ok_or(Error::<T>::LawyerNotFound)?;
            PalletRegions::<T>::decrement_active_cases(&real_estate_developer_lawyer_id)?;
            PalletRegions::<T>::decrement_active_cases(&spv_lawyer_id)?;
            RefundToken::<T>::insert(
                listing_id,
                RefundInfos {
                    refund_amount: property_details.token_amount,
                    property_lawyer_details: property_lawyer_details.clone(),
                },
            );
            Ok(())
        }

        fn refund_investors_with_fees(
            listing_id: ListingId,
            property_lawyer_details: PropertyLawyerDetails<T>,
        ) -> DispatchResult {
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let property_account = Self::property_account_id(property_details.asset_id);
            let treasury_id = Self::treasury_account_id();
            let spv_lawyer_id = property_lawyer_details
                .spv_lawyer
                .ok_or(Error::<T>::LawyerNotFound)?;

            // Process fees and transfers for each asset
            for asset in T::AcceptedAssets::get().iter() {
                // Fetch fees and lawyer costs
                let fees = property_details
                    .collected_fees
                    .get(asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let lawyer_costs = property_lawyer_details
                    .spv_lawyer_costs
                    .get(asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;

                // Calculate treasury amount
                let treasury_amount = fees
                    .checked_sub(&lawyer_costs)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;

                // Perform fund transfers
                Self::transfer_funds(&property_account, &treasury_id, treasury_amount, *asset)?;
                Self::transfer_funds(&property_account, &spv_lawyer_id, lawyer_costs, *asset)?;
            }
            T::PropertyToken::clear_token_owners(property_details.asset_id)?;
            Ok(())
        }

        fn buying_token_process(
            listing_id: u32,
            transfer_from: &AccountIdOf<T>,
            account: &AccountIdOf<T>,
            mut listing_details: ListingDetailsType<T>,
            price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            Self::calculate_fees(price, transfer_from, &listing_details.seller, payment_asset)?;
            let property_account = Self::property_account_id(listing_details.asset_id);
            T::PropertyToken::transfer_property_token(
                listing_details.asset_id,
                &listing_details.seller,
                &property_account,
                account,
                amount,
            )?;
            listing_details.amount = listing_details
                .amount
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            if listing_details.amount > 0 {
                TokenListings::<T>::insert(listing_id, listing_details.clone());
            }
            Self::deposit_event(Event::<T>::RelistedTokenBought {
                listing_index: listing_id,
                asset_id: listing_details.asset_id,
                buyer: account.clone(),
                seller: listing_details.seller,
                price: listing_details.token_price,
                amount,
                payment_asset,
                new_amount_remaining: listing_details.amount,
            });
            Ok(())
        }

        fn unfreeze_token(
            property_details: &mut PropertyListingDetailsType<T>,
            token_details: &TokenOwnerDetails<T>,
            signer: &AccountIdOf<T>,
        ) -> DispatchResult {
            for asset in T::AcceptedAssets::get().iter() {
                if let Some(paid_funds) = token_details.paid_funds.get(asset).copied() {
                    if paid_funds.is_zero() {
                        continue;
                    }

                    let default = Default::default();
                    let paid_tax = token_details
                        .paid_tax
                        .get(asset)
                        .copied()
                        .unwrap_or(default);

                    // Calculate refund and investor fee (1% of paid funds)
                    let refund_amount = paid_funds
                        .checked_add(&paid_tax)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    let investor_fee = paid_funds
                        .checked_div(&(100u128.into()))
                        .ok_or(Error::<T>::DivisionError)?;
                    let total_investor_amount = refund_amount
                        .checked_add(&investor_fee)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;

                    // Release funds
                    T::ForeignAssetsHolder::release(
                        *asset,
                        &MarketplaceHoldReason::Marketplace,
                        signer,
                        total_investor_amount,
                        Precision::Exact,
                    )?;
                    if let Some(funds) = property_details.collected_funds.get_mut(asset) {
                        *funds = funds
                            .checked_sub(&paid_funds)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    }
                    if let Some(tax) = property_details.collected_tax.get_mut(asset) {
                        *tax = tax
                            .checked_sub(&paid_tax)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    }
                    if let Some(fee) = property_details.collected_fees.get_mut(asset) {
                        *fee = fee
                            .checked_sub(&investor_fee)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    }
                }
            }
            Ok(())
        }

        fn unfreeze_token_with_refunds(
            property_details: &mut PropertyListingDetailsType<T>,
            token_details: &TokenOwnerDetails<T>,
            signer: &AccountIdOf<T>,
        ) -> Result<
            (
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::Balance,
            ),
            DispatchError,
        > {
            let mut principal_usdc = Default::default();
            let mut tax_usdc = Default::default();
            let mut principal_usdt = Default::default();
            let mut tax_usdt = Default::default();

            for asset in T::AcceptedAssets::get().iter() {
                if let Some(paid_funds) = token_details.paid_funds.get(asset).copied() {
                    if paid_funds.is_zero() {
                        continue;
                    }

                    let default = Default::default();
                    let paid_tax = token_details
                        .paid_tax
                        .get(asset)
                        .copied()
                        .unwrap_or(default);

                    // Calculate refund and investor fee (1% of paid funds)
                    let refund_amount = paid_funds
                        .checked_add(&paid_tax)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    let investor_fee = paid_funds
                        .checked_div(&(100u128.into()))
                        .ok_or(Error::<T>::DivisionError)?;
                    let total_investor_amount = refund_amount
                        .checked_add(&investor_fee)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;

                    // Release funds
                    T::ForeignAssetsHolder::release(
                        *asset,
                        &MarketplaceHoldReason::Marketplace,
                        signer,
                        total_investor_amount,
                        Precision::Exact,
                    )?;
                    if let Some(funds) = property_details.collected_funds.get_mut(asset) {
                        *funds = funds
                            .checked_sub(&paid_funds)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    }
                    if let Some(tax) = property_details.collected_tax.get_mut(asset) {
                        *tax = tax
                            .checked_sub(&paid_tax)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    }
                    if let Some(fee) = property_details.collected_fees.get_mut(asset) {
                        *fee = fee
                            .checked_sub(&investor_fee)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                    }

                    if *asset == 1337u32 {
                        principal_usdc = paid_funds;
                        tax_usdc = paid_tax;
                    } else if *asset == 1984u32 {
                        principal_usdt = paid_funds;
                        tax_usdt = paid_tax;
                    }
                }
            }
            Ok((principal_usdc, tax_usdc, principal_usdt, tax_usdt))
        }

        fn calculate_fees(
            price: <T as pallet::Config>::Balance,
            sender: &AccountIdOf<T>,
            receiver: &AccountIdOf<T>,
            asset: u32,
        ) -> DispatchResult {
            let fee_percent = T::MarketplaceFeePercentage::get();
            ensure!(
                fee_percent < 100u128.into(),
                Error::<T>::InvalidFeePercentage
            );

            let fees = price
                .checked_mul(&fee_percent)
                .ok_or(Error::<T>::MultiplyError)?
                .checked_div(&(100u128.into()))
                .ok_or(Error::<T>::DivisionError)?;
            let treasury_id = Self::treasury_account_id();
            let seller_part = price
                .checked_sub(&fees)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            Self::transfer_funds(sender, &treasury_id, fees, asset)?;
            Self::transfer_funds(sender, receiver, seller_part, asset)?;
            Ok(())
        }

        fn transfer_funds(
            from: &AccountIdOf<T>,
            to: &AccountIdOf<T>,
            amount: <T as pallet::Config>::Balance,
            asset: u32,
        ) -> DispatchResult {
            if !amount.is_zero() {
                T::ForeignCurrency::transfer(asset, from, to, amount, Preservation::Expendable)
                    .map_err(|_| Error::<T>::NotEnoughFunds)?;
            }
            Ok(())
        }

        fn create_initial_funds() -> Result<
            BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            DispatchError,
        > {
            let mut map = BoundedBTreeMap::default();
            for &asset in T::AcceptedAssets::get().iter() {
                map.try_insert(asset, Default::default())
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }
            Ok(map)
        }

        fn update_map(
            map: &mut BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            asset: u32,
            value: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            if let Some(existing) = map.get_mut(&asset) {
                *existing = existing
                    .checked_add(&value)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
            } else {
                map.try_insert(asset, value)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }
            Ok(())
        }

        fn allocate_fees(
            costs_map: &mut BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            asset_id_usdt: u32,
            collected_fee_usdt: <T as pallet::Config>::Balance,
            asset_id_usdc: u32,
            collected_fee_usdc: <T as pallet::Config>::Balance,
            costs: <T as pallet::Config>::Balance,
        ) -> Result<
            (
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::Balance,
            ),
            DispatchError,
        > {
            let mut usdt_cost = 0u32.into();
            let mut usdc_cost = 0u32.into();

            if collected_fee_usdt >= costs {
                costs_map
                    .try_insert(asset_id_usdt, costs)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                usdt_cost = costs;
            } else if collected_fee_usdc >= costs {
                costs_map
                    .try_insert(asset_id_usdc, costs)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                usdc_cost = costs;
            } else {
                let remaining_costs = costs
                    .checked_sub(&collected_fee_usdt)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                ensure!(
                    collected_fee_usdc >= remaining_costs,
                    Error::<T>::CostsTooHigh
                );
                costs_map
                    .try_insert(asset_id_usdt, collected_fee_usdt)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                costs_map
                    .try_insert(asset_id_usdc, remaining_costs)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                usdt_cost = collected_fee_usdt;
                usdc_cost = remaining_costs;
            }
            Ok((usdc_cost, usdt_cost))
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
