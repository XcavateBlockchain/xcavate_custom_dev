use crate::{mock::*, Error, Event};
use crate::{
    HoldReason, LastRegionProposalBlock, LocationRegistration, OngoingRegionOwnerProposalVotes,
    OngoingRegionProposalVotes, RegionAuctions, RegionDetails, RegionOperatorAccounts,
    RegionOwnerProposals, RegionProposals, RegionReplacementAuctions, UserRegionOwnerVote,
    UserRegionVote, VoteStats,
};
use frame_support::BoundedVec;
use frame_support::{
    assert_noop, assert_ok,
    traits::{fungible::Inspect, fungible::InspectHold, OnFinalize, OnInitialize},
};
use sp_runtime::{traits::BadOrigin, Permill, TokenError};

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            Regions::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Regions::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        bvec![10, 10]
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        0,
        crate::Vote::Yes
    ));
    run_to_block(31);
    assert_ok!(Regions::bid_on_region(
        RuntimeOrigin::signed([8; 32].into()),
        0,
        100_000
    ));
    run_to_block(61);
    assert_ok!(Regions::create_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        0,
        30,
        Permill::from_percent(3)
    ));
}

#[test]
fn add_regional_operator_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_eq!(
            RegionOperatorAccounts::<Test>::get::<AccountId>([8; 32].into()),
            Some(())
        );
    })
}

#[test]
fn add_regional_operator_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Regions::add_regional_operator(RuntimeOrigin::signed([8; 32].into()), [8; 32].into()),
            BadOrigin
        );
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_eq!(
            RegionOperatorAccounts::<Test>::get::<AccountId>([8; 32].into()),
            Some(())
        );
        assert_noop!(
            Regions::add_regional_operator(RuntimeOrigin::root(), [8; 32].into()),
            Error::<Test>::AlreadyRegionOperator
        );
    })
}

#[test]
fn remove_regional_operator_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_eq!(
            RegionOperatorAccounts::<Test>::get::<AccountId>([8; 32].into()),
            Some(())
        );
        assert_ok!(Regions::remove_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_eq!(
            RegionOperatorAccounts::<Test>::get::<AccountId>([8; 32].into()),
            None
        );
    })
}

#[test]
fn remove_regional_operator_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_eq!(
            RegionOperatorAccounts::<Test>::get::<AccountId>([8; 32].into()),
            Some(())
        );
        assert_noop!(
            Regions::remove_regional_operator(
                RuntimeOrigin::signed([8; 32].into()),
                [8; 32].into()
            ),
            BadOrigin
        );
        assert_noop!(
            Regions::remove_regional_operator(RuntimeOrigin::root(), [7; 32].into()),
            Error::<Test>::NoRegionalOperator
        );
    })
}

#[test]
fn propose_new_region_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_eq!(RegionProposals::<Test>::get(0).unwrap().proposal_expiry, 31);
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0
            }
        );
    })
}

#[test]
fn propose_new_region_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Regions::propose_new_region(RuntimeOrigin::signed([0; 32].into()), bvec![10, 10]),
            Error::<Test>::UserNotWhitelisted
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_eq!(LastRegionProposalBlock::<Test>::get(), None);
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_eq!(LastRegionProposalBlock::<Test>::get(), Some(1));
        run_to_block(10);
        assert_noop!(
            Regions::propose_new_region(RuntimeOrigin::signed([0; 32].into()), bvec![10, 10]),
            Error::<Test>::RegionProposalCooldownActive
        );
    })
}

#[test]
fn vote_on_region_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [2; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 300_000,
                no_voting_power: 0
            }
        );
        assert_eq!(
            UserRegionVote::<Test>::get::<u32, AccountId>(0, [2; 32].into()).unwrap(),
            crate::Vote::Yes
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 200_000,
                no_voting_power: 450_000
            }
        );
        assert_eq!(
            UserRegionVote::<Test>::get::<u32, AccountId>(0, [2; 32].into()).unwrap(),
            crate::Vote::No
        );
    })
}

#[test]
fn vote_on_region_proposal_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [2; 32].into()
        ));
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([3; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::UserNotWhitelisted
        );
        run_to_block(40);
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::ProposalExpired
        );
    })
}

