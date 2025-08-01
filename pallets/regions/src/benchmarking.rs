//! Benchmarking setup for pallet-regions
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Regions;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_xcavate_whitelist::Pallet as Whitelist;
use scale_info::prelude::vec;
use sp_runtime::Permill;

pub trait Config: pallet_xcavate_whitelist::Config + crate::Config {}

impl<T: crate::Config + pallet_xcavate_whitelist::Config> Config for T {}

#[benchmarks]
mod benchmarks {
    use super::*;

    fn create_whitelisted_user<T: Config>() -> T::AccountId {
        let signer: T::AccountId = account("signer", 0, 0);
        Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), signer.clone()).unwrap();
        signer
    }

    fn create_a_new_region<T: Config>(signer: T::AccountId) -> u16 {
        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, deposit * 1000u32.into());

        LastRegionProposalBlock::<T>::kill();
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

        let auction_amount = T::MinimumRegionDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, auction_amount * 100u32.into());

        let bid_amount = auction_amount.saturating_mul(10u32.into());

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Regions::<T>::bid_on_region(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            bid_amount
        ));

        let auction_expiry =
            frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
        frame_system::Pallet::<T>::set_block_number(auction_expiry);
        assert_ok!(Regions::<T>::create_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            T::MaxListingDuration::get(),
            Permill::from_percent(5)
        ));
        region_id
    }

    #[benchmark]
    fn propose_new_region() {
        let signer: T::AccountId = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;

        assert!(!ProposedRegionIds::<T>::contains_key(
            region.clone().into_u16()
        ));
        assert!(!RegionDetails::<T>::contains_key(region.clone().into_u16()));

        let deposit = T::RegionProposalDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, deposit * 10u32.into());

        LastRegionProposalBlock::<T>::kill();
        assert_ok!(Regions::<T>::add_regional_operator(
            RawOrigin::Root.into(),
            signer.clone()
        ));

        #[extrinsic_call]
        propose_new_region(RawOrigin::Signed(signer.clone()), region.clone());

        let current_block = frame_system::Pallet::<T>::block_number();
        let expiry_block = current_block.saturating_add(T::RegionVotingTime::get());

        assert!(ProposedRegionIds::<T>::contains_key(
            region.clone().into_u16()
        ));
        assert_eq!(
            RegionProposals::<T>::get(region.into_u16())
                .unwrap()
                .proposal_expiry,
            expiry_block
        );
    }

    #[benchmark]
    fn vote_on_region_proposal() {
        let signer: T::AccountId = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, deposit * 10u32.into());

        LastRegionProposalBlock::<T>::kill();
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

        #[extrinsic_call]
        vote_on_region_proposal(RawOrigin::Signed(signer.clone()), region_id, Vote::Yes);

        assert_eq!(
            UserRegionVote::<T>::get(region_id).unwrap().get(&signer).cloned().unwrap().vote,
            Vote::Yes
        );
    }

    #[benchmark]
    fn bid_on_region() {
        let signer: T::AccountId = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, deposit * 1000u32.into());

        LastRegionProposalBlock::<T>::kill();
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

        for i in 1..T::MaxRegionVoters::get(){
            let voter: T::AccountId = account("voter", i, i);
            let _ = T::NativeCurrency::mint_into(&voter, deposit * 1000u32.into());
            Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), voter.clone()).unwrap();
            assert_ok!(Regions::<T>::vote_on_region_proposal(
                RawOrigin::Signed(voter.clone()).into(),
                region_id,
                Vote::Yes
            ));
        }

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        let auction_amount = T::MinimumRegionDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, auction_amount * 100u32.into());

        let bid_amount = auction_amount.saturating_mul(10u32.into());

        let first_bidder: T::AccountId = account("first_bidder", 0, 0);

        let _ = T::NativeCurrency::mint_into(&first_bidder, auction_amount * 100u32.into());

        let first_bid_amount = auction_amount.saturating_mul(9u32.into());
        assert_ok!(Regions::<T>::add_regional_operator(
            RawOrigin::Root.into(),
            first_bidder.clone()
        ));
