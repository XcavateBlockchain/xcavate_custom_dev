use crate::{mock::*, Error};
use frame_support::BoundedVec;
use frame_support::{assert_noop, assert_ok, traits::fungible::InspectHold};
use crate::{Regions, LocationRegistration, HoldReason, TakeoverRequests};
use sp_runtime::{Permill, TokenError};

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

#[test]
fn create_new_region_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_eq!(Balances::free_balance(&([8; 32].into())), 200_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 200_000);
		assert_eq!(Regions::<Test>::get(0).unwrap().collection_id, 0);
		assert_eq!(Regions::<Test>::get(1).unwrap().collection_id, 1);
		assert_eq!(Regions::<Test>::get(0).unwrap().listing_duration, 30);
		assert_eq!(Regions::<Test>::get(0).unwrap().owner, [8; 32].into());
	})
}

#[test]
fn create_new_region_does_not_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Region::create_new_region(RuntimeOrigin::signed([7; 32].into()), 30, Permill::from_percent(3)),
			Error::<Test>::UserNotWhitelisted
		);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [7; 32].into()));
		assert_noop!(
			Region::create_new_region(RuntimeOrigin::signed([7; 32].into()), 30, Permill::from_percent(3)),
			TokenError::FundsUnavailable
		);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_noop!(
			Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 0, Permill::from_percent(3)),
			Error::<Test>::ListingDurationCantBeZero
		);
		assert_noop!(
			Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 10_001, Permill::from_percent(3)),
			Error::<Test>::ListingDurationTooHigh
		);
	})
}

#[test]
fn adjust_listing_duration_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_eq!(Regions::<Test>::get(0).unwrap().listing_duration, 30);
		assert_ok!(Region::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
	/* 	assert_ok!(Region::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		)); */
		assert_ok!(Region::adjust_listing_duration(
			RuntimeOrigin::signed([8; 32].into()),
			0,
			50,
		));
		/* assert_ok!(Region::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry, 31);
		assert_eq!(OngoingObjectListing::<Test>::get(1).unwrap().listing_expiry, 51);
		run_to_block(32);
		assert_noop!(
			Region::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
			Error::<Test>::ListingExpired
		);
		assert_ok!(Region::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 30, 1984)); */
	})
}

#[test]
fn adjust_listing_duration_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(
			Region::adjust_listing_duration(
				RuntimeOrigin::signed([8; 32].into()),
				0,
				50,
			),
			Error::<Test>::UserNotWhitelisted
		);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_noop!(
			Region::adjust_listing_duration(
				RuntimeOrigin::signed([8; 32].into()),
				0,
				50,
			),
			Error::<Test>::RegionUnknown
		);
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_noop!(
			Region::adjust_listing_duration(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				50,
			),
			Error::<Test>::NoPermission
		);
		assert_noop!(
			Region::adjust_listing_duration(
				RuntimeOrigin::signed([8; 32].into()),
				0,
				0,
			),
			Error::<Test>::ListingDurationCantBeZero
		);
		assert_noop!(
			Region::adjust_listing_duration(
				RuntimeOrigin::signed([8; 32].into()),
				0,
				100000,
			),
			Error::<Test>::ListingDurationTooHigh
		);
	})
}

#[test]
fn propose_region_takeover_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 100_000);
		assert_eq!(Regions::<Test>::get(0).unwrap().collection_id, 0);
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(TakeoverRequests::<Test>::get(0).unwrap(), [1; 32].into());
		assert_eq!(Balances::free_balance(&([1; 32].into())), 14_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 100_000);
	})
}

#[test]
fn propose_region_takeover_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_noop!(
			Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::UserNotWhitelisted,
		);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_noop!(
			Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 1),
			Error::<Test>::RegionUnknown,
		);
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_noop!(
			Region::propose_region_takeover(RuntimeOrigin::signed([8; 32].into()), 0),
			Error::<Test>::AlreadyRegionOwner,
		);
		assert_noop!(
			Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::TakeoverAlreadyPending,
		);
	})
}

