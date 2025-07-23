//! Benchmarking setup for pallet-property-governance
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as PropertyGovernance;
use frame_benchmarking::v2::*;
use frame_support::sp_runtime::{Permill, Saturating};
use frame_support::traits::{fungible::Mutate, Hooks};
use frame_support::BoundedVec;
use frame_support::{assert_ok, traits::Get};
use frame_system::{Pallet as System, RawOrigin};
use pallet_marketplace::types::LegalProperty;
use pallet_marketplace::Pallet as Marketplace;
use pallet_property_management::Pallet as PropertyManagement;
use pallet_regions::Pallet as Regions;
use pallet_regions::{RegionIdentifier, Vote};
use pallet_xcavate_whitelist::Pallet as Whitelist;
use scale_info::prelude::vec;

pub trait Config: pallet_marketplace::Config + crate::Config {}

impl<T: crate::Config + pallet_marketplace::Config> Config for T {}

fn create_whitelisted_user<T: Config>() -> T::AccountId {
    let signer: T::AccountId = account("signer", 0, 0);
    assert_ok!(Whitelist::<T>::add_to_whitelist(
        RawOrigin::Root.into(),
        signer.clone()
    ));
    signer
}

fn create_a_new_region<T: Config>(signer: T::AccountId) -> (u16, LocationId<T>) {
    let region = RegionIdentifier::France;
    let region_id = region.clone().into_u16();

    let deposit = T::RegionProposalDeposit::get();
    let auction_amount = T::MinimumRegionDeposit::get();
    let total_funds = deposit
        .saturating_mul(1000u32.into())
        .saturating_add(auction_amount.saturating_mul(100u32.into()));
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(
        &signer,
        total_funds
    ));

    assert_ok!(Regions::<T>::add_regional_operator(
        RawOrigin::Root.into(),
        signer.clone()
    ));
    assert_ok!(Regions::<T>::propose_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region.clone()
    ));
    assert_ok!(Regions::<T>::vote_on_region_proposal(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        Vote::Yes
    ));

    let bid_amount = auction_amount.saturating_mul(10u32.into());

    let expiry = System::<T>::block_number() + T::RegionVotingTime::get();
    System::<T>::set_block_number(expiry);

    assert_ok!(Regions::<T>::bid_on_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        bid_amount
    ));

    let auction_expiry = System::<T>::block_number() + T::RegionAuctionTime::get();
    System::<T>::set_block_number(auction_expiry);
    assert_ok!(Regions::<T>::create_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        T::MaxListingDuration::get(),
        Permill::from_percent(5)
    ));

    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();
    assert_ok!(Regions::<T>::create_new_location(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        location.clone()
    ));

    // Verify region and location
    assert!(pallet_regions::RegionDetails::<T>::contains_key(region_id));
    assert!(pallet_regions::LocationRegistration::<T>::contains_key(
        region_id, &location
    ));

    (region_id, location)
}

fn list_and_sell_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
) -> T::AccountId {
    let token_amount: u32 = <T as pallet_marketplace::Config>::MaxPropertyToken::get();
    let token_price: <T as pallet_marketplace::Config>::Balance = 1_000u32.into();
    let property_price = token_price.saturating_mul((token_amount as u128).into());
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
    assert_ok!(
        <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        )
    );

    let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
        BoundedVec::truncate_from(vec![
            42u8;
            <T as pallet_nfts::Config>::StringLimit::get() as usize
        ]);

    let tax_paid_by_developer = true;
    assert_ok!(Marketplace::<T>::list_object(
        RawOrigin::Signed(seller).into(),
        region_id,
        location,
        token_price,
        token_amount,
        metadata,
        tax_paid_by_developer,
    ));
    let listing_id = 0;
    let payment_asset = <T as pallet_marketplace::Config>::AcceptedAssets::get()[0];
    let buyer: T::AccountId = account("buyer", 0, 0);
    assert_ok!(
        <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        )
    );
    assert_ok!(
        <T as pallet_marketplace::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        )
    );
    assert_ok!(Whitelist::<T>::add_to_whitelist(
        RawOrigin::Root.into(),
        buyer.clone()
    ));
    add_buyers_to_listing::<T>(token_amount - 1, payment_asset, property_price);

    assert_ok!(Marketplace::<T>::buy_property_token(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
        1,
        payment_asset,
    ));
    buyer
}

