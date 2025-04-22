use crate::{mock::*, Error};
use frame_support::traits::Currency;
use frame_support::BoundedVec;
use frame_support::{assert_noop, assert_ok};

use crate::{PropertyReserve, LettingStorage, LettingInfo, LettingAgentLocations, InvestorFunds};

use sp_runtime::TokenError;

use pallet_nft_marketplace::{LegalProperty, PaymentAssets};

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

#[test]
fn add_letting_agent_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
		let location: BoundedVec<u8, Postcode> = bvec![10, 10];
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap().locations[0],
			location
		);
	});
}

#[test]
fn add_letting_agent_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			PropertyManagement::add_letting_agent(
				RuntimeOrigin::root(),
				0,
				bvec![10, 10],
				[0; 32].into(),
			),
			Error::<Test>::RegionUnknown
		);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_noop!(
			PropertyManagement::add_letting_agent(
				RuntimeOrigin::root(),
				0,
				bvec![10, 10],
				[0; 32].into(),
			),
			Error::<Test>::LocationUnknown
		);
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
		assert_noop!(
			PropertyManagement::add_letting_agent(
				RuntimeOrigin::root(),
				0,
				bvec![10, 10],
				[0; 32].into(),
			),
			Error::<Test>::LettingAgentExists
		);
	});
}

#[test]
fn let_letting_agent_deposit() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_eq!(
			LettingAgentLocations::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				0,
				bvec![10, 10]
			)
			.contains(&[0; 32].into()),
			false
		);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap().deposited,
			false
		);
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_eq!(
			LettingAgentLocations::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				0,
				bvec![10, 10]
			)
			.contains(&[0; 32].into()),
			true
		);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap().deposited,
			true
		);
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
	});
}

#[test]
fn let_letting_agent_deposit_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_noop!(
			PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed([0; 32].into())),
			Error::<Test>::AlreadyDeposited
		);
		assert_noop!(
			PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed([1; 32].into())),
			Error::<Test>::NoPermission
		);
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
		for x in 1..100 {
			assert_ok!(PropertyManagement::add_letting_agent(
				RuntimeOrigin::root(),
				0,
				bvec![10, 10],
				[x; 32].into(),
			));
			Balances::make_free_balance_be(&[x; 32].into(), 200);
			assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
				[x; 32].into()
			)));
		}
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[100; 32].into(),
		));
		Balances::make_free_balance_be(&[100; 32].into(), 200);
		assert_noop!(
			PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed([100; 32].into())),
			Error::<Test>::TooManyLettingAgents
		);
	});
}

#[test]
fn let_letting_agent_deposit_not_enough_funds() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[5; 32].into(),
		));
		assert_noop!(
			PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed([5; 32].into())),
			TokenError::FundsUnavailable
		);
	});
}

#[test]
fn add_letting_agent_to_location_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![9, 10]));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![9, 10],
			[0; 32].into(),
		));
		assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_ok!(PropertyManagement::add_letting_agent_to_location(
			RuntimeOrigin::root(),
			bvec![10, 10],
			[0; 32].into()
		));
		assert_eq!(
			LettingAgentLocations::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				0,
				bvec![9, 10]
			)
			.contains(&[0; 32].into()),
			true
		);
		assert_eq!(
			LettingAgentLocations::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				0,
				bvec![10, 10]
			)
			.contains(&[0; 32].into()),
			true
		);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([0; 32].into())
				.unwrap()
				.locations
				.len(),
			2
		);
	});
}

#[test]
fn add_letting_agent_to_location_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			PropertyManagement::add_letting_agent_to_location(
				RuntimeOrigin::root(),
				bvec![10, 10],
				[0; 32].into()
			),
			Error::<Test>::NoLettingAgentFound
		);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![9, 10]));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
		assert_noop!(
			PropertyManagement::add_letting_agent_to_location(
				RuntimeOrigin::root(),
				bvec![10, 10],
				[0; 32].into()
			),
			Error::<Test>::NotDeposited
		);
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_noop!(
			PropertyManagement::add_letting_agent_to_location(
				RuntimeOrigin::root(),
				bvec![5, 10],
				[0; 32].into()
			),
			Error::<Test>::LocationUnknown
		);
		assert_noop!(
			PropertyManagement::add_letting_agent_to_location(
				RuntimeOrigin::root(),
				bvec![10, 10],
				[0; 32].into()
			),
			Error::<Test>::LettingAgentInLocation
		);
	});
}

#[test]
fn set_letting_agent_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			1_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 1, 100, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			1_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 2, 100, PaymentAssets::USDT));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[2; 32].into(),
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[3; 32].into(),
		));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[2; 32].into()
		)));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[3; 32].into()
		)));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([3; 32].into()), 1));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 2));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			1_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 3, 100, PaymentAssets::USDT));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([2; 32].into()), 3));
		assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
		assert_eq!(LettingStorage::<Test>::get(1).unwrap(), [3; 32].into());
		assert_eq!(LettingStorage::<Test>::get(2).unwrap(), [4; 32].into());
		assert_eq!(LettingStorage::<Test>::get(3).unwrap(), [2; 32].into());
		assert_eq!(
			LettingAgentLocations::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				0,
				bvec![10, 10]
			)
			.len(),
			3
		);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([2; 32].into())
				.unwrap()
				.assigned_properties
				.len(),
			2
		);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([3; 32].into())
				.unwrap()
				.assigned_properties
				.len(),
			1
		);
		assert_eq!(
			LettingInfo::<Test>::get::<AccountId>([4; 32].into())
				.unwrap()
				.assigned_properties
				.len(),
			1
		);
	});
}