#[test]
fn bid_on_region_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [2; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            10_000
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 190_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            10_000
        );
        assert_eq!(RegionAuctions::<Test>::get(0).unwrap().collateral, 10_000);
        assert_eq!(
            RegionAuctions::<Test>::get(0)
                .unwrap()
                .highest_bidder
                .unwrap(),
            [0; 32].into()
        );
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25_001
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 200_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::free_balance(&([1; 32].into())), 124_999);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            25_001
        );
        assert_eq!(RegionAuctions::<Test>::get(0).unwrap().collateral, 25_001);
        assert_eq!(
            RegionAuctions::<Test>::get(0)
                .unwrap()
                .highest_bidder
                .unwrap(),
            [1; 32].into()
        );
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            190_000
        ));
        assert_eq!(RegionAuctions::<Test>::get(0).unwrap().collateral, 190_000);
        assert_eq!(
            RegionAuctions::<Test>::get(0)
                .unwrap()
                .highest_bidder
                .unwrap(),
            [0; 32].into()
        );
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            195_000
        ));
        assert_eq!(RegionAuctions::<Test>::get(0).unwrap().collateral, 195_000);
        assert_eq!(
            RegionAuctions::<Test>::get(0)
                .unwrap()
                .highest_bidder
                .unwrap(),
            [0; 32].into()
        );
        assert_eq!(Balances::free_balance(&([0; 32].into())), 5_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            195_000
        );
        assert_eq!(Balances::free_balance(&([1; 32].into())), 150_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            0
        );
    })
}

#[test]
fn bid_on_region_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 0, 1_000),
            Error::<Test>::VotingStillOngoing,
        );
        run_to_block(31);
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([3; 32].into()), 0, 1_000),
            Error::<Test>::UserNotRegionalOperator,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 0, 0),
            Error::<Test>::BidCannotBeZero,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 0, 5_000),
            Error::<Test>::BidBelowMinimum,
        );
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            10_000
        ));
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 0, 5_000),
            Error::<Test>::BidTooLow,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 0, 10_000),
            Error::<Test>::BidTooLow,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 0, 160_000),
            TokenError::FundsUnavailable,
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            crate::Vote::No
        ));
        run_to_block(61);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            10_000
        ));
        System::assert_last_event(Event::RegionRejected { proposal_id: 1 }.into());
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 1, 10_000),
            Error::<Test>::NoOngoingAuction,
        );
    })
}

#[test]
fn create_new_region_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(29);
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            100_000
        ));
        run_to_block(59);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            70_000
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 30_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            170_000
        );
        run_to_block(61);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            30,
            Permill::from_percent(3)
        ));
        assert_eq!(RegionAuctions::<Test>::get(0).is_none(), true);
        run_to_block(89);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            30,
            Permill::from_percent(3)
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 30_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            170_000
        );
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().collection_id, 0);
        assert_eq!(RegionDetails::<Test>::get(1).unwrap().collection_id, 1);
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().listing_duration, 30);
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().owner, [0; 32].into());
    })
}

#[test]
fn create_new_region_does_not_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [7; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [7; 32].into()
        ));
        assert_noop!(
            Regions::create_new_region(
                RuntimeOrigin::signed([7; 32].into()),
                0,
                30,
                Permill::from_percent(3)
            ),
            Error::<Test>::NoAuction
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(31);
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            100_000
        ));
        assert_noop!(
            Regions::create_new_region(
                RuntimeOrigin::signed([7; 32].into()),
                0,
                30,
                Permill::from_percent(3)
            ),
            Error::<Test>::AuctionNotFinished
        );
        run_to_block(61);
        assert_noop!(
            Regions::create_new_region(
                RuntimeOrigin::signed([7; 32].into()),
                0,
                30,
                Permill::from_percent(3)
            ),
            Error::<Test>::NotRegionOwner
        );
        assert_noop!(
            Regions::create_new_region(
                RuntimeOrigin::signed([8; 32].into()),
                0,
                0,
                Permill::from_percent(3)
            ),
            Error::<Test>::ListingDurationCantBeZero
        );
        assert_noop!(
            Regions::create_new_region(
                RuntimeOrigin::signed([8; 32].into()),
                0,
                10_001,
                Permill::from_percent(3)
            ),
            Error::<Test>::ListingDurationTooHigh
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            1,
            crate::Vote::Yes
        ));
        run_to_block(121);
        assert_noop!(
            Regions::create_new_region(
                RuntimeOrigin::signed([8; 32].into()),
                1,
                10_000,
                Permill::from_percent(3)
            ),
            Error::<Test>::NoAuction
        );
    })
}

