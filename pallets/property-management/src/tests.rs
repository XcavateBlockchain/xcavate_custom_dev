use crate::{mock::*, Error};
use frame_support::BoundedVec;
use frame_support::{
    assert_noop, assert_ok,
    traits::{fungible::InspectHold, OnFinalize, OnInitialize},
};

use crate::{
    HoldReason, InvestorFunds, LettingAgentProposal, LettingInfo, LettingStorage,
    OngoingLettingAgentVoting, UserLettingAgentVote,
};

use sp_runtime::{traits::BadOrigin, Permill, TokenError};

use pallet_marketplace::types::LegalProperty;

use pallet_regions::RegionIdentifier;

use pallet_real_estate_asset::Error as RealEstateAssetError;

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            PropertyManagement::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        PropertyManagement::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [6; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([6; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([6; 32].into()),
        3,
        pallet_regions::Vote::Yes
    ));
    run_to_block(31);
    assert_ok!(Regions::bid_on_region(
        RuntimeOrigin::signed([6; 32].into()),
        3,
        100_000
    ));
    run_to_block(61);
    assert_ok!(Regions::create_new_region(
        RuntimeOrigin::signed([6; 32].into()),
        3,
        30,
        Permill::from_percent(3)
    ));
}

fn lawyer_process_helper(
    real_estate_developer: AccountId,
    token_holder: AccountId,
    listing_id: u32,
) {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [10; 32].into(),
        pallet_xcavate_whitelist::Role::Lawyer
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [11; 32].into(),
        pallet_xcavate_whitelist::Role::Lawyer
    ));
    assert_ok!(Regions::register_lawyer(
        RuntimeOrigin::signed([10; 32].into()),
        3,
    ));
    assert_ok!(Regions::register_lawyer(
        RuntimeOrigin::signed([11; 32].into()),
        3,
    ));
    finalize_property_helper(real_estate_developer, token_holder, listing_id);
}

fn finalize_property_helper(
    real_estate_developer: AccountId,
    token_holder: AccountId,
    listing_id: u32,
) {
    assert_ok!(Marketplace::lawyer_claim_property(
        RuntimeOrigin::signed([10; 32].into()),
        listing_id,
        LegalProperty::RealEstateDeveloperSide,
        400,
    ));
    assert_ok!(Marketplace::approve_developer_lawyer(
        RuntimeOrigin::signed(real_estate_developer),
        listing_id,
        true
    ));
    assert_ok!(Marketplace::lawyer_claim_property(
        RuntimeOrigin::signed([11; 32].into()),
        listing_id,
        LegalProperty::SpvSide,
        400,
    ));
    assert_ok!(Marketplace::vote_on_spv_lawyer(
        RuntimeOrigin::signed(token_holder.clone()),
        listing_id,
        pallet_marketplace::types::Vote::Yes
    ));
    let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
    run_to_block(expiry);
    assert_ok!(Marketplace::finalize_spv_lawyer(
        RuntimeOrigin::signed(token_holder),
        listing_id,
    ));
    assert_ok!(Marketplace::lawyer_confirm_documents(
        RuntimeOrigin::signed([10; 32].into()),
        listing_id,
        true,
    ));
    assert_ok!(Marketplace::lawyer_confirm_documents(
        RuntimeOrigin::signed([11; 32].into()),
        listing_id,
        true,
    ));
}

#[test]
fn add_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        let location: BoundedVec<u8, Postcode> = bvec![10, 10];
        let letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        let location_info = letting_info.locations.get(&location).unwrap();
        assert_eq!(location_info.assigned_properties, 0);
        assert_eq!(location_info.deposit, 1_000);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_000);
    });
}

#[test]
fn add_letting_agent_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![11, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_000);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![11, 10],
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        let location: BoundedVec<u8, Postcode> = bvec![10, 10];
        let letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        let location_info = letting_info.locations.get(&location).unwrap();
        assert_eq!(location_info.assigned_properties, 0);
        assert_eq!(location_info.deposit, 1_000);
        let location_info_2 = letting_info.locations.get(&bvec![11, 10]).unwrap();
        assert_eq!(location_info_2.assigned_properties, 0);
        assert_eq!(location_info_2.deposit, 1_000);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_998_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            2_000
        );
    });
}

