use crate::{mock::*, Error, Event};
use frame_support::{
    assert_noop, assert_ok,
    sp_runtime::{traits::BadOrigin, Percent, Permill},
    traits::{
        fungible::InspectHold,
        fungibles::{Inspect, InspectFreeze, InspectHold as FungiblesInspectHold},
        OnFinalize, OnInitialize,
    },
};

use crate::{
    AssetLettingChallenge, AssetProposal, ChallengeRoundsExpiring, Challenges,
    OngoingChallengeVotes, OngoingProposalVotes, OngoingSaleProposalVotes, PropertySale,
    PropertySaleFunds, Proposals, Reserve, SaleAuctions, SaleProposals, UserProposalVote,
    UserSaleProposalVote, VoteRecord, UserChallengeVote, AssetSaleProposal,
};

use pallet_property_management::{InvestorFunds, LettingInfo, LettingStorage};

use pallet_marketplace::types::LegalProperty;

use pallet_real_estate_asset::{Error as RealEstateAssetError, PropertyAssetInfo, PropertyOwner};

use pallet_regions::{RealEstateLawyer, RegionIdentifier};

use primitives::MarketplaceFreezeReason;

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            PropertyGovernance::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        PropertyGovernance::on_initialize(System::block_number());
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

fn listing_process() {
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
        pallet_xcavate_whitelist::Role::RealEstateDeveloper
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
}

fn setting_letting_agent(agent: AccountId, voters: Vec<(AccountId, u32)>) {
    assert_ok!(PropertyManagement::add_letting_agent(
        RuntimeOrigin::signed(agent.clone()),
        3,
        bvec![10, 10],
    ));
    assert_ok!(PropertyManagement::letting_agent_propose(
        RuntimeOrigin::signed(agent.clone()),
        0
    ));
    for voter in &voters {
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed(voter.0.clone()),
            0,
            pallet_property_management::Vote::Yes,
            voter.1
        ));
    }
    let expiry = frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
    frame_system::Pallet::<Test>::set_block_number(expiry);
    assert_ok!(PropertyManagement::finalize_letting_agent(
        RuntimeOrigin::signed(voters[0].0.clone()),
        0,
    ));
    assert_eq!(LettingStorage::<Test>::get(0).unwrap(), agent);
}

fn lawyer_process(accounts: Vec<(AccountId, u32)>) {
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
    for account in accounts {
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed(account.0),
            0,
            pallet_marketplace::types::Vote::Yes,
            account.1,
        ));
    }
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
}

#[test]
fn propose_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
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
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).is_some(), true);
        assert_eq!(AssetProposal::<Test>::get(0).unwrap(), 0);
    });
}

#[test]
fn proposal_with_low_amount_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
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
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            500,
            bvec![10, 10]
        ));
        System::assert_last_event(
            Event::ProposalExecuted {
                asset_id: 0,
                amount: 500,
            }
            .into(),
        );
        assert_eq!(Balances::free_balance(&([4; 32].into())), 4000);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).is_some(), false);
    });
}

#[test]
fn propose_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
            1984
        ));
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                1000,
                bvec![10, 10]
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
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
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                1000,
                bvec![10, 10]
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                1000,
                bvec![10, 10]
            ),
            Error::<Test>::ProposalOngoing
        );
    });
}

#[test]
fn challenge_against_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(AssetLettingChallenge::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingChallengeVotes::<Test>::get(0).is_some(), true);
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
    });
}

#[test]
fn challenge_against_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([2; 32].into()),
                0
            ),
            Error::<Test>::NoPermission
        );
        assert_eq!(Challenges::<Test>::get(0).is_some(), false);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::ChallengeAlreadyOngoing
        );
    });
}

#[test]
fn vote_on_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
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
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            40,
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
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            50
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            50
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::Yes,
                asset_id: 0,
                power: 50,
            }
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(
            OngoingProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            10
        );
        assert_eq!(
            OngoingProposalVotes::<Test>::get(0)
                .unwrap()
                .no_voting_power,
            70
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            30
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::No,
                asset_id: 0,
                power: 30,
            }
        );
    });
}

#[test]
fn proposal_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([2; 32].into()),
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
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
            pallet_marketplace::types::Vote::Yes,
            100
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_000);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 19_999_000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1_000
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(
            Event::ProposalExecuted {
                asset_id: 0,
                amount: 1000,
            }
            .into(),
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn proposal_pass_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            10000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            100
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(
            Event::ProposalExecuted {
                asset_id: 0,
                amount: 10000,
            }
            .into(),
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(
            InvestorFunds::<Test>::get::<(AccountId, u32, u32)>(([1; 32].into(), 0, 1984)),
            0
        );
    });
}