fn create_registered_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
) -> T::AccountId {
    let token_owner = list_and_sell_property::<T>(seller.clone(), region_id, location);
    let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
    let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        region_id,
        lawyer_1.clone()
    ));
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        region_id,
        lawyer_2.clone()
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        0,
        LegalProperty::RealEstateDeveloperSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::approve_developer_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        0,
        LegalProperty::SpvSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
        RawOrigin::Signed(token_owner.clone()).into(),
        0,
        pallet_marketplace::types::Vote::Yes
    ));
    let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);
    assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
        RawOrigin::Signed(token_owner.clone()).into(),
        0,
    ));

    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_1).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_2).into(),
        0,
        true
    ));
    token_owner
}

fn add_buyers_to_listing<T: Config + pallet_marketplace::Config>(
    buyers: u32,
    payment_asset: u32,
    property_price: <T as pallet_marketplace::Config>::Balance,
) {
    let deposit_amount = property_price
        .saturating_mul(<T as pallet_marketplace::Config>::ListingDeposit::get())
        / 100u128.into();

    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        let payment_asset_buyers = <T as pallet_marketplace::Config>::AcceptedAssets::get()[0];
        assert_ok!(
            <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
                &buyer,
                deposit_amount.saturating_mul(20u32.into())
            )
        );
        assert_ok!(
            <T as pallet_marketplace::Config>::ForeignCurrency::mint_into(
                payment_asset,
                &buyer,
                property_price
            )
        );
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));
        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(buyer).into(),
            0,
            1,
            payment_asset_buyers
        ));
    }
}

fn set_letting_agent<T: Config>(
    region_owner: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
    asset_id: u32,
) -> T::AccountId {
    let letting_agent: T::AccountId = account("letting_agent", 0, 0);
    assert_ok!(Whitelist::<T>::add_to_whitelist(
        RawOrigin::Root.into(),
        letting_agent.clone()
    ));
    let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
    assert_ok!(
        <T as pallet_property_management::Config>::NativeCurrency::mint_into(
            &letting_agent,
            deposit
        )
    );

    assert_ok!(PropertyManagement::<T>::add_letting_agent(
        RawOrigin::Signed(region_owner).into(),
        region_id,
        location,
        letting_agent.clone()
    ));
    assert_ok!(PropertyManagement::<T>::letting_agent_deposit(
        RawOrigin::Signed(letting_agent.clone()).into()
    ));
    assert_ok!(PropertyManagement::<T>::set_letting_agent(
        RawOrigin::Signed(letting_agent.clone()).into(),
        asset_id
    ));
    letting_agent
}