#[test]
fn add_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                bvec![10, 10],
            ),
            Error::<Test>::RegionUnknown
        );
        assert_ok!(XcavateWhitelist::remove_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
            ),
            Error::<Test>::LocationUnknown
        );
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
            ),
            Error::<Test>::LettingAgentInLocation
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([5; 32].into()),
                3,
                bvec![10, 10],
            ),
            TokenError::FundsUnavailable,
        );
    });
}

#[test]
fn remove_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        let location = bvec![10, 10];
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );
        let mut letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        if let Some(location_info) = letting_info.locations.get_mut(&location) {
            location_info.assigned_properties = 5;
        }
        let account: AccountId = [0; 32].into();
        LettingInfo::<Test>::insert(account.clone(), letting_info);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>(account)
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            5
        );
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![10, 10],
            ),
            Error::<Test>::LettingAgentActive
        );
        let mut letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        if let Some(location_info) = letting_info.locations.get_mut(&location) {
            location_info.assigned_properties = 0;
        }
        let account: AccountId = [0; 32].into();
        LettingInfo::<Test>::insert(account.clone(), letting_info);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            1_000
        );
        assert_ok!(PropertyManagement::remove_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10],
        ));
        assert!(LettingInfo::<Test>::get::<AccountId>(account)
            .unwrap()
            .locations
            .get(&location)
            .is_none());
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            0
        );
    });
}

#[test]
fn remove_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![11, 10]
        ));
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![10, 10],
            ),
            Error::<Test>::AgentNotFound
        );
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                bvec![10, 10],
            ),
            BadOrigin
        );
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![11, 10],
            ),
            Error::<Test>::LettingAgentNotActiveInLocation
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        let location = bvec![10, 10];
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );
        let mut letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        if let Some(location_info) = letting_info.locations.get_mut(&location) {
            location_info.assigned_properties = 5;
        }
        let account: AccountId = [0; 32].into();
        LettingInfo::<Test>::insert(account.clone(), letting_info);
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![10, 10],
            ),
            Error::<Test>::LettingAgentActive
        );
    });
}

#[test]
fn letting_agent_propose_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(
            LettingAgentProposal::<Test>::get(0).unwrap().letting_agent,
            [4; 32].into()
        );
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0,
            },
        );
    });
}

#[test]
fn letting_agent_propose_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::letting_agent_propose(RuntimeOrigin::signed([4; 32].into()), 0),
            Error::<Test>::NoObjectFound
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyManagement::letting_agent_propose(RuntimeOrigin::signed([4; 32].into()), 0),
            RealEstateAssetError::<Test>::PropertyNotFinalized
        );
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_noop!(
            PropertyManagement::letting_agent_propose(RuntimeOrigin::signed([4; 32].into()), 0),
            Error::<Test>::LettingAgentProposalOngoing
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyManagement::letting_agent_propose(RuntimeOrigin::signed([4; 32].into()), 0),
            Error::<Test>::LettingAgentAlreadySet
        );
    });
}

#[test]
fn vote_on_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            70,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats {
                yes_voting_power: 0,
                no_voting_power: 0,
            },
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats {
                yes_voting_power: 70,
                no_voting_power: 0,
            },
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats {
                yes_voting_power: 0,
                no_voting_power: 70,
            },
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats {
                yes_voting_power: 30,
                no_voting_power: 70,
            },
        );
        assert_eq!(
            UserLettingAgentVote::<Test>::get(0)
                .unwrap()
                .get(&[1; 32].into())
                .clone(),
            Some(&crate::Vote::No)
        );
    });
}

#[test]
fn vote_on_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
            ),
            Error::<Test>::NoLettingAgentProposed
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
            ),
            Error::<Test>::NoLettingAgentProposed
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
            ),
            Error::<Test>::NoPermission
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
            ),
            Error::<Test>::VotingExpired
        );
    });
}