#[test]
fn proposal_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            100
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1000
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1000
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        System::assert_last_event(Event::ProposalRejected { proposal_id: 0 }.into());
    });
}

#[test]
fn proposal_not_pass_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 60)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            10000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(Proposals::<Test>::get(0).unwrap().amount, 10000);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1000
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(
            Event::ProposalThresHoldNotReached {
                proposal_id: 0,
                required_threshold: Percent::from_percent(67),
            }
            .into(),
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1000
        );
    });
}

#[test]
fn proposal_not_pass_3() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 60)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(Proposals::<Test>::get(0).unwrap().amount, 1000);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1000
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(
            Event::ProposalRejected {
                proposal_id: 0,
            }
            .into(),
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1000
        );
    });
}

#[test]
fn vote_on_proposal_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NoPermission
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NotOngoing
        );
    });
}

#[test]
fn unfreeze_proposal_token_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
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
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            40,
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
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            50
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            50
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            30
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::No,
                asset_id: 0,
                power: 30,
            }
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert!(UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none());
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            0
        );
    });
}

#[test]
fn unfreeze_proposal_token_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
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
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            40,
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
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            50
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            50
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            30
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::No,
                asset_id: 0,
                power: 30,
            }
        );
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_token(RuntimeOrigin::signed([3; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
    });
}

#[test]
fn vote_on_challenge_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            40,
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
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            30
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::Yes,
                asset_id: 0,
                power: 30,
            }
        );
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            40
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::No,
                asset_id: 0,
                power: 40,
            }
        );
        assert_eq!(
            OngoingChallengeVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            30
        );
        assert_eq!(
            OngoingChallengeVotes::<Test>::get(0)
                .unwrap()
                .no_voting_power,
            40
        );
    });
}

#[test]
fn challenge_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            70,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 30), ([2; 32].into(), 70)]);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            50
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        frame_system::Pallet::<Test>::set_block_number(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            70
        ));
        assert_eq!(AssetLettingChallenge::<Test>::get(0), Some(0));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_eq!(ChallengeRoundsExpiring::<Test>::get(expiry).len(), 1);
        run_to_block(expiry);
        assert_eq!(AssetLettingChallenge::<Test>::get(0), None);
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .unwrap(),
            &1u8
        );
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            70
        ));
        assert_eq!(Balances::total_balance_on_hold(&[0; 32].into()), 900);
        assert_eq!(Balances::total_issuance(), 57_509_901);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .unwrap(),
            &2u8
        );
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Balances::total_balance_on_hold(&[0; 32].into()), 800);
        assert_eq!(Balances::total_issuance(), 57_509_801);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            70
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .len(),
            1
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .unwrap(),
            &2u8
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        run_to_block(211);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .is_none(),
            true
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        assert_eq!(LettingStorage::<Test>::get(0).is_none(), true);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .len(),
            1
        );
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [1; 32].into());
    });
}

#[test]
fn challenge_does_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            4_000,
            250,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            75,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            175,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 75), ([2; 32].into(), 75)]);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            75
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            75
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        frame_system::Pallet::<Test>::set_block_number(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            75
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            175
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_eq!(ChallengeRoundsExpiring::<Test>::get(expiry).len(), 1);
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            75
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
/*         System::assert_last_event(
            Event::ChallengeThresHoldNotReached {
                asset_id: 0,
                required_threshold: Percent::from_percent(51),
            }
            .into(),
        ); */
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn challenge_pass_only_one_agent() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![9, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            bvec![9, 10],
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            70,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 30), ([2; 32].into(), 70)]);
        assert_ok!(PropertyManagement::letting_agent_propose(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            70
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        frame_system::Pallet::<Test>::set_block_number(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            70
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_eq!(ChallengeRoundsExpiring::<Test>::get(expiry).len(), 1);
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            70
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            70
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        run_to_block(211);
        assert_eq!(LettingStorage::<Test>::get(0).is_none(), true);
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn challenge_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            100
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ChallengeRejected { asset_id: 0 }.into());
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn challenge_not_pass2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            5_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            200,
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
        lawyer_process(vec![([1; 32].into(), 200)]);
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 150)]);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ChallengeRejected { asset_id: 0 }.into());
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn vote_on_challenge_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_noop!(
            PropertyGovernance::vote_on_letting_agent_challenge(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 100)]);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_noop!(
            PropertyGovernance::vote_on_letting_agent_challenge(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
                10
            ),
            Error::<Test>::NoPermission
        );
    });
}

