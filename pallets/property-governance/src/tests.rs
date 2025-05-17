use crate::{mock::*, Error, Event};
use frame_support::{
	assert_noop, assert_ok,
	traits::{OnFinalize, OnInitialize},
	sp_runtime::Percent,
};

use crate::{Proposals, Challenges, ChallengeRoundsExpiring, OngoingChallengeVotes, OngoingVotes};

use pallet_property_management::{
	LettingStorage, InvestorFunds, 
	LettingInfo
};

use pallet_nft_marketplace::types::{LegalProperty, PaymentAssets};

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

#[test]
fn propose_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[2; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[2; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([2; 32].into()),
			0,
			1000,
			bvec![10, 10]
		));
		assert_eq!(Proposals::<Test>::get(1).unwrap().asset_id, 0);
		assert_eq!(OngoingVotes::<Test>::get(1).is_some(), true);
	});
}

#[test]
fn proposal_with_low_amount_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			500,
			bvec![10, 10]
		));
		System::assert_last_event(Event::ProposalExecuted{ asset_id: 0, amount: 500}.into());
		assert_eq!(Balances::free_balance(&([4; 32].into())), 4900);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
		assert_eq!(OngoingVotes::<Test>::get(1).is_some(), false);
	});
}

#[test]
fn propose_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_noop!(
			PropertyGovernance::propose(
				RuntimeOrigin::signed([2; 32].into()),
				0,
				1000,
				bvec![10, 10]
			),
			Error::<Test>::NoLettingAgentFound
		);
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
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
	});
}

#[test]
fn challenge_against_letting_agent_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_eq!(Challenges::<Test>::get(1).is_some(), true);
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::First);
	});
}


#[test]
fn challenge_against_letting_agent_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_noop!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		), Error::<Test>::NoLettingAgentFound);
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_noop!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([2; 32].into()),
			0
		), Error::<Test>::NoPermission);
		assert_eq!(Challenges::<Test>::get(1).is_some(), false);
	});
}

#[test]
fn vote_on_proposal_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 20, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 10, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([3; 32].into()), 0, 40, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([3; 32].into()),
			1,
			crate::Vote::No
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::No
		));
		assert_eq!(OngoingVotes::<Test>::get(1).unwrap().yes_voting_power, 10);
		assert_eq!(OngoingVotes::<Test>::get(1).unwrap().no_voting_power, 90);
	});
}

#[test]
fn proposal_pass() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([2; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(Proposals::<Test>::get(1).is_some(), true);
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 19_999_000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1_000);
		run_to_block(31);
		System::assert_last_event(Event::ProposalExecuted{ asset_id: 0, amount: 1000}.into());
		assert_eq!(Proposals::<Test>::get(1).is_none(), true);
		assert_eq!(OngoingVotes::<Test>::get(1).is_none(), true);
	});
}

#[test]
fn proposal_pass_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			10000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::No
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(Proposals::<Test>::get(1).is_some(), true);
		run_to_block(31);
		System::assert_last_event(Event::ProposalExecuted{ asset_id: 0, amount: 10000}.into());
		assert_eq!(Proposals::<Test>::get(1).is_none(), true);
		assert_eq!(InvestorFunds::<Test>::get::<(AccountId, u32, PaymentAssets)>(([1; 32].into(), 0, PaymentAssets::USDT)), 0);
	});
}

#[test]
fn proposal_not_pass() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::No
		));
		assert_eq!(Proposals::<Test>::get(1).is_some(), true);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
		run_to_block(31);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
		assert_eq!(Proposals::<Test>::get(1).is_none(), true);
		System::assert_last_event(Event::ProposalRejected{ proposal_id: 1}.into());
	});
}

#[test]
fn proposal_not_pass_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 60, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 40, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			10000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(Proposals::<Test>::get(1).is_some(), true);
		assert_eq!(Proposals::<Test>::get(1).unwrap().amount, 10000);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
		run_to_block(31);
		System::assert_last_event(Event::ProposalThresHoldNotReached{ proposal_id: 1, required_threshold: Percent::from_percent(67)}.into());
		assert_eq!(Proposals::<Test>::get(1).is_none(), true);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
	});
}

#[test]
fn vote_on_proposal_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_noop!(
			PropertyGovernance::vote_on_proposal(
				RuntimeOrigin::signed([1; 32].into()),
				1,
				crate::Vote::Yes
			),
			Error::<Test>::NotOngoing
		);
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_noop!(
			PropertyGovernance::vote_on_proposal(
				RuntimeOrigin::signed([2; 32].into()),
				1,
				crate::Vote::Yes
			),
			Error::<Test>::NoPermission
		);
	});
}

#[test]
fn vote_on_challenge_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 20, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 30, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 10, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([3; 32].into()), 0, 40, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([3; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::No
		));
		assert_eq!(OngoingChallengeVotes::<Test>::get(1, crate::ChallengeState::First).unwrap().yes_voting_power, 60);
		assert_eq!(OngoingChallengeVotes::<Test>::get(1, crate::ChallengeState::First).unwrap().no_voting_power, 40);
	});
}