#[test]
fn finalize_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            1_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            1_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            70,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            100,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            2
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            1,
        ));
        finalize_property_helper([0; 32].into(), [1; 32].into(), 1);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            1
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            2,
        ));
        finalize_property_helper([0; 32].into(), [1; 32].into(), 2);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([3; 32].into()),
            2
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            2,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_eq!(LettingStorage::<Test>::get(1).unwrap(), [4; 32].into());
        assert_eq!(LettingStorage::<Test>::get(2).unwrap(), [3; 32].into());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            2
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([3; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        assert!(LettingAgentProposal::<Test>::get(0).is_none());
        assert_eq!(OngoingLettingAgentVoting::<Test>::get(0), None);
        assert_eq!(UserLettingAgentVote::<Test>::get(0), None);
    });
}

#[test]
fn finalize_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            70,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoLettingAgentProposed
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoLettingAgentProposed
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([3; 32].into()), 0,),
            BadOrigin
        );
        for x in 1..=MaxProperty::get() {
            assert_ok!(Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                1_000,
                100,
                bvec![22, 22],
                false
            ));
            assert_ok!(Marketplace::buy_property_token(
                RuntimeOrigin::signed([0; 32].into()),
                x,
                100,
                1984
            ));
            assert_ok!(Marketplace::claim_property_token(
                RuntimeOrigin::signed([0; 32].into()),
                x
            ));
            assert_ok!(Marketplace::create_spv(
                RuntimeOrigin::signed([5; 32].into()),
                x,
            ));
            finalize_property_helper([0; 32].into(), [0; 32].into(), x);
            assert_ok!(PropertyManagement::letting_agent_propose(
                RuntimeOrigin::signed([4; 32].into()),
                x
            ));
            assert_ok!(PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                x,
                crate::Vote::Yes,
            ));
            let expiry =
                frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
            frame_system::Pallet::<Test>::set_block_number(expiry);
            assert_ok!(PropertyManagement::finalize_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                x
            ));
        }
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            MaxProperty::get()
        );
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ),);
        assert!(LettingStorage::<Test>::get(0).is_some());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            MaxProperty::get() + 1
        );
        assert!(LettingAgentProposal::<Test>::get(0).is_none());
        assert_eq!(OngoingLettingAgentVoting::<Test>::get(0), None);
        assert_eq!(UserLettingAgentVote::<Test>::get(0), None);
    });
}

#[test]
fn distribute_income_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            9_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            50,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3200,
            1984,
        ));
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1984)),
            640
        );
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([2; 32].into(), 0, 1984)),
            960
        );
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([3; 32].into(), 0, 1984)),
            1600
        );
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 1800);
    });
}

#[test]
fn distribute_income_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                200,
                1984
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1984)),
            0
        );
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process_helper([0; 32].into(), [1; 32].into(), 0);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                200,
                1984
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([4; 32].into()),
                0,
                20000,
                1984
            ),
            Error::<Test>::NotEnoughFunds
        );
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([4; 32].into()),
                0,
                2000,
                1
            ),
            Error::<Test>::PaymentAssetNotSupported
        );
    });
}

#[test]
fn claim_income_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            9_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            2200,
            1984,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1337,
        ));
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1984)),
            2200
        );
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1337)),
            1000
        );
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2800);
        assert_eq!(Balances::free_balance(&([4; 32].into())), 4000);
        assert_eq!(
            Balances::free_balance(&PropertyManagement::property_account_id(0)),
            5085
        );
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)),
            2200
        );
        assert_eq!(
            ForeignAssets::balance(1337, &PropertyManagement::property_account_id(0)),
            1000
        );
        assert_ok!(PropertyManagement::claim_income(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1337
        ));
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1984)),
            2200
        );
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)),
            2200
        );
        assert_eq!(
            ForeignAssets::balance(1337, &PropertyManagement::property_account_id(0)),
            0
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 564_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1000);
    });
}

#[test]
fn claim_income_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            900,
            1000,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1000,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(LocalAssets::total_supply(0), 1000);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3200,
            1984,
        ));
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1984)),
            3200
        );
        assert_noop!(
            PropertyManagement::claim_income(RuntimeOrigin::signed([2; 32].into()), 0, 1984),
            Error::<Test>::UserHasNoFundsStored
        );
        assert_noop!(
            PropertyManagement::claim_income(RuntimeOrigin::signed([1; 32].into()), 0, 1),
            Error::<Test>::PaymentAssetNotSupported
        );
    });
}