#[test]
fn unfreeze_challenge_token_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            40,
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
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            30
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::Yes,
                asset_id: 0,
                power: 30,
            }
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert!(UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).is_none());
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            0
        );
    });
}

#[test]
fn unfreeze_challenge_token_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            40,
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
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_noop!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ), Error::<Test>::NoFrozenAmount);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            30
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::Yes,
                asset_id: 0,
                power: 30,
            }
        );
        assert_noop!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ), Error::<Test>::VotingStillOngoing);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_noop!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
        ), Error::<Test>::NoFrozenAmount);
        assert_ok!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_noop!(PropertyGovernance::unfreeze_challenge_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ), Error::<Test>::NoFrozenAmount);
    });
}

#[test]
fn different_proposals() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            5_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            80,
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
        lawyer_process(vec![([1; 32].into(), 60), ([2; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 60), ([2; 32].into(), 60)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            1984,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            3000
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            3000
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(1).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            3000
        );
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(2).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            3000
        );
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1700,
            1984,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            300,
            1337,
        ));
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1500,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(3).is_some(), true);
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            2,
        ));
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([2; 32].into()),
            2,
        ));
        assert_ok!(PropertyGovernance::unfreeze_proposal_token(
            RuntimeOrigin::signed([3; 32].into()),
            2,
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::No,
            80
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 300);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            4700
        );
        assert_eq!(ForeignAssets::balance(1337, &[4; 32].into()), 4700);
        assert_eq!(
            ForeignAssets::balance(1337, &PropertyGovernance::property_account_id(0)),
            300
        );
    });
}

#[test]
fn propose_property_sale_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), true);
        assert_eq!(AssetSaleProposal::<Test>::get(0).unwrap(), 0);
    });
}

#[test]
fn propose_property_sale_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
        assert_noop!(
            PropertyGovernance::propose_property_sale(RuntimeOrigin::signed([1; 32].into()), 0),
            RealEstateAssetError::<Test>::PropertyNotFound
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
            PropertyGovernance::propose_property_sale(RuntimeOrigin::signed([1; 32].into()), 0),
            RealEstateAssetError::<Test>::PropertyNotFinalized
        );
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_noop!(
            PropertyGovernance::propose_property_sale(RuntimeOrigin::signed([6; 32].into()), 0),
            Error::<Test>::NoPermission
        );
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_noop!(
            PropertyGovernance::propose_property_sale(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::PropertySaleProposalOngoing
        );
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).is_some(), true);
        assert_noop!(
            PropertyGovernance::propose_property_sale(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::SaleOngoing
        );
    });
}

#[test]
fn vote_on_property_sale_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            35,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            25,
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
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 35)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 35)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            35
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            25
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            65
        );
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .no_voting_power,
            35
        );
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::Yes,
                asset_id: 0,
                power: 40,
            }
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SaleVoting,
                &[1; 32].into()
            ),
            40
        );
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            25
        );
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .no_voting_power,
            70
        );
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::No,
                asset_id: 0,
                power: 35,
            }
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SaleVoting,
                &[1; 32].into()
            ),
            35
        );
    });
}

#[test]
fn vote_on_property_sale_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 100)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 100)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_noop!(
            PropertyGovernance::vote_on_property_sale(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_noop!(
            PropertyGovernance::vote_on_property_sale(
                RuntimeOrigin::signed([6; 32].into()),
                0,
                crate::Vote::Yes,
                10
            ),
            Error::<Test>::NoPermission
        );
    });
}

#[test]
fn unfreeze_sale_proposal_token_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            35,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            25,
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
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 25)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 25)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            35
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            25
        ));
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord {
                vote: crate::Vote::Yes,
                asset_id: 0,
                power: 40,
            }
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SaleVoting,
                &[1; 32].into()
            ),
            40
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SaleVoting,
                &[1; 32].into()
            ),
            0
        );
        assert!(UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none());
        assert!(UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).is_some());
    });
}