#[test]
fn adjust_listing_duration_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().listing_duration, 30);
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::adjust_listing_duration(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            50,
        ));
    })
}

#[test]
fn adjust_listing_duration_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Regions::adjust_listing_duration(RuntimeOrigin::signed([8; 32].into()), 0, 50,),
            Error::<Test>::UserNotWhitelisted
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_noop!(
            Regions::adjust_listing_duration(RuntimeOrigin::signed([8; 32].into()), 0, 50,),
            Error::<Test>::RegionUnknown
        );
        new_region_helper();
        assert_noop!(
            Regions::adjust_listing_duration(RuntimeOrigin::signed([0; 32].into()), 0, 50,),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            Regions::adjust_listing_duration(RuntimeOrigin::signed([8; 32].into()), 0, 0,),
            Error::<Test>::ListingDurationCantBeZero
        );
        assert_noop!(
            Regions::adjust_listing_duration(RuntimeOrigin::signed([8; 32].into()), 0, 100000,),
            Error::<Test>::ListingDurationTooHigh
        );
    })
}

// create_new_location function
#[test]
fn create_new_location_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            bvec![10, 10]
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            1,
            crate::Vote::Yes
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            1,
            100_000
        ));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            1,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![9, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            1,
            bvec![9, 10]
        ));
        assert_eq!(Balances::free_balance(&([8; 32].into())), 197_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            203_000
        );
        assert_eq!(
            LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(0, bvec![10, 10]),
            true
        );
        assert_eq!(
            LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(0, bvec![9, 10]),
            true
        );
        assert_eq!(
            LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(1, bvec![9, 10]),
            true
        );
        assert_eq!(
            LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(1, bvec![10, 10]),
            false
        );
        assert_eq!(
            LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(1, bvec![8, 10]),
            false
        );
    })
}

#[test]
fn create_new_location_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]),
            Error::<Test>::RegionUnknown
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_noop!(
            Regions::create_new_location(RuntimeOrigin::signed([7; 32].into()), 0, bvec![10, 10]),
            Error::<Test>::NoPermission
        );
    })
}

#[test]
fn propose_remove_regional_operator_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0
            }
        );
    })
}

#[test]
fn propose_remove_regional_operator_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_noop!(
            Regions::propose_remove_regional_operator(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::RegionUnknown,
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_noop!(
            Regions::propose_remove_regional_operator(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::UserNotWhitelisted,
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0
            }
        );
        assert_noop!(
            Regions::propose_remove_regional_operator(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::ProposalAlreadyOngoing,
        );
    })
}

#[test]
fn vote_on_remove_owner_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0
            }
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::No
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 200_000,
                no_voting_power: 400_000
            }
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u32, AccountId>(0, [0; 32].into()).unwrap(),
            crate::Vote::Yes
        );
    })
}

#[test]
fn vote_on_remove_owner_proposal_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([8; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::NotOngoing,
        );
        new_region_helper();
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([8; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::NotOngoing,
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::UserNotWhitelisted,
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(91);
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::Vote::Yes
            ),
            Error::<Test>::NotOngoing,
        );
    })
}

#[test]
fn remove_owner_proposal_passes() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            1_000
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 600_000,
                no_voting_power: 0
            }
        );
        run_to_block(91);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            90_000
        );
        assert_eq!(Balances::total_balance(&([8; 32].into())), 390_000);
        assert_eq!(Balances::total_issuance(), 1_045_000);
        System::assert_last_event(
            Event::RegionalOperatorSlashed {
                region_id: 0,
                operator: [8; 32].into(),
                amount: 10_000,
            }
            .into(),
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().active_strikes,
            1
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(121);
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(151);
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().active_strikes,
            3
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            151
        );
        System::assert_last_event(
            Event::RegionOwnerChangeEnabled {
                region_id: 0,
                next_change_allowed: 151,
            }
            .into(),
        );
    })
}