#[test]
fn challenge_pass() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[1; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[1; 32].into()
		)));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 70, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_eq!(Challenges::<Test>::get(1).unwrap().asset_id, 0);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::No
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(ChallengeRoundsExpiring::<Test>::get(31).len(), 1);
		run_to_block(31);
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Second);
		run_to_block(61);
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Third);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		run_to_block(91);
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Fourth);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([0; 32].into())
				.unwrap()
				.locations
				.len(),
			1
		);
		run_to_block(121);
		assert_eq!(LettingStorage::<Test>::get(0).is_none(), true);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([0; 32].into())
				.unwrap()
				.locations
				.len(),
			1
		);
		assert_eq!(Challenges::<Test>::get(1).is_none(), true);
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [1; 32].into());
	});
}

#[test]
fn challenge_does_not_pass() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[1; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[1; 32].into()
		)));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			4_000,
			250,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 75, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 175, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_eq!(Challenges::<Test>::get(1).unwrap().asset_id, 0);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(ChallengeRoundsExpiring::<Test>::get(31).len(), 1);
		run_to_block(31);
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Second);
		run_to_block(61);
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Third);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		run_to_block(91);
		System::assert_last_event(Event::ChallengeThresHoldNotReached{ challenge_id: 1, required_threshold: Percent::from_percent(51), challenge_state: crate::ChallengeState::Third}.into());
		assert_eq!(Challenges::<Test>::get(1).is_none(), true);
	});
}


#[test]
fn challenge_pass_only_one_agent() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![9, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![9, 10],
			[1; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[1; 32].into()
		)));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 70, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_eq!(Challenges::<Test>::get(1).unwrap().asset_id, 0);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(ChallengeRoundsExpiring::<Test>::get(31).len(), 1);
		run_to_block(31);
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Second);
		run_to_block(61);
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Third);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		run_to_block(91);
		assert_eq!(Challenges::<Test>::get(1).unwrap().state, crate::ChallengeState::Fourth);
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([2; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
		run_to_block(121);
		assert_eq!(LettingStorage::<Test>::get(0).is_none(), true);
		assert_eq!(Challenges::<Test>::get(1).is_none(), true);
	});
}

#[test]
fn challenge_not_pass() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_noop!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		), Error::<Test>::NoLettingAgentFound);
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::No
		));
		assert_eq!(Challenges::<Test>::get(1).is_some(), true);
		run_to_block(31);
		System::assert_last_event(Event::ChallengeRejected{ challenge_id: 1, challenge_state: crate::ChallengeState::First}.into());
		assert_eq!(Challenges::<Test>::get(1).is_none(), true);
	});
}

#[test]
fn vote_on_challenge_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_noop!(
			PropertyGovernance::vote_on_letting_agent_challenge(
				RuntimeOrigin::signed([1; 32].into()),
				1,
				crate::Vote::Yes
			),
			Error::<Test>::NotOngoing
		);
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_ok!(PropertyGovernance::challenge_against_letting_agent(
			RuntimeOrigin::signed([1; 32].into()),
			0
		));
		assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_noop!(
			PropertyGovernance::vote_on_letting_agent_challenge(
				RuntimeOrigin::signed([2; 32].into()),
				1,
				crate::Vote::Yes
			),
			Error::<Test>::NoPermission
		);
	});
}

#[test]
fn different_proposals() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::signed([6; 32].into()), 30, 3));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::signed([6; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::signed([6; 32].into()), 0, [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			5_000,
			200,
			bvec![22, 22],
			false
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 60, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 60, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([3; 32].into()), 0, 80, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_ok!(NftMarketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::signed([6; 32].into()),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			3000,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1000,
			bvec![10, 10]
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			crate::Vote::Yes
		));
		assert_eq!(Proposals::<Test>::get(1).is_some(), true);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
		run_to_block(31);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
		assert_eq!(Proposals::<Test>::get(1).is_none(), true);
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			3000,
			bvec![10, 10]
		));
		assert_eq!(Proposals::<Test>::get(2).is_some(), true);
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			2,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([2; 32].into()),
			2,
			crate::Vote::Yes
		));
		run_to_block(61);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			3000,
			bvec![10, 10]
		));
		assert_eq!(Proposals::<Test>::get(3).is_some(), true);
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			3,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([2; 32].into()),
			3,
			crate::Vote::No
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([3; 32].into()),
			3,
			crate::Vote::Yes
		));
		run_to_block(91);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1700,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			300,
			PaymentAssets::USDC,
		));
		assert_ok!(PropertyGovernance::propose(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1500,
			bvec![10, 10]
		));
		assert_eq!(Proposals::<Test>::get(4).is_some(), true);
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([1; 32].into()),
			4,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([2; 32].into()),
			4,
			crate::Vote::Yes
		));
		assert_ok!(PropertyGovernance::vote_on_proposal(
			RuntimeOrigin::signed([3; 32].into()),
			4,
			crate::Vote::No
		));
		run_to_block(121);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 300);
		assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 4700);
		assert_eq!(ForeignAssets::balance(1337, &[4; 32].into()), 4700);
		assert_eq!(ForeignAssets::balance(1337, &PropertyGovernance::property_account_id(0)), 300);
	});
}  