#[test]
fn unfreeze_sale_proposal_token_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            35,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            25,
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
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 35)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 35)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_noop!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ), Error::<Test>::NoFrozenAmount);
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            35
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            25
        ));
        assert_noop!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ), Error::<Test>::VotingStillOngoing);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ), Error::<Test>::NoFrozenAmount);
    });
}

#[test]
fn auction_starts() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            5,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 60)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            5
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            95
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), false);
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), false);
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().highest_bidder, None);
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 0);
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().reserve, None);
    });
}

#[test]
fn proposal_does_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            50,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 50), ([2; 32].into(), 15)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 50), ([2; 32].into(), 15)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            50
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            15
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            85
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).is_some(), false);
        System::assert_last_event(Event::PropertySaleProposalRejected { asset_id: 0 }.into());
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), false);
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), false);
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        assert_eq!(SaleAuctions::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn proposal_does_not_pass2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            5_000,
            200,
            bvec![22, 22],
            false
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            100,
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
            70,
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
        lawyer_process(vec![([1; 32].into(), 100), ([2; 32].into(), 30)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 100), ([2; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            50
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            150
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).is_some(), false);
        System::assert_last_event(Event::PropertySaleProposalRejected { asset_id: 0 }.into());
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), false);
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), false);
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        assert_eq!(SaleAuctions::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn bid_on_sale_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            5,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 60)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            5
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            95
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).is_some(), true);
        assert_eq!(PropertySale::<Test>::get(0).unwrap().price, None);
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().highest_bidder, None);
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 0);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            10,
            1984
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[4; 32].into()),
            1
        );
        assert_eq!(
            SaleAuctions::<Test>::get(0).unwrap().highest_bidder,
            Some([4; 32].into())
        );
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 10);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            20,
            1984
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[4; 32].into()),
            2
        );
        assert_eq!(
            SaleAuctions::<Test>::get(0).unwrap().highest_bidder,
            Some([4; 32].into())
        );
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 20);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([5; 32].into()),
            0,
            2000,
            1984
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[5; 32].into()),
            200
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[4; 32].into()),
            0
        );
        assert_eq!(
            SaleAuctions::<Test>::get(0).unwrap().highest_bidder,
            Some([5; 32].into())
        );
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 2000);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            1984
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[5; 32].into()),
            0
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[4; 32].into()),
            300
        );
        assert_eq!(
            SaleAuctions::<Test>::get(0).unwrap().highest_bidder,
            Some([4; 32].into())
        );
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 3000);
        assert_eq!(
            SaleAuctions::<Test>::get(0).unwrap().reserve,
            Some(Reserve {
                payment_asset: 1984,
                amount: 300
            })
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).unwrap().price, Some(3000));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer,
            Some([4; 32].into())
        );
    });
}

#[test]
fn bid_on_sale_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            5,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 60)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            5
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            95
        );
        assert_noop!(
            PropertyGovernance::bid_on_sale(RuntimeOrigin::signed([4; 32].into()), 0, 10, 1984),
            Error::<Test>::NoOngoingAuction,
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::bid_on_sale(RuntimeOrigin::signed([7; 32].into()), 0, 10, 1337),
            BadOrigin
        );
        assert_noop!(
            PropertyGovernance::bid_on_sale(RuntimeOrigin::signed([4; 32].into()), 0, 10, 1),
            Error::<Test>::PaymentAssetNotSupported,
        );
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().highest_bidder, None);
        assert_eq!(SaleAuctions::<Test>::get(0).unwrap().price, 0);
    });
}

#[test]
fn lawyer_claim_sale_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer.unwrap(),
            [7; 32].into()
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().price.unwrap(),
            300_000
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().reserve.unwrap(),
            Reserve {
                payment_asset: 1984,
                amount: 30_000
            }
        );
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_lawyer.unwrap(),
            [10; 32].into()
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_lawyer_costs,
            1_000
        );
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer_costs,
            1_000
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            1
        );
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([11; 32].into())
                .unwrap()
                .active_cases,
            1
        );
    });
}

#[test]
fn lawyer_claim_sale_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([6; 32].into()),
            RegionIdentifier::India
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([6; 32].into()),
            4,
            pallet_regions::Vote::Yes
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([6; 32].into()),
            4,
            100_000
        ));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([6; 32].into()),
            4,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
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
            pallet_marketplace::types::Vote::Yes,
            55
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::NotForSale
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                1,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::AssetNotFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([12; 32].into()),
            4,
        ));
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([12; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::NoPermissionInRegion
        );
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::PriceNotSet
        );
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalSale::BuyerSide,
                1_000
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::LawyerJobTaken
        );
    });
}