#[test]
fn handle_takeover_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 100_000);
		assert_eq!(Regions::<Test>::get(0).unwrap().collection_id, 0);
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(TakeoverRequests::<Test>::get(0).unwrap(), [1; 32].into());
		assert_eq!(Balances::free_balance(&([1; 32].into())), 14_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 100_000);
		assert_ok!(Region::handle_takeover(RuntimeOrigin::signed([8; 32].into()), 0, crate::TakeoverAction::Reject));
		assert_eq!(TakeoverRequests::<Test>::get(0).is_none(), true);
		assert_eq!(Balances::free_balance(&([1; 32].into())), 15_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 0);
		assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 100_000);
		assert_eq!(Regions::<Test>::get(0).unwrap().owner, [8; 32].into());
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())), 100_000);
		assert_ok!(Region::handle_takeover(RuntimeOrigin::signed([8; 32].into()), 0, crate::TakeoverAction::Accept));
		assert_eq!(Balances::free_balance(&([0; 32].into())), 19_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())), 100_000);
		assert_eq!(Balances::free_balance(&([8; 32].into())), 400_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 0);
		assert_eq!(Regions::<Test>::get(0).unwrap().owner, [0; 32].into());
		assert_eq!(TakeoverRequests::<Test>::get(0).is_none(), true);
	})
}

#[test]
fn handle_takeover_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_noop!(
			Region::handle_takeover(RuntimeOrigin::signed([8; 32].into()), 0, crate::TakeoverAction::Reject),
			Error::<Test>::NoTakeoverRequest
		);
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_noop!(
			Region::handle_takeover(RuntimeOrigin::signed([8; 32].into()), 1, crate::TakeoverAction::Reject),
			Error::<Test>::RegionUnknown
		);
		assert_noop!(
			Region::handle_takeover(RuntimeOrigin::signed([1; 32].into()), 0, crate::TakeoverAction::Reject),
			Error::<Test>::NoPermission
		);
	})
}

#[test]
fn cancel_takeover_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 100_000);
		assert_eq!(Regions::<Test>::get(0).unwrap().collection_id, 0);
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(TakeoverRequests::<Test>::get(0).unwrap(), [1; 32].into());
		assert_eq!(Balances::free_balance(&([1; 32].into())), 14_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 100_000);
		assert_ok!(Region::cancel_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(TakeoverRequests::<Test>::get(0).is_none(), true);
		assert_eq!(Balances::free_balance(&([1; 32].into())), 15_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 0);
		assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 100_000);
	})
}

#[test]
fn cancel_takeover_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_noop!(
			Region::cancel_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::NoTakeoverRequest
		);
		assert_ok!(Region::propose_region_takeover(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(TakeoverRequests::<Test>::get(0).unwrap(), [1; 32].into());
		assert_eq!(Balances::free_balance(&([1; 32].into())), 14_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 100_000);
		assert_noop!(
			Region::cancel_region_takeover(RuntimeOrigin::signed([8; 32].into()), 0),
			Error::<Test>::NoPermission
		);
		assert_eq!(Balances::free_balance(&([1; 32].into())), 14_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())), 100_000);
		assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 100_000);
	})
}

// create_new_location function
#[test]
fn create_new_location_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_ok!(Region::create_new_region(RuntimeOrigin::signed([8; 32].into()), 30, Permill::from_percent(3)));
		assert_ok!(Region::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(Region::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![9, 10]));
		assert_ok!(Region::create_new_location(RuntimeOrigin::signed([8; 32].into()), 1, bvec![9, 10]));
		assert_eq!(Balances::free_balance(&([8; 32].into())), 170_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())), 200_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::LocationDepositReserve.into(), &([8; 32].into())), 30_000);
		assert_eq!(
			LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				0,
				bvec![10, 10]
			),
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
			LocationRegistration::<Test>::get::<u32, BoundedVec<u8, Postcode>>(
				1,
				bvec![10, 10]
			),
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
			Region::create_new_location(RuntimeOrigin::signed([8; 32].into()), 1, bvec![10, 10]),
			Error::<Test>::UserNotWhitelisted
		);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_noop!(
			Region::create_new_location(RuntimeOrigin::signed([8; 32].into()), 1, bvec![10, 10]),
			Error::<Test>::RegionUnknown
		);
	})
}