fn run_to_block<T: Config>(new_block: frame_system::pallet_prelude::BlockNumberFor<T>) {
    while System::<T>::block_number() < new_block {
        if System::<T>::block_number() > 0u32.into() {
            PropertyGovernance::<T>::on_initialize(System::<T>::block_number());
            System::<T>::on_finalize(System::<T>::block_number());
        }
        System::<T>::reset_events();
        System::<T>::set_block_number(System::<T>::block_number() + 1u32.into());
        System::<T>::on_initialize(System::<T>::block_number());
        PropertyGovernance::<T>::on_initialize(System::<T>::block_number());
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn propose() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let _ = create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let letting_agent =
            set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let expiry_block = <System<T>>::block_number().saturating_add(T::VotingTime::get());
        let mut proposals = BoundedVec::default();
        for i in 1..T::MaxVotesForBlock::get() {
            proposals.try_push(i).unwrap();
        }
        ProposalRoundsExpiring::<T>::insert(expiry_block, proposals);

        let data = BoundedVec::try_from("Proposal".as_bytes().to_vec()).unwrap();
        let asset_id = 0;

        let proposal_amount = T::LowProposal::get().saturating_mul(2_u32.into());

        #[extrinsic_call]
        propose(
            RawOrigin::Signed(letting_agent.clone()),
            asset_id,
            proposal_amount,
            data.clone(),
        );

        let proposal_id = 0;
        assert!(Proposals::<T>::contains_key(proposal_id));
        assert!(ProposalRoundsExpiring::<T>::get(expiry_block).contains(&proposal_id));
        assert!(OngoingProposalVotes::<T>::get(proposal_id).is_some());
    }

    #[benchmark]
    fn challenge_against_letting_agent() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let expiry_block = <System<T>>::block_number().saturating_add(T::VotingTime::get());
        let mut challenges = BoundedVec::default();
        for i in 1..T::MaxVotesForBlock::get() {
            challenges.try_push(i).unwrap();
        }
        ChallengeRoundsExpiring::<T>::insert(expiry_block, challenges);

        let asset_id = 0;

        #[extrinsic_call]
        challenge_against_letting_agent(RawOrigin::Signed(token_owner.clone()), asset_id);

        assert!(Challenges::<T>::contains_key(asset_id));
        assert!(ChallengeRoundsExpiring::<T>::get(expiry_block).contains(&asset_id));
        assert!(OngoingChallengeVotes::<T>::get(asset_id).is_some());
    }

    #[benchmark]
    fn vote_on_proposal() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let letting_agent =
            set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let data = BoundedVec::try_from("Proposal".as_bytes().to_vec()).unwrap();
        let asset_id = 0;
        let proposal_amount = T::LowProposal::get().saturating_mul(2_u32.into());

        assert_ok!(PropertyGovernance::<T>::propose(
            RawOrigin::Signed(letting_agent.clone()).into(),
            asset_id,
            proposal_amount,
            data.clone(),
        ));
        assert_ok!(PropertyGovernance::<T>::vote_on_proposal(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            crate::Vote::No,
        ));

        #[extrinsic_call]
        vote_on_proposal(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            crate::Vote::Yes,
        );

        let proposal_id = 0;
        assert_eq!(
            OngoingProposalVotes::<T>::get(proposal_id)
                .unwrap()
                .yes_voting_power,
            1
        );
        assert_eq!(
            UserProposalVote::<T>::get(proposal_id, token_owner).unwrap(),
            crate::Vote::Yes
        );
    }

    #[benchmark]
    fn vote_on_letting_agent_challenge() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;
        assert_ok!(PropertyGovernance::<T>::challenge_against_letting_agent(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id
        ));
        assert_ok!(PropertyGovernance::<T>::vote_on_letting_agent_challenge(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            crate::Vote::No
        ));

        #[extrinsic_call]
        vote_on_letting_agent_challenge(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            crate::Vote::Yes,
        );

        assert_eq!(
            OngoingChallengeVotes::<T>::get(asset_id)
                .unwrap()
                .yes_voting_power,
            1
        );
        assert_eq!(
            UserChallengeVote::<T>::get(asset_id, token_owner).unwrap(),
            crate::Vote::Yes
        );
    }

    #[benchmark]
    fn propose_property_sale() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let expiry_block = <System<T>>::block_number().saturating_add(T::SaleVotingTime::get());
        let mut sale_proposals = BoundedVec::default();
        for i in 1..T::MaxVotesForBlock::get() {
            sale_proposals.try_push(i).unwrap();
        }
        ProposalRoundsExpiring::<T>::insert(expiry_block, sale_proposals);

        let asset_id = 0;

        #[extrinsic_call]
        propose_property_sale(RawOrigin::Signed(token_owner.clone()), asset_id);

        assert!(SaleProposals::<T>::contains_key(asset_id));
        assert!(SaleProposalRoundsExpiring::<T>::get(expiry_block).contains(&asset_id));
        assert!(OngoingSaleProposalVotes::<T>::get(asset_id).is_some());
    }

    #[benchmark]
    fn vote_on_property_sale() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;

        assert_ok!(PropertyGovernance::<T>::propose_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
        ));
        assert_ok!(PropertyGovernance::<T>::vote_on_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            crate::Vote::No,
        ));

        #[extrinsic_call]
        vote_on_property_sale(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            crate::Vote::Yes,
        );

        assert_eq!(
            OngoingSaleProposalVotes::<T>::get(asset_id)
                .unwrap()
                .yes_voting_power,
            1
        );
        assert_eq!(
            UserSaleProposalVote::<T>::get(asset_id, token_owner).unwrap(),
            crate::Vote::Yes
        );
    }

    #[benchmark]
    fn bid_on_sale() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;

        assert_ok!(PropertyGovernance::<T>::propose_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
        ));
        let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
        for owner in owner_list {
            assert_ok!(PropertyGovernance::<T>::vote_on_property_sale(
                RawOrigin::Signed(owner.clone()).into(),
                asset_id,
                crate::Vote::Yes,
            ));
        }

        let expiry = System::<T>::block_number() + T::SaleVotingTime::get();
        run_to_block::<T>(expiry);

        assert_eq!(SaleAuctions::<T>::get(asset_id).is_some(), true);

        let bidder_1: T::AccountId = account("bidder1", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            bidder_1.clone()
        ));
        let bidder_2: T::AccountId = account("bidder2", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            bidder_2.clone()
        ));
        let deposit = T::HighProposal::get();
        let total_funds = deposit.saturating_mul(1000u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &bidder_1,
            total_funds
        ));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &bidder_2,
            total_funds
        ));
        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        let auction_amount: <T as pallet::Config>::Balance = 100_000_000_000u128.into();
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &bidder_1,
            auction_amount.saturating_mul(100u128.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &bidder_2,
            auction_amount.saturating_mul(100u128.into())
        ));

        assert_ok!(PropertyGovernance::<T>::bid_on_sale(
            RawOrigin::Signed(bidder_1.clone()).into(),
            asset_id,
            auction_amount,
            payment_asset,
        ));

        #[extrinsic_call]
        bid_on_sale(
            RawOrigin::Signed(bidder_2.clone()),
            asset_id,
            auction_amount.saturating_mul(2u128.into()),
            payment_asset,
        );

        let auction = SaleAuctions::<T>::get(asset_id).unwrap();
        assert_eq!(auction.highest_bidder, Some(bidder_2.clone()));
        assert_eq!(auction.price, auction_amount.saturating_mul(2u128.into()));
    }

    #[benchmark]
    fn lawyer_claim_sale() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;

        assert_ok!(PropertyGovernance::<T>::propose_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
        ));
        let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
        for owner in owner_list {
            assert_ok!(PropertyGovernance::<T>::vote_on_property_sale(
                RawOrigin::Signed(owner.clone()).into(),
                asset_id,
                crate::Vote::Yes,
            ));
        }

        let expiry = System::<T>::block_number() + T::SaleVotingTime::get();
        run_to_block::<T>(expiry);

        assert_eq!(SaleAuctions::<T>::get(asset_id).is_some(), true);

        let bidder: T::AccountId = account("bidder", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            bidder.clone()
        ));
        let deposit = T::HighProposal::get();
        let total_funds = deposit.saturating_mul(1000u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &bidder,
            total_funds
        ));
        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        let auction_amount: <T as pallet::Config>::Balance = 100_000_000_000u128.into();
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &bidder,
            auction_amount.saturating_mul(100u128.into())
        ));

        assert_ok!(PropertyGovernance::<T>::bid_on_sale(
            RawOrigin::Signed(bidder.clone()).into(),
            asset_id,
            auction_amount,
            payment_asset,
        ));

        let lawyer: T::AccountId = account("lawyer", 0, 0);

        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(region_owner.clone()).into(),
            region_id,
            lawyer.clone()
        ));

        let auction_expiry = System::<T>::block_number() + T::AuctionTime::get();
        run_to_block::<T>(auction_expiry);

        let costs = 500_000_000_u32.into();

        #[extrinsic_call]
        lawyer_claim_sale(
            RawOrigin::Signed(lawyer.clone()),
            asset_id,
            LegalSale::SpvSide,
            costs,
        );

        let sale_info = PropertySale::<T>::get(asset_id).unwrap();
        assert_eq!(sale_info.spv_lawyer, Some(lawyer.clone()));
        assert_eq!(sale_info.spv_lawyer_costs, costs);
        assert!(sale_info.buyer_lawyer.is_none());
    }

    #[benchmark]
    fn lawyer_confirm_sale() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;

        assert_ok!(PropertyGovernance::<T>::propose_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
        ));
        let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
        for owner in owner_list {
            assert_ok!(PropertyGovernance::<T>::vote_on_property_sale(
                RawOrigin::Signed(owner.clone()).into(),
                asset_id,
                crate::Vote::Yes,
            ));
        }

        let expiry = System::<T>::block_number() + T::SaleVotingTime::get();
        run_to_block::<T>(expiry);

        assert_eq!(SaleAuctions::<T>::get(asset_id).is_some(), true);

        let bidder: T::AccountId = account("bidder", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            bidder.clone()
        ));
        let deposit = T::HighProposal::get();
        let total_funds = deposit.saturating_mul(1000u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &bidder,
            total_funds
        ));
        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        let auction_amount: <T as pallet::Config>::Balance = 100_000_000_000u128.into();
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &bidder,
            auction_amount.saturating_mul(100u128.into())
        ));

        assert_ok!(PropertyGovernance::<T>::bid_on_sale(
            RawOrigin::Signed(bidder.clone()).into(),
            asset_id,
            auction_amount,
            payment_asset,
        ));

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        let auction_expiry = System::<T>::block_number() + T::AuctionTime::get();
        run_to_block::<T>(auction_expiry);

        let costs = 500_000_000_u32.into();

        assert_ok!(PropertyGovernance::<T>::lawyer_claim_sale(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            asset_id,
            LegalSale::SpvSide,
            costs
        ));
        assert_ok!(PropertyGovernance::<T>::lawyer_claim_sale(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            asset_id,
            LegalSale::BuyerSide,
            costs
        ));
        assert_ok!(PropertyGovernance::<T>::lawyer_confirm_sale(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            asset_id,
            false
        ));

        #[extrinsic_call]
        lawyer_confirm_sale(RawOrigin::Signed(lawyer_2.clone()), asset_id, false);

        assert!(PropertySale::<T>::get(asset_id).is_none());
    }

    #[benchmark]
    fn finalize_sale() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;

        assert_ok!(PropertyGovernance::<T>::propose_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
        ));
        let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
        for owner in owner_list.clone() {
            assert_ok!(PropertyGovernance::<T>::vote_on_property_sale(
                RawOrigin::Signed(owner.clone()).into(),
                asset_id,
                crate::Vote::Yes,
            ));
        }

        let expiry = System::<T>::block_number() + T::SaleVotingTime::get();
        run_to_block::<T>(expiry);

        assert_eq!(SaleAuctions::<T>::get(asset_id).is_some(), true);

        let bidder: T::AccountId = account("bidder", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            bidder.clone()
        ));
        let deposit = T::HighProposal::get();
        let total_funds = deposit.saturating_mul(1000u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &bidder,
            total_funds
        ));
        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        let auction_amount: <T as pallet::Config>::Balance = 100_000_000_000u128.into();
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &bidder,
            auction_amount.saturating_mul(100u128.into())
        ));

        assert_ok!(PropertyGovernance::<T>::bid_on_sale(
            RawOrigin::Signed(bidder.clone()).into(),
            asset_id,
            auction_amount,
            payment_asset,
        ));

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        let auction_expiry = System::<T>::block_number() + T::AuctionTime::get();
        run_to_block::<T>(auction_expiry);

        let costs = 500_000_000_u32.into();

        assert_ok!(PropertyGovernance::<T>::lawyer_claim_sale(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            asset_id,
            LegalSale::SpvSide,
            costs
        ));
        assert_ok!(PropertyGovernance::<T>::lawyer_claim_sale(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            asset_id,
            LegalSale::BuyerSide,
            costs
        ));
        assert_ok!(PropertyGovernance::<T>::lawyer_confirm_sale(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            asset_id,
            true
        ));

        assert_ok!(PropertyGovernance::<T>::lawyer_confirm_sale(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            asset_id,
            true
        ));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &lawyer_2,
            total_funds
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &lawyer_2,
            auction_amount.saturating_mul(100u128.into())
        ));

        #[extrinsic_call]
        finalize_sale(RawOrigin::Signed(lawyer_2.clone()), asset_id, payment_asset);

        assert!(PropertySale::<T>::get(asset_id).unwrap().finalized);
        let net_amount = auction_amount.saturating_mul(98u128.into()) / 100u128.into();
        for owner in owner_list.iter() {
            let owner_share = net_amount
                / (<T as pallet_marketplace::Config>::MaxPropertyToken::get() as u128).into();
            assert_eq!(
                SaleFunds::<T>::get((owner, asset_id, payment_asset)),
                owner_share
            );
        }
    }

    #[benchmark]
    fn claim_sale_funds() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());
        let _ = set_letting_agent::<T>(region_owner.clone(), region_id, location.clone(), 0);

        let asset_id = 0;

        assert_ok!(PropertyGovernance::<T>::propose_property_sale(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
        ));
        let owner_list = <T as pallet::Config>::PropertyToken::get_property_owner(asset_id);
        for owner in owner_list.clone() {
            assert_ok!(PropertyGovernance::<T>::vote_on_property_sale(
                RawOrigin::Signed(owner.clone()).into(),
                asset_id,
                crate::Vote::Yes,
            ));
        }

        let expiry = System::<T>::block_number() + T::SaleVotingTime::get();
        run_to_block::<T>(expiry);

        assert_eq!(SaleAuctions::<T>::get(asset_id).is_some(), true);

        let bidder: T::AccountId = account("bidder", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            bidder.clone()
        ));
        let deposit = T::HighProposal::get();
        let total_funds = deposit.saturating_mul(1000u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &bidder,
            total_funds
        ));
        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        let auction_amount: <T as pallet::Config>::Balance = 100_000_000_000u128.into();
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &bidder,
            auction_amount.saturating_mul(100u128.into())
        ));

        assert_ok!(PropertyGovernance::<T>::bid_on_sale(
            RawOrigin::Signed(bidder.clone()).into(),
            asset_id,
            auction_amount,
            payment_asset,
        ));

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        let auction_expiry = System::<T>::block_number() + T::AuctionTime::get();
        run_to_block::<T>(auction_expiry);

        let costs = 500_000_000_u32.into();

        assert_ok!(PropertyGovernance::<T>::lawyer_claim_sale(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            asset_id,
            LegalSale::SpvSide,
            costs
        ));
        assert_ok!(PropertyGovernance::<T>::lawyer_claim_sale(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            asset_id,
            LegalSale::BuyerSide,
            costs
        ));
        assert_ok!(PropertyGovernance::<T>::lawyer_confirm_sale(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            asset_id,
            true
        ));

        assert_ok!(PropertyGovernance::<T>::lawyer_confirm_sale(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            asset_id,
            true
        ));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &lawyer_2,
            total_funds
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &lawyer_2,
            auction_amount.saturating_mul(100u128.into())
        ));

        assert_ok!(PropertyGovernance::<T>::finalize_sale(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            asset_id,
            payment_asset
        ));

        for owner in owner_list.iter() {
            if *owner != token_owner {
                assert_ok!(PropertyGovernance::<T>::claim_sale_funds(
                    RawOrigin::Signed(owner.clone()).into(),
                    asset_id,
                    payment_asset
                ));
            }
        }

        assert_eq!(
            PropertySale::<T>::get(asset_id)
                .unwrap()
                .property_token_amount,
            1
        );

        #[extrinsic_call]
        claim_sale_funds(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            payment_asset,
        );

        assert!(PropertySale::<T>::get(asset_id).is_none());
        assert_eq!(
            SaleFunds::<T>::get((&token_owner, asset_id, payment_asset)),
            0u128.into()
        );
        assert_eq!(
            <T as pallet::Config>::PropertyToken::get_property_asset_info(asset_id).is_none(),
            true
        );
    }

    impl_benchmark_test_suite!(
        PropertyGovernance,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