#[test]
fn lawyer_claim_sale_fails_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
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
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([6; 32].into()),
            RegionIdentifier::France
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([6; 32].into()),
            2,
            pallet_regions::Vote::Yes
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([6; 32].into()),
            2,
            100_000
        ));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([6; 32].into()),
            2,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
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
            pallet_marketplace::types::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            25
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 25)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            25
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            75
        );
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .no_voting_power,
            25
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
        ));
        assert_eq!(PropertySale::<Test>::get(0).is_some(), false);
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            BadOrigin
        );
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                1,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::AssetNotFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([12; 32].into()),
            2,
        ));
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([12; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::NoPermissionInRegion
        );
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::NotForSale
        );
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            25
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_eq!(PropertySale::<Test>::get(0).is_some(), true);
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                1_000
            ),
            Error::<Test>::PriceNotSet
        );
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::lawyer_claim_sale(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalSale::SpvSide,
                5_000
            ),
            Error::<Test>::CostsTooHigh
        );
    });
}

#[test]
fn lawyer_confirm_sale_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            2_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            2_500
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer_costs,
            2_000
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_lawyer_costs,
            2_500
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Pending
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Approved
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, true);
    });
}

#[test]
fn lawyer_confirm_sale_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            2_500
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            2_500
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Pending
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().second_attempt, false);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Pending
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_status,
            crate::DocumentStatus::Pending
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().second_attempt, true);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_status,
            crate::DocumentStatus::Approved
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, true);
    });
}

#[test]
fn lawyer_confirm_sale_works_deny() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_lawyer.unwrap(),
            [10; 32].into()
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            false
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[7; 32].into()),
            30_000
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[7; 32].into()),
            0
        );
        assert_eq!(PropertySale::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn lawyer_confirm_sale_works_deny_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().buyer_lawyer.unwrap(),
            [10; 32].into()
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            false
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[7; 32].into()),
            30_000
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[7; 32].into()),
            0
        );
        assert_eq!(PropertySale::<Test>::get(0).is_none(), true);
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            0
        );
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([11; 32].into())
                .unwrap()
                .active_cases,
            0
        );
    });
}

#[test]
fn lawyer_confirm_sale_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .yes_voting_power,
            90
        );
        assert_eq!(
            OngoingSaleProposalVotes::<Test>::get(0)
                .unwrap()
                .no_voting_power,
            10
        );
        assert_noop!(
            PropertyGovernance::lawyer_confirm_sale(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                true
            ),
            Error::<Test>::NotForSale
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::lawyer_confirm_sale(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                true
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_noop!(
            PropertyGovernance::lawyer_confirm_sale(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                true
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyGovernance::lawyer_confirm_sale(
                RuntimeOrigin::signed([11; 32].into()),
                1,
                true
            ),
            Error::<Test>::NotForSale
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_noop!(
            PropertyGovernance::lawyer_confirm_sale(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                false
            ),
            Error::<Test>::SaleAlreadyConfirmed
        );
        assert_noop!(
            PropertyGovernance::lawyer_confirm_sale(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                true
            ),
            Error::<Test>::SaleAlreadyConfirmed
        );
    });
}

#[test]
fn finalize_sale_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, true);
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, false);
        assert_ok!(PropertyGovernance::finalize_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            1337
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, true);
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1337), 264_000);
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1984), 30_000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            30_000
        );
        assert_eq!(
            ForeignAssets::balance(1337, &PropertyGovernance::property_account_id(0)),
            264_000
        );
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 1_000);
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            2_000
        );
        assert_eq!(ForeignAssets::balance(1337, &[6; 32].into()), 2_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 731_000);
    });
}

#[test]
fn finalize_sale_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            3_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            3_000
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::finalize_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            1337
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, true);
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1337), 264_000);
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1984), 30_000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            30_000
        );
        assert_eq!(
            ForeignAssets::balance(1337, &PropertyGovernance::property_account_id(0)),
            264_000
        );
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 3_000);
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            0
        );
        assert_eq!(ForeignAssets::balance(1337, &[6; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 733_000);
    });
}