/*         assert_ok!(Regions::<T>::bid_on_region(
            RawOrigin::Signed(first_bidder).into(),
            region_id,
            first_bid_amount
        )); */

        #[extrinsic_call]
        bid_on_region(RawOrigin::Signed(signer.clone()), region_id, bid_amount);

        assert_eq!(
            RegionAuctions::<T>::get(region_id).unwrap().highest_bidder,
            Some(signer)
        );
    }

    #[benchmark]
    fn create_new_region() {
        let signer: T::AccountId = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, deposit * 1000u32.into());

        LastRegionProposalBlock::<T>::kill();
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

        let auction_amount = T::MinimumRegionDeposit::get();
        let _ = T::NativeCurrency::mint_into(&signer, auction_amount * 100u32.into());

        let bid_amount = auction_amount.saturating_mul(10u32.into());

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Regions::<T>::bid_on_region(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            bid_amount
        ));

        let auction_expiry =
            frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
        frame_system::Pallet::<T>::set_block_number(auction_expiry);

        #[extrinsic_call]
        create_new_region(
            RawOrigin::Signed(signer.clone()),
            region_id,
            T::MaxListingDuration::get(),
            Permill::from_percent(5),
        );

        assert_eq!(RegionDetails::<T>::get(region_id).unwrap().owner, signer);
    }

    #[benchmark]
    fn adjust_listing_duration() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());
        let new_listing_duration = T::MaxListingDuration::get() / 3u32.into();

        #[extrinsic_call]
        adjust_listing_duration(
            RawOrigin::Signed(signer.clone()),
            region_id,
            new_listing_duration,
        );

        assert_eq!(
            RegionDetails::<T>::get(region_id).unwrap().listing_duration,
            new_listing_duration
        );
    }

    #[benchmark]
    fn adjust_region_tax() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());
        let new_tax = Permill::from_percent(99);

        #[extrinsic_call]
        adjust_region_tax(RawOrigin::Signed(signer.clone()), region_id, new_tax);

        assert_eq!(RegionDetails::<T>::get(region_id).unwrap().tax, new_tax);
    }

    #[benchmark]
    fn create_new_location() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());
        let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();

        #[extrinsic_call]
        create_new_location(
            RawOrigin::Signed(signer.clone()),
            region_id,
            location.clone(),
        );

        assert!(LocationRegistration::<T>::contains_key(region_id, location));
        assert_eq!(
            RegionDetails::<T>::get(region_id).unwrap().location_count,
            1
        );
    }

    #[benchmark]
    fn propose_remove_regional_operator() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let proposer: T::AccountId = account("proposer", 0, 0);
        Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), proposer.clone()).unwrap();

        let deposit = T::RegionProposalDeposit::get();
        let _ = T::NativeCurrency::mint_into(&proposer, deposit * 10u32.into());

        let expiry_block =
            frame_system::Pallet::<T>::block_number() + T::RegionOperatorVotingTime::get();
        let dummy_region = u16::MAX;
        let max_proposals = T::MaxProposalsForBlock::get() as usize;
        RegionOwnerRoundsExpiring::<T>::insert(
            expiry_block,
            BoundedVec::truncate_from(vec![dummy_region; max_proposals - 1]),
        );

        #[extrinsic_call]
        propose_remove_regional_operator(RawOrigin::Signed(proposer.clone()), region_id);

        assert!(RegionOwnerProposals::<T>::contains_key(region_id));
        assert!(OngoingRegionOwnerProposalVotes::<T>::contains_key(
            region_id
        ));
    }

    #[benchmark]
    fn vote_on_remove_owner_proposal() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let proposer: T::AccountId = account("proposer", 0, 0);
        Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), proposer.clone()).unwrap();

        let voter: T::AccountId = account("voter", 0, 0);
        Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), voter.clone()).unwrap();

        let deposit = T::RegionProposalDeposit::get() * 100u32.into();
        let _ = T::NativeCurrency::mint_into(&proposer, deposit);

        let vote_power = T::MinimumVotingAmount::get() * 100u32.into();
        let _ = T::NativeCurrency::mint_into(&voter, vote_power);
        assert_ok!(Regions::<T>::propose_remove_regional_operator(
            RawOrigin::Signed(proposer.clone()).into(),
            region_id
        ));

        assert_ok!(Regions::<T>::vote_on_remove_owner_proposal(
            RawOrigin::Signed(voter.clone()).into(),
            region_id,
            Vote::No
        ));

        #[extrinsic_call]
        vote_on_remove_owner_proposal(RawOrigin::Signed(voter.clone()), region_id, Vote::Yes);

        assert_eq!(
            OngoingRegionOwnerProposalVotes::<T>::get(region_id)
                .unwrap()
                .yes_voting_power,
            vote_power
        );
        assert!(UserRegionOwnerVote::<T>::get(region_id).unwrap().get(&voter).is_some());
    }

    #[benchmark]
    fn bid_on_region_replacement() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let bidder_1: T::AccountId = account("bidder1", 0, 0);
        let bidder_2: T::AccountId = account("bidder2", 0, 0);
        Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), bidder_1.clone()).unwrap();
        Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), bidder_2.clone()).unwrap();
        assert_ok!(Regions::<T>::add_regional_operator(
            RawOrigin::Root.into(),
            bidder_1.clone()
        ));
        assert_ok!(Regions::<T>::add_regional_operator(
            RawOrigin::Root.into(),
            bidder_2.clone()
        ));

        let expiry = frame_system::Pallet::<T>::block_number()
            + T::RegionOwnerChangePeriod::get()
            + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        let base_bid = T::MinimumRegionDeposit::get() * 10u32.into();
        let _ = T::NativeCurrency::mint_into(&bidder_1, base_bid);
        let higher_bid = T::MinimumRegionDeposit::get() * 20u32.into();
        let _ = T::NativeCurrency::mint_into(&bidder_2, higher_bid);

        assert_ok!(Regions::<T>::bid_on_region_replacement(
            RawOrigin::Signed(bidder_1.clone()).into(),
            region_id,
            base_bid / 2u32.into()
        ));

        #[extrinsic_call]
        bid_on_region_replacement(
            RawOrigin::Signed(bidder_2.clone()),
            region_id,
            higher_bid / 2u32.into(),
        );

        let auction = RegionReplacementAuctions::<T>::get(region_id).unwrap();
        assert_eq!(auction.highest_bidder, Some(bidder_2));
        assert_eq!(auction.collateral, higher_bid / 2u32.into());
    }

    #[benchmark]
    fn initiate_region_owner_resignation() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let initial_block = frame_system::Pallet::<T>::block_number();
        let new_block = initial_block + 100u32.into();
        frame_system::Pallet::<T>::set_block_number(new_block);

        #[extrinsic_call]
        initiate_region_owner_resignation(RawOrigin::Signed(signer.clone()), region_id);

        let next_owner_change = new_block + T::RegionOwnerNoticePeriod::get();

        assert_eq!(
            RegionDetails::<T>::get(region_id)
                .unwrap()
                .next_owner_change,
            next_owner_change
        );
    }

    #[benchmark]
    fn add_regional_operator() {
        let regional_operator: T::AccountId = account("operator", 0, 0);

        #[extrinsic_call]
        add_regional_operator(RawOrigin::Root, regional_operator.clone());

        assert!(RegionOperatorAccounts::<T>::contains_key(
            &regional_operator
        ));
    }

    #[benchmark]
    fn remove_regional_operator() {
        let regional_operator: T::AccountId = account("operator", 0, 0);

        assert_ok!(Regions::<T>::add_regional_operator(
            RawOrigin::Root.into(),
            regional_operator.clone()
        ));
        assert!(RegionOperatorAccounts::<T>::contains_key(
            &regional_operator
        ));

        #[extrinsic_call]
        remove_regional_operator(RawOrigin::Root, regional_operator.clone());

        assert!(!RegionOperatorAccounts::<T>::contains_key(
            &regional_operator
        ));
    }

    #[benchmark]
    fn register_lawyer() {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);

        #[extrinsic_call]
        register_lawyer(RawOrigin::Signed(signer.clone()), region_id, lawyer.clone());

        assert_eq!(RealEstateLawyer::<T>::get(lawyer), Some(region_id));
    }

    impl_benchmark_test_suite!(Regions, crate::mock::new_test_ext(), crate::mock::Test);
}