#[test]
fn set_letting_agent_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[0; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[0; 32].into()
		)));
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
		assert_noop!(
			PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0),
			Error::<Test>::NoObjectFound
		);
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			100,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_noop!(
			PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0),
			Error::<Test>::LettingAgentAlreadySet
		);
		for x in 1..100 {
			assert_ok!(NftMarketplace::list_object(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				bvec![10, 10],
				1_000,
				100,
				bvec![22, 22]
			));
			assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [(x + 1); 32].into()));
			Balances::make_free_balance_be(&[x; 32].into(), 100_000);
			assert_ok!(ForeignAssets::mint(
				RuntimeOrigin::signed([0; 32].into()),
				1984.into(),
				sp_runtime::MultiAddress::Id([(x + 1); 32].into()),
				1_000_000,
			));
			assert_ok!(NftMarketplace::buy_token(
				RuntimeOrigin::signed([(x + 1); 32].into()),
				(x as u32).into(),
				100,
				PaymentAssets::USDT
			));
			assert_ok!(PropertyManagement::set_letting_agent(
				RuntimeOrigin::signed([0; 32].into()),
				x.into()
			));
		}
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 100, 100, PaymentAssets::USDT));
		assert_noop!(
			PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 100),
			Error::<Test>::TooManyAssignedProperties
		);
	});
}

#[test]
fn set_letting_agent_no_letting_agent() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 20, PaymentAssets::USDT));
		assert_noop!(
			PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0),
			Error::<Test>::AgentNotFound
		);
	});
}

#[test]
fn distribute_income_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::root(), [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::root(), [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			9_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 20, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([2; 32].into()), 0, 30, PaymentAssets::USDT));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([3; 32].into()), 0, 50, PaymentAssets::USDT));
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
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			3200,
			PaymentAssets::USDT,
		));
		assert_eq!(PropertyReserve::<Test>::get(0).total, 3000);
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([1; 32].into(), PaymentAssets::USDT), 40);
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([2; 32].into(), PaymentAssets::USDT), 60);
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([3; 32].into(), PaymentAssets::USDT), 100);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 1800);
	});
}

#[test]
fn distribute_income_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, PaymentAssets::USDT));
		assert_noop!(
			PropertyManagement::distribute_income(RuntimeOrigin::signed([5; 32].into()), 0, 200, PaymentAssets::USDT),
			Error::<Test>::NoLettingAgentFound
		);
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([1; 32].into(), PaymentAssets::USDT), 0);
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_noop!(
			PropertyManagement::distribute_income(RuntimeOrigin::signed([5; 32].into()), 0, 200, PaymentAssets::USDT),
			Error::<Test>::NoPermission
		);
		assert_noop!(
			PropertyManagement::distribute_income(RuntimeOrigin::signed([4; 32].into()), 0, 20000, PaymentAssets::USDT),
			Error::<Test>::NotEnoughFunds
		);
	});
}

#[test]
fn withdraw_funds_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::root(), [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::root(), [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			9_000,
			100,
			bvec![22, 22]
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
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			2200,
			PaymentAssets::USDT,
		));
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			1000,
			PaymentAssets::USDC,
		));
		assert_eq!(PropertyReserve::<Test>::get(0).total, 3000);
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([1; 32].into(), PaymentAssets::USDT), 0);
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([1; 32].into(), PaymentAssets::USDC), 200);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2800);
		assert_eq!(Balances::free_balance(&([4; 32].into())), 4900);
		assert_eq!(Balances::free_balance(&PropertyManagement::account_id()), 5000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::account_id()), 0);
		assert_eq!(ForeignAssets::balance(1337, &PropertyManagement::account_id()), 200);
		assert_ok!(PropertyManagement::withdraw_funds(RuntimeOrigin::signed([1; 32].into()), PaymentAssets::USDC));
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([1; 32].into(), PaymentAssets::USDT), 0);
		assert_eq!(Balances::free_balance(&PropertyManagement::account_id()), 5000);
		assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::governance_account_id()), 2200);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 564_000);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 200);
	});
}

#[test]
fn withdraw_funds_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(NftMarketplace::create_new_region(RuntimeOrigin::root()));
		assert_ok!(NftMarketplace::create_new_location(RuntimeOrigin::root(), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::root(), [10; 32].into()));
		assert_ok!(NftMarketplace::register_lawyer(RuntimeOrigin::root(), [11; 32].into()));
		assert_ok!(NftMarketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			900,
			1000,
			bvec![22, 22]
		));
		assert_ok!(NftMarketplace::buy_token(RuntimeOrigin::signed([1; 32].into()), 0, 1000, PaymentAssets::USDT));
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
		assert_eq!(LocalAssets::total_supply(0), 1000);
		assert_ok!(PropertyManagement::add_letting_agent(
			RuntimeOrigin::root(),
			0,
			bvec![10, 10],
			[4; 32].into(),
		));
		assert_ok!(PropertyManagement::letting_agent_deposit(RuntimeOrigin::signed(
			[4; 32].into()
		)));
		assert_ok!(PropertyManagement::set_letting_agent(RuntimeOrigin::signed([4; 32].into()), 0));
		assert_ok!(PropertyManagement::distribute_income(
			RuntimeOrigin::signed([4; 32].into()),
			0,
			3200,
			PaymentAssets::USDT,
		));
		assert_eq!(InvestorFunds::<Test>::get::<AccountId, PaymentAssets>([1; 32].into(), PaymentAssets::USDT), 200);
		assert_noop!(
			PropertyManagement::withdraw_funds(RuntimeOrigin::signed([2; 32].into()), PaymentAssets::USDT),
			Error::<Test>::UserHasNoFundsStored
		);
	});
}