#[test]
fn finalize_sale_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            55,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            35,
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
        lawyer_process(vec![([1; 32].into(), 55)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 55)]);
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            55
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            35
        ));
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([10; 32].into()), 0, 1984),
            Error::<Test>::NotForSale,
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([4; 32].into()), 0, 1984),
            BadOrigin
        );
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([10; 32].into()), 0, 1984),
            Error::<Test>::SaleHasNotBeenApproved,
        );
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, true);
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([11; 32].into()), 0, 1984),
            Error::<Test>::NoPermission,
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, false);
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([10; 32].into()), 0, 1),
            Error::<Test>::PaymentAssetNotSupported,
        );
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([10; 32].into()), 0, 1984),
            Error::<Test>::NotEnoughFunds,
        );
        assert_ok!(PropertyGovernance::finalize_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            1337
        ));
        assert_noop!(
            PropertyGovernance::finalize_sale(RuntimeOrigin::signed([10; 32].into()), 0, 1984),
            Error::<Test>::AlreadyFinalized,
        );
    });
}

#[test]
fn claim_sale_funds_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            90,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 90)]);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 90)]);
        assert_ok!(PropertyManagement::unfreeze_letting_voting_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            90
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::finalize_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            1337
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, true);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 564_000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            30_000
        );
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1337), 264_000);
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1984), 30_000);
        assert_eq!(
            LocalAssets::balance(0, &PropertyGovernance::property_account_id(0)),
            0
        );
        assert_ok!(PropertyGovernance::claim_sale_funds(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1984
        ));
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1337), 29_400);
        assert_eq!(PropertySaleFunds::<Test>::get(0, 1984), 0);
        assert_eq!(
            LocalAssets::balance(0, &PropertyGovernance::property_account_id(0)),
            90
        );
        assert_eq!(
            ForeignAssets::balance(1337, &PropertyGovernance::property_account_id(0)),
            29_400
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 594_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 234_600);
        assert_eq!(LocalAssets::total_issuance(0), 100);
        assert_ok!(PropertyGovernance::claim_sale_funds(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            1337
        ));
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            0
        );
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 29_400);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_046_000);
        assert_eq!(Nfts::owner(0, 0).is_none(), true);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 0);
        assert_eq!(LocalAssets::total_issuance(0), 0);
        assert_eq!(PropertySale::<Test>::get(0).is_none(), true);
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            0
        );
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([11; 32].into())
                .unwrap()
                .active_cases,
            0
        );
    });
}

#[test]
fn claim_sale_funds_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        listing_process();
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
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            90,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        lawyer_process(vec![([1; 32].into(), 90)]);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 90)]);
        assert_ok!(PropertyManagement::unfreeze_letting_voting_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::propose_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            90
        ));
        assert_ok!(PropertyGovernance::vote_on_property_sale(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_noop!(
            PropertyGovernance::claim_sale_funds(RuntimeOrigin::signed([1; 32].into()), 0, 1984),
            Error::<Test>::NotForSale,
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_sale_proposal_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(PropertySale::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingSaleProposalVotes::<Test>::get(0).is_some(), false);
        assert_eq!(SaleProposals::<Test>::get(0).is_some(), false);
        assert_eq!(
            UserSaleProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            false
        );
        assert_ok!(PropertyGovernance::bid_on_sale(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            300_000,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertySaleVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalSale::SpvSide,
            1_000
        ));
        assert_ok!(PropertyGovernance::lawyer_claim_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalSale::BuyerSide,
            1_000
        ));
        assert_eq!(
            PropertySale::<Test>::get(0).unwrap().spv_lawyer.unwrap(),
            [11; 32].into()
        );
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, false);
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true
        ));
        assert_ok!(PropertyGovernance::lawyer_confirm_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().lawyer_approved, true);
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, false);
        assert_noop!(
            PropertyGovernance::claim_sale_funds(RuntimeOrigin::signed([1; 32].into()), 0, 1984),
            Error::<Test>::SaleNotFinalized,
        );
        assert_ok!(PropertyGovernance::finalize_sale(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            1337
        ));
        assert_eq!(PropertySale::<Test>::get(0).unwrap().finalized, true);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 564_000);
        assert_noop!(
            PropertyGovernance::claim_sale_funds(RuntimeOrigin::signed([6; 32].into()), 0, 1984),
            Error::<Test>::NoFundsToClaim,
        );
        assert_ok!(PropertyGovernance::claim_sale_funds(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1984
        ));
        assert_noop!(
            PropertyGovernance::claim_sale_funds(RuntimeOrigin::signed([1; 32].into()), 0, 1984),
            Error::<Test>::NoFundsToClaim,
        );
    });
}