#[test]
fn remove_owner_proposal_doesnt_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            1_000
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        run_to_block(91);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 199_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_none(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).is_none(),
            true
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u32, AccountId>(0, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 599_000,
                no_voting_power: 0
            }
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::No
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 400_000,
                no_voting_power: 199_000
            }
        );
        run_to_block(121);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 198_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_none(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).is_none(),
            true
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u32, AccountId>(0, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            100_000
        );
        assert_eq!(Balances::total_balance(&([8; 32].into())), 400_000);
        assert_eq!(Balances::total_issuance(), 1_053_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_none(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).is_none(),
            true
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u32, AccountId>(0, [0; 32].into()).is_none(),
            true
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(151);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 198_000);
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::No
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 198_000,
                no_voting_power: 390_000
            }
        );
        run_to_block(181);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionalOperatorRemovalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 197_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(0).is_none(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(0).is_none(),
            true
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u32, AccountId>(0, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().active_strikes,
            1
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
    })
}

#[test]
fn bid_on_region_replacement_after_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![11, 10]
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(91);
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        run_to_block(121);
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().active_strikes,
            2
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes
        ));
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
        run_to_block(152);
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            151
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        assert_eq!(RegionReplacementAuctions::<Test>::get(0).is_none(), true);
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            12_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            72_000
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            12_000
        );
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(0).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([0; 32].into()),
                collateral: 12_000,
                auction_expiry: 182,
            }
        );
        run_to_block(170);
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            35_000
        ));
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(0).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([1; 32].into()),
                collateral: 35_000,
                auction_expiry: 182,
            }
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            35_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            40_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            112_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            51_000
        ));
        run_to_block(213);
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().owner, [0; 32].into());
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().collateral, 51_000);
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            482
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().active_strikes,
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            51_000
        );
        assert_eq!(RegionReplacementAuctions::<Test>::get(0).is_none(), true);
    })
}

#[test]
fn bid_on_region_replacement_after_time_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        new_region_helper();
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
        run_to_block(362);
        assert_eq!(RegionReplacementAuctions::<Test>::get(0).is_none(), true);
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            80_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            100_000
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            80_000
        );
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(0).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([1; 32].into()),
                collateral: 80_000,
                auction_expiry: 392,
            }
        );
        run_to_block(370);
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            85_000
        ));
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(0).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([1; 32].into()),
                collateral: 85_000,
                auction_expiry: 392,
            }
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            85_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            90_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            190_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            92_000
        ));
        run_to_block(394);
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().owner, [0; 32].into());
        assert_eq!(RegionDetails::<Test>::get(0).unwrap().collateral, 92_000);
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            692
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            92_000
        );
        assert_eq!(RegionReplacementAuctions::<Test>::get(0).is_none(), true);
    })
}

#[test]
fn bid_on_region_replacement_after_time_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 0, 10_000),
            Error::<Test>::RegionUnknown
        );
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            0,
            bvec![11, 10]
        ));
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
        run_to_block(200);
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 0, 10_000),
            Error::<Test>::RegionOwnerCantBeChanged
        );
        run_to_block(362);
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([2; 32].into()), 0, 10_000),
            Error::<Test>::UserNotRegionalOperator
        );
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 0, 11_500),
            Error::<Test>::BidBelowMinimum
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            80_000
        ));
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 0, 10_000),
            Error::<Test>::BidTooLow
        );
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 0, 200_000),
            TokenError::FundsUnavailable,
        );
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([0; 32].into()), 0, 300_000),
            TokenError::FundsUnavailable,
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            80_000
        );
    })
}

#[test]
fn initiate_region_owner_resignation_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        new_region_helper();
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
        run_to_block(100);
        assert_ok!(Regions::initiate_region_owner_resignation(
            RuntimeOrigin::signed([8; 32].into()),
            0
        ));
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            200
        );
        System::assert_last_event(
            Event::RegionOwnerResignationInitiated {
                region_id: 0,
                region_owner: [8; 32].into(),
                next_owner_change: 200,
            }
            .into(),
        );
        run_to_block(201);
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            80_000
        ));
    })
}

#[test]
fn initiate_region_owner_resignation_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [8; 32].into()
        ));
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([8; 32].into()), 0),
            Error::<Test>::RegionUnknown
        );
        new_region_helper();
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::NotRegionOwner
        );
        assert_eq!(
            RegionDetails::<Test>::get(0).unwrap().next_owner_change,
            361
        );
        run_to_block(300);
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([8; 32].into()), 0),
            Error::<Test>::OwnerChangeTooEarly
        );
        run_to_block(400);
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([8; 32].into()), 0),
            Error::<Test>::OwnerChangeTooEarly
        );
    })
}
