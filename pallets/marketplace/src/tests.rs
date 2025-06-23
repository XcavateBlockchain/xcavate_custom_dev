use crate::{mock::*, *, Error, Event};
use frame_support::{assert_noop, assert_ok, traits::{OnFinalize, OnInitialize, fungible::InspectHold, fungibles::InspectHold as FungiblesInspectHold}};
use crate::{ListedToken, NextNftId,
	OngoingObjectListing, NextAssetId, TokenOwner, TokenBuyer,
	TokenListings, OngoingOffers, PropertyOwnerToken, PropertyOwner, PropertyLawyer,
	RealEstateLawyer, RefundToken, AssetIdDetails};
use pallet_regions::RegionDetails;
use sp_runtime::{TokenError, Permill};

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 0 {
			Marketplace::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
		}
		System::reset_events();
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Marketplace::on_initialize(System::block_number());
	}
}

fn new_region_helper() {
	assert_ok!(Regions::add_regional_operator(RuntimeOrigin::root(), [8; 32].into()));
	assert_ok!(Regions::propose_new_region(RuntimeOrigin::signed([8; 32].into()), bvec![10, 10]));
	assert_ok!(Regions::vote_on_region_proposal(RuntimeOrigin::signed([8; 32].into()), 0, pallet_regions::Vote::Yes));
	run_to_block(31);
	assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 0, 100_000));
	run_to_block(61);
	assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 0, 30, Permill::from_percent(3)));
}

#[test]
fn adjust_listing_duration_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_eq!(RegionDetails::<Test>::get(0).unwrap().listing_duration, 30);
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Regions::adjust_listing_duration(
			RuntimeOrigin::signed([8; 32].into()),
			0,
			50,
		));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry, 91);
		assert_eq!(OngoingObjectListing::<Test>::get(1).unwrap().listing_expiry, 111);
		run_to_block(92);
		assert_noop!(
			Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
			Error::<Test>::ListingExpired
		);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 30, 1984));
	})
}

// register_lawyer function
#[test]
fn register_lawyer_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_eq!(RealEstateLawyer::<Test>::get::<AccountId>([0; 32].into()).is_none(), true);
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [0; 32].into()));
		assert_eq!(RealEstateLawyer::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
	})
}

#[test]
fn register_lawyer_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_noop!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [0; 32].into()), Error::<Test>::RegionUnknown);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [0; 32].into()));
		assert_noop!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [0; 32].into()), Error::<Test>::LawyerAlreadyRegistered);
		assert_ok!(Regions::propose_new_region(RuntimeOrigin::signed([8; 32].into()), bvec![10, 10]));
		assert_ok!(Regions::vote_on_region_proposal(RuntimeOrigin::signed([8; 32].into()), 1, pallet_regions::Vote::Yes));
		run_to_block(91);
		assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 1, 100_000));
		run_to_block(121);
		assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 1, 30, Permill::from_percent(3)));
		assert_noop!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 1, [0; 32].into()), Error::<Test>::LawyerAlreadyRegistered);
	})
}

// list_object function
#[test]
fn list_object_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 100);
		assert_eq!(NextNftId::<Test>::get(0), 1);
		assert_eq!(NextNftId::<Test>::get(1), 0);
		assert_eq!(NextAssetId::<Test>::get(), 1);
		assert_eq!(OngoingObjectListing::<Test>::get(0).is_some(), true);
		assert_eq!(AssetIdDetails::<Test>::get(0).is_some(), true);
		assert_eq!(Nfts::owner(0, 0).unwrap(), Marketplace::property_account_id(0));
	})
}

#[test]
fn list_object_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_noop!(
			Marketplace::list_object(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				bvec![10, 10],
				10_000,
				100,
				bvec![22, 22],
				false
			),
			Error::<Test>::RegionUnknown
		);
		new_region_helper();
		assert_noop!(
			Marketplace::list_object(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				bvec![10, 10],
				10_000,
				100,
				bvec![22, 22],
				false
			),
			Error::<Test>::LocationUnknown
		);
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_noop!(
			Marketplace::list_object(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				bvec![10, 10],
				10_000,
				251,
				bvec![22, 22],
				false
			),
			Error::<Test>::TooManyToken
		);
		assert_noop!(
			Marketplace::list_object(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				bvec![10, 10],
				10_000,
				99,
				bvec![22, 22],
				false
			),
			Error::<Test>::TokenAmountTooLow
		);
		assert_noop!(
			Marketplace::list_object(
				RuntimeOrigin::signed([0; 32].into()),
				0,
				bvec![10, 10],
				10_000,
				0,
				bvec![22, 22],
				false
			),
			Error::<Test>::AmountCannotBeZero
		);
	})
}

// buy_property_token function
#[test]
fn buy_property_token_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [6; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [14; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([14; 32].into()),
			0,
			bvec![10, 10],
			10_000_000_000_000_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([6; 32].into()), 0, 30, 1984));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 70);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([6; 32].into(), 0).token_amount, 30);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 1);
		assert_eq!(Balances::free_balance(&([6; 32].into())), 5_000);
		assert_eq!(ForeignAssets::total_balance(1984, &[6; 32].into()), 1_500_000_000_000_000_000);
		assert_eq!(ForeignAssets::balance(1984, &[6; 32].into()), 1_188_000_000_000_000_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[6; 32].into()), 312_000_000_000_000_000);
	})
}

#[test]
fn buy_property_token_works_developer_covers_fees() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			true
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 70);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 30);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 1);
		assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_197_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 303_000);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([6; 32].into(), 0).paid_tax.get(&1984), None);
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(), Some(9_000));
	})
}

#[test]
fn buy_property_token_doesnt_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_noop!(
			Marketplace::buy_property_token(RuntimeOrigin::signed([0; 32].into()), 1, 1, 1984),
			Error::<Test>::TokenNotForSale
		);
	})
}

#[test]
fn buy_property_token_doesnt_work_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_noop!(
			Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 101, 1984),
			Error::<Test>::NotEnoughTokenAvailable
		);
		assert_noop!(
			Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1985),
			Error::<Test>::PaymentAssetNotSupported
		);
		run_to_block(92);
		assert_noop!(
			Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
			Error::<Test>::ListingExpired
		);
	})
}

#[test]
fn buy_property_token_fails_insufficient_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [14; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([14; 32].into()),
			0,
			bvec![10, 10],
			10_000_000_000_000_000,
			100,
			bvec![22, 22],
			false
		));
		assert_noop!(
			Marketplace::buy_property_token(RuntimeOrigin::signed([4; 32].into()), 0, 30, 1984),
			TokenError::FundsUnavailable
		);
		assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 50);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[6; 32].into()), 0);
	})
}

#[test]
fn listing_and_selling_multiple_objects() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [15; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([15; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([2; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 80, 1984));
		assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 20, 1984));
		assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), true);
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			1,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			1,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			1,
			true,
		));
  		System::assert_last_event(Event::DocumentsConfirmed{ 
			signer: [10; 32].into(), 
			listing_id: 1, 
			legal_side: LegalProperty::RealEstateDeveloperSide, 
			approve: true,
		}.into()); 
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			1,
			true,
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 2, 10, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 2, 10, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 2, 30, 1984));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([15; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 33, 1984));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 67);
		assert_eq!(ListedToken::<Test>::get(2).unwrap(), 50);
		assert_eq!(ListedToken::<Test>::get(3).unwrap(), 100);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 2).token_amount, 30);
		assert_eq!(TokenBuyer::<Test>::get(2).len(), 2);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 1).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(1).len(), 0);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(1, [1; 32].into()), 100);
	});
}

// lawyer_claim_property function
#[test]
fn claim_property_works1() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
	})
}

#[test]
fn claim_property_works2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [9; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			200,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 1, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([9; 32].into()), 0, 199, 1337));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			15_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			16_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer_costs.get(&1984).unwrap(), &0u128);
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer_costs.get(&1337).unwrap(), &16_000u128);
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer_costs.get(&1984).unwrap(), &0u128);
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer_costs.get(&1337).unwrap(), &15_000u128);
	})
}

#[test]
fn claim_property_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 99, 1984));
		assert_noop!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		), Error::<Test>::InvalidIndex);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 1, 1984));
		assert_noop!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([9; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		), Error::<Test>::NoPermission);
		assert_noop!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			11_000,
		), Error::<Test>::CostsTooHigh);
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_noop!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		), Error::<Test>::LawyerJobTaken);
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
		assert_ok!(Regions::propose_new_region(RuntimeOrigin::signed([8; 32].into()), bvec![10, 10]));
		assert_ok!(Regions::vote_on_region_proposal(RuntimeOrigin::signed([8; 32].into()), 1, pallet_regions::Vote::Yes));
		run_to_block(91);
		assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 1, 100_000));
		run_to_block(121);
		assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 1, 30, Permill::from_percent(3)));
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 1, bvec![20, 10]));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			1,
			bvec![20, 10],
			1_000,
			100,
			bvec![22, 22],
			false
		));		
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 100, 1984));
		assert_noop!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			1,
			crate::LegalProperty::RealEstateDeveloperSide,
			400,
		), Error::<Test>::WrongRegion);
	})
}
  
// remove_from_case function
#[test]
fn remove_from_case_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [12; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::remove_from_case(
			RuntimeOrigin::signed([10; 32].into()),
			0,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, None);
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([12; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([12; 32].into()));
	})
}
 
#[test]
fn remove_from_case_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_noop!(Marketplace::remove_from_case(
			RuntimeOrigin::signed([10; 32].into()),
			0,
		), Error::<Test>::NoPermission);
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_noop!(Marketplace::remove_from_case(
			RuntimeOrigin::signed([10; 32].into()),
			1,
		), Error::<Test>::InvalidIndex);
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_noop!(Marketplace::remove_from_case(
			RuntimeOrigin::signed([10; 32].into()),
			0,
		), Error::<Test>::AlreadyConfirmed);
	})
} 

// lawyer_confirm_documents function
#[test]
fn distributes_nfts_and_funds() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 60, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1337));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Approved);
		assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 0);
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 100);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 0);
		assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_594_000);
		assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_396_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 2_000);
		assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 2_000);
		assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 4_000);
		assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 4_000);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 876_000);
		assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 22_000);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_084_000);
		assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 12_000);
		assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 100);
	})
} 


#[test]
fn distributes_nfts_and_funds_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		//assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 00, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1337));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Approved);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
		assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_990_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 0);
		assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 6000);
		assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 6000);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 0);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 460_000);
		assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 34_000);
		assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 4_000);
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 100);
	})
} 

#[test]
fn distributes_nfts_and_funds_3() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Regions::propose_new_region(RuntimeOrigin::signed([8; 32].into()), bvec![10, 10]));
		assert_ok!(Regions::vote_on_region_proposal(RuntimeOrigin::signed([8; 32].into()), 0, pallet_regions::Vote::Yes));
		run_to_block(31);
		assert_ok!(Regions::add_regional_operator(RuntimeOrigin::root(), [8; 32].into()));
		assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 0, 100_000));
		run_to_block(61);
		assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 0, 30, Permill::from_parts(32_500)));
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			true
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 60, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1337));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Approved);
		assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 0);
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 100);
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(), Some(19_500));
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1337).copied(), Some(13_000));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 0);
		assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_574_500);
		assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_383_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 2_000);
		assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 2_000);
		assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 4_000);
		assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 4_000);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 894_000);
		assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 23_500);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_096_000);
		assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 13_000);
		assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 100);
	})
} 

#[test]
fn reject_contract_and_refund() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 60, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1337));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			false,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Rejected);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);

		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
		assert_eq!(AssetsHolder::total_balance_on_hold(1337, &[1; 32].into()), 0);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 876_000);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_084_000);
 		assert_eq!(
			TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0)
				.paid_funds
				.get(&1984)
				.unwrap(),
			&600000_u128
		);
		assert_eq!(
			TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0)
				.paid_tax
				.get(&1984)
				.unwrap(),
			&18000_u128
		); 
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			false,
		));		
		System::assert_last_event(Event::DocumentsConfirmed{ 
			signer: [11; 32].into(), 
			listing_id: 0, 
			legal_side: LegalProperty::SpvSide, 
			approve: false,
		}.into()); 
		assert_eq!(RefundToken::<Test>::get(0).unwrap().refund_amount, 100);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 100);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 100);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 0);
		assert_eq!(RefundToken::<Test>::get(0).is_none(), true);
		assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 2000);
		assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 4000); 
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_494_000);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_496_000);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4000);
		assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
		assert_eq!(AssetIdDetails::<Test>::get(0).is_none(), true);
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
		assert_eq!(Balances::free_balance(&(Marketplace::property_account_id(0))), 0);
		assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
		assert_eq!(Nfts::owner(0, 0), None);
		assert_eq!(AssetIdDetails::<Test>::get(0), None);
	})
}

#[test]
fn reject_contract_and_refund_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [7; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([7; 32].into()), 0, 40, 1337));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			false,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Rejected);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
		assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 838_000);
		assert_eq!(ForeignAssets::balance(1337, &[7; 32].into()), 84_000);
		assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_500_000);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			false,
		));
		assert_eq!(RefundToken::<Test>::get(0).unwrap().refund_amount, 100);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 30);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 30);
		assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(RefundToken::<Test>::get(0).unwrap().refund_amount, 70);
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 0);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_497_000);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
		assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 3);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 315000);
		assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([2; 32].into()), 0));
		assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([7; 32].into()), 0));
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 2000);
		assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 4000);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4000);
		assert_eq!(RefundToken::<Test>::get(0).is_none(), true);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
	})
}

#[test]
fn second_attempt_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Approved);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			false,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().second_attempt, true);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			false,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status, crate::DocumentStatus::Rejected);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		));
		assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6000);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_490_000);
		assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
		assert_eq!(AssetIdDetails::<Test>::get(0).is_none(), true);
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
	})
}

#[test]
fn lawyer_confirm_documents_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [12; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			1,
			false,
		), Error::<Test>::InvalidIndex);
		assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([12; 32].into()),
			0,
			false,
		), Error::<Test>::NoPermission);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			false,
		));
		assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		), Error::<Test>::AlreadyConfirmed);
	})
}

// list_token function
#[test]
fn relist_a_nft() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			1
		));
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_eq!(TokenListings::<Test>::get(1).unwrap().item_id, 0);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 99);
		assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 1);
	})
}

#[test]
fn relist_nfts_not_created_with_marketplace_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Nfts::create(
			RuntimeOrigin::signed([0; 32].into()),
			sp_runtime::MultiAddress::Id([0; 32].into()),
			Default::default()
		));
		assert_ok!(Nfts::mint(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			0,
			sp_runtime::MultiAddress::Id([0; 32].into()),
			None
		));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_noop!(
			Marketplace::relist_token(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1),
			Error::<Test>::NftNotFound
		);
	})
} 

#[test]
fn relist_a_nft_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_noop!(
			Marketplace::relist_token(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 10),
			Error::<Test>::SpvNotCreated
		);
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_noop!(
			Marketplace::relist_token(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1),
			TokenError::FundsUnavailable
		);
		assert_noop!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			0
		), Error::<Test>::AmountCannotBeZero);
		assert_noop!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			0,
			1
		), Error::<Test>::InvalidTokenPrice);
	})
} 

// buy_relisted_token function
#[test]
fn buy_relisted_token_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 3, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 97, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 20990000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6000);
		assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 6000);
		assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_468_800);
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([2; 32].into()),
			0,
			1000,
			3
		));
		assert_ok!(Marketplace::buy_relisted_token(RuntimeOrigin::signed([3; 32].into()), 1, 2, 1984));
		assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 3_000);
		assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 2);
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_ok!(Marketplace::buy_relisted_token(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1984));
		assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 2_000);
		assert_eq!(TokenListings::<Test>::get(1).is_some(), false);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			500,
			1
		));
		assert_ok!(Marketplace::buy_relisted_token(RuntimeOrigin::signed([3; 32].into()), 2, 1, 1984));
		assert_eq!(TokenListings::<Test>::get(0).is_some(), false);
		assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 2);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [3; 32].into()), 4);
		assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_469_295);
		assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 1_500);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 2);
		assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 4);
	})
}

#[test]
fn buy_relisted_token_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 20990000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6_000);
		assert_eq!(ForeignAssets::balance(1984,&([8; 32].into())), 6_000);
		assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 460_000);
		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_noop!(
			Marketplace::buy_relisted_token(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1984),
			Error::<Test>::TokenNotForSale
		);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			500,
			1
		));
		assert_noop!(
			Marketplace::buy_relisted_token(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1983),
			Error::<Test>::PaymentAssetNotSupported
		);
	})
}

// make_offer function
#[test]
fn make_offer_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			500,
			1
		));
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 2000, 1, 1984));
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), true);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
		assert_eq!(ForeignAssets::total_balance(1984, &[2; 32].into()), 1_150_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 2000);
	})
}

#[test]
fn make_offer_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_noop!(
			Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984),
			Error::<Test>::TokenNotForSale
		);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			500,
			1
		));
		assert_noop!(
			Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 2, 1984),
			Error::<Test>::NotEnoughTokenAvailable
		);
		assert_noop!(
			Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 100),
			Error::<Test>::PaymentAssetNotSupported
		);
		assert_noop!(
			Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 0, 1984),
			Error::<Test>::AmountCannotBeZero
		);
		assert_noop!(
			Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 0, 1, 1984),
			Error::<Test>::InvalidTokenPrice
		);
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984));
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([3; 32].into()), 1, 300, 1, 1984));
		assert_noop!(
			Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 400, 1, 1984),
			Error::<Test>::OnlyOneOfferPerUser
		);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).unwrap().token_price, 200);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [3; 32].into()).unwrap().token_price, 300);
	})
}

// handle_offer function
#[test]
fn handle_offer_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			5000,
			20
		));
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984));
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([3; 32].into()), 1, 150, 1, 1337));
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 200);
		assert_eq!(AssetsHolder::total_balance_on_hold(1337, &[3; 32].into()), 150);
		assert_ok!(Marketplace::handle_offer(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			[2; 32].into(),
			crate::Offer::Reject
		));
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 0);
		assert_ok!(Marketplace::cancel_offer(RuntimeOrigin::signed([3; 32].into()), 1));
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_150_000);
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_none(), true);
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 2000, 10, 1984));
		assert_eq!(ForeignAssets::total_balance(1984, &([2; 32].into())), 1_150_000);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_130_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 20000);
		assert_ok!(Marketplace::handle_offer(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			[2; 32].into(),
			crate::Offer::Accept
		));
		assert_eq!(TokenListings::<Test>::get(1).unwrap().amount, 10);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_none(), true);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(1)), 0);
		assert_eq!(LocalAssets::balance(0, &([1; 32].into())), 80);
		assert_eq!(LocalAssets::balance(0, &([2; 32].into())), 10);
		assert_eq!(ForeignAssets::balance(0, &Marketplace::property_account_id(0)), 0);
		assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 479_800);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_130_000);
	})
}

#[test]
fn handle_offer_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_noop!(
			Marketplace::handle_offer(
				RuntimeOrigin::signed([1; 32].into()),
				1,
				[2; 32].into(),
				crate::Offer::Reject
			),
			Error::<Test>::TokenNotForSale
		);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			5000,
			2
		));
		assert_noop!(
			Marketplace::handle_offer(
				RuntimeOrigin::signed([1; 32].into()),
				1,
				[2; 32].into(),
				crate::Offer::Reject
			),
			Error::<Test>::InvalidIndex
		);
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984));
		assert_noop!(
			Marketplace::handle_offer(
				RuntimeOrigin::signed([2; 32].into()),
				1,
				[2; 32].into(),
				crate::Offer::Accept
			),
			Error::<Test>::NoPermission
		);
	})
}

// cancel_offer function
#[test]
fn cancel_offer_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			500,
			1
		));
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 2000, 1, 1984));
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), true);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
		assert_eq!(ForeignAssets::total_balance(1984, &([2; 32].into())), 1_150_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 2000);
		assert_ok!(Marketplace::cancel_offer(RuntimeOrigin::signed([2; 32].into()), 1));
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), false);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_150_000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(1)), 0);
	})
}

#[test]
fn cancel_offer_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			500,
			1
		));
		assert_noop!(
			Marketplace::cancel_offer(RuntimeOrigin::signed([2; 32].into()), 1),
			Error::<Test>::InvalidIndex
		);
		assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 2000, 1, 1984));
		assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
		assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), true);
		assert_eq!(ForeignAssets::total_balance(1984, &([2; 32].into())), 1_150_000);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 2000);
		assert_noop!(
			Marketplace::cancel_offer(RuntimeOrigin::signed([1; 32].into()), 1),
			Error::<Test>::InvalidIndex
		);
	})
}

// upgrade_listing function
#[test]
fn upgrade_price_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			1
		));
		assert_ok!(Marketplace::upgrade_listing(RuntimeOrigin::signed([1; 32].into()), 1, 300));
		assert_eq!(TokenListings::<Test>::get(1).unwrap().token_price, 300);
	})
}

#[test]
fn upgrade_price_fails_if_not_owner() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			1
		));
		assert_noop!(
			Marketplace::upgrade_listing(RuntimeOrigin::signed([4; 32].into()), 1, 300),
			Error::<Test>::NoPermission
		);
	})
}

// upgrade_object function
#[test]
fn upgrade_object_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 30000));
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().token_price, 30000);
	})
}

#[test]
fn upgrade_object_and_distribute_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 50, 1984));
		assert_ok!(Marketplace::upgrade_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			20_000
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 50, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 21485000);
		assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 11000);
		assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 11000);
		assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 980_000);
		assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 110_000);

		assert_eq!(AssetIdDetails::<Test>::get(0).unwrap().spv_created, true);
		assert_eq!(ListedToken::<Test>::get(0), None);
	})
}

#[test]
fn upgrade_single_nft_from_listed_object_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_noop!(
			Marketplace::upgrade_listing(RuntimeOrigin::signed([0; 32].into()), 0, 300),
			Error::<Test>::TokenNotForSale
		);
	})
}
 
#[test]
fn upgrade_object_for_relisted_nft_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([0; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			1000,
			1
		));
		assert_noop!(
			Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 1, 300),
			Error::<Test>::TokenNotForSale
		);
	})
}

#[test]
fn upgrade_object_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_noop!(
			Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 300),
			Error::<Test>::TokenNotForSale
		);
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([0; 32].into()), 0, 100, 1984));
		assert_noop!(
			Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 300),
			Error::<Test>::TokenNotForSale
		);
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		run_to_block(100);
		assert_noop!(
			Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 1, 300),
			Error::<Test>::ListingExpired
		);
	})
}

// delist_token function
#[test]
fn delist_single_token_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			1
		));
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 99);
		assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 1);
		assert_ok!(Marketplace::delist_token(RuntimeOrigin::signed([1; 32].into()), 1));
		assert_eq!(TokenListings::<Test>::get(0), None);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			3
		));
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 97);
		assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 3);
		assert_ok!(Marketplace::buy_relisted_token(RuntimeOrigin::signed([2; 32].into()), 2, 2, 1984));
		assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 1);
		assert_ok!(Marketplace::delist_token(RuntimeOrigin::signed([1; 32].into()), 2));
		assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 2);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 98);
	})
}
 
#[test]
fn delist_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [4; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			1
		));
		assert_noop!(
			Marketplace::delist_token(RuntimeOrigin::signed([4; 32].into()), 1),
			Error::<Test>::NoPermission
		);
		assert_noop!(
			Marketplace::delist_token(RuntimeOrigin::signed([1; 32].into()), 2),
			Error::<Test>::TokenNotForSale
		);
	})
}

#[test]
fn listing_objects_in_different_regions() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::propose_new_region(RuntimeOrigin::signed([8; 32].into()), bvec![10, 10]));
		assert_ok!(Regions::vote_on_region_proposal(RuntimeOrigin::signed([8; 32].into()), 1, pallet_regions::Vote::Yes));
		run_to_block(91);
		assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 1, 100_000));
		run_to_block(121);
		assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 1, 30, Permill::from_percent(3)));
		assert_ok!(Regions::propose_new_region(RuntimeOrigin::signed([8; 32].into()), bvec![10, 10]));
		assert_ok!(Regions::vote_on_region_proposal(RuntimeOrigin::signed([8; 32].into()), 2, pallet_regions::Vote::Yes));
		run_to_block(151);
		assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 2, 100_000));
		run_to_block(181);
		assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 2, 30, Permill::from_percent(3)));
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 1, bvec![10, 10]));
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 2, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 1, [12; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 1, [13; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 2, [14; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 2, [15; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			1,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			2,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([12; 32].into()),
			1,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([13; 32].into()),
			1,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([12; 32].into()),
			1,
			true,
		));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([13; 32].into()),
			1,
			true,
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 2, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([14; 32].into()),
			2,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([15; 32].into()),
			2,
			crate::LegalProperty::SpvSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([14; 32].into()),
			2,
			true,
		));
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([15; 32].into()),
			2,
			true,
		));
		assert_eq!(AssetIdDetails::<Test>::get(1).unwrap().spv_created, true);
		assert_eq!(AssetIdDetails::<Test>::get(2).unwrap().spv_created, true);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			1000,
			100
		));
		assert_ok!(Marketplace::buy_relisted_token(
			RuntimeOrigin::signed([2; 32].into()),
			3,
			100,
			1984
		));
		assert_eq!(LocalAssets::balance(1, &[2; 32].into()), 100);
		assert_eq!(LocalAssets::balance(2, &[2; 32].into()), 100);
	})
}


#[test]
fn cancel_property_purchase_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 40);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 30);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0).token_amount, 30);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 2);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
		assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 312_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 312_000);
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(), Some(600_000));
		assert_ok!(Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(), Some(300_000));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 70);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 1);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
	})
}

#[test]
fn cancel_property_purchase_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_noop!(
			Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::InvalidIndex
		);
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984));
		assert_noop!(
			Marketplace::cancel_property_purchase(RuntimeOrigin::signed([3; 32].into()), 0),
			Error::<Test>::NoTokenBought
		);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 40, 1984));
		assert_noop!(
			Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::PropertyAlreadySold
		);
	})
}

#[test]
fn cancel_property_purchase_fails_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 40, 1984));
		run_to_block(100);
		assert_noop!(
			Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::ListingExpired
		);
	})
}

#[test]
fn withdraw_expired_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 70);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 30);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 1);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
		assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 312_000);
		assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(), Some(300_000));
		run_to_block(100);
		assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
	})
}

#[test]
fn withdraw_expired_works_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			1_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 20, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([3; 32].into()), 0, 4, 1984));
		assert_eq!(ListedToken::<Test>::get(0).unwrap(), 46);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 30);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 3);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_468_800);
		assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_129_200);
		assert_eq!(ForeignAssets::total_balance(1984, &[2; 32].into()), 1_150_000);
		assert_eq!(ForeignAssets::balance(1984, &[3; 32].into()), 840);
		assert_eq!(ForeignAssets::total_balance(1984, &[3; 32].into()), 5_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 31_200);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 20_800);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[3; 32].into()), 4_160);
		run_to_block(100);
		assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0));
		assert_eq!(ListedToken::<Test>::get(0), Some(76));
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([3; 32].into(), 0).token_amount, 4);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 2);
		assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
		assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([2; 32].into()), 0));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 10_000);
		assert_eq!(Balances::free_balance(&(Marketplace::property_account_id(0))), 99);
		assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 99);
		assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([3; 32].into()), 0));
		assert_eq!(Balances::free_balance(&(Marketplace::property_account_id(0))), 0);
		assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
		assert_eq!(TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).token_amount, 0);
		assert_eq!(TokenBuyer::<Test>::get(0).len(), 0);
		assert_eq!(ForeignAssets::balance(1984, &[3; 32].into()), 5_000);
		assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[3; 32].into()), 0);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 0);
	})
}

#[test]
fn withdraw_expired_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_noop!(
			Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::InvalidIndex
		);
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_noop!(
			Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::ListingNotExpired
		);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		run_to_block(100);
		assert_noop!(
			Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
			Error::<Test>::PropertyAlreadySold
		);
	})
}

#[test]
fn withdraw_expired_fails_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 99, 1984));
		run_to_block(100);
		assert_noop!(
			Marketplace::withdraw_expired(RuntimeOrigin::signed([2; 32].into()), 0),
			Error::<Test>::NoTokenBought
		);
	})
} 

#[test]
fn send_property_token_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));	
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [10; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [11; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));	
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 100);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 100);
		assert_eq!(PropertyOwner::<Test>::get(0).len(), 1);
		assert_ok!(Marketplace::send_property_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			[2; 32].into(),
			20
		));
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 80);
		assert_eq!(PropertyOwner::<Test>::get(0).len(), 2);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 80);
		assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 20);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [2; 32].into()), 20);
		assert_ok!(Marketplace::send_property_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			[3; 32].into(),
			80
		));
		assert_ok!(Marketplace::send_property_token(
			RuntimeOrigin::signed([2; 32].into()),
			0,
			[3; 32].into(),
			20
		));
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
		assert_eq!(PropertyOwner::<Test>::get(0).len(), 1);
		assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [2; 32].into()), 0);
		assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 100);
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [3; 32].into()), 100);
	})
} 

#[test]
fn send_property_token_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_noop!(Marketplace::send_property_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			[2; 32].into(),
			20
		), Error::<Test>::UserNotWhitelisted);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		assert_noop!(Marketplace::send_property_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			[2; 32].into(),
			20
		), Error::<Test>::UserNotWhitelisted);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_noop!(Marketplace::send_property_token(
			RuntimeOrigin::signed([1; 32].into()),
			1,
			[1; 32].into(),
			20
		), Error::<Test>::NotEnoughToken);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [10; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [11; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));	
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_noop!(Marketplace::send_property_token(
			RuntimeOrigin::signed([2; 32].into()),
			0,
			[1; 32].into(),
			20
		), Error::<Test>::NotEnoughToken);
	})
} 

#[test]
fn send_property_token_fails_if_relist() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [2; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [3; 32].into()));	
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [10; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [11; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [10; 32].into()));
		assert_ok!(Marketplace::register_lawyer(RuntimeOrigin::signed([8; 32].into()), 0, [11; 32].into()));	
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 20, 1984));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 80, 1984));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
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
		assert_eq!(PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 20);
		assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 20);
		assert_eq!(PropertyOwner::<Test>::get(0).len(), 2);
		assert_ok!(Marketplace::relist_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			1000,
			15
		));
		assert_noop!(Marketplace::send_property_token(
			RuntimeOrigin::signed([1; 32].into()),
			0,
			[2; 32].into(),
			10
		), Error::<Test>::NotEnoughToken);
	})
} 

#[test]
fn withdraw_deposit_unsold_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		run_to_block(100);
		assert_ok!(Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0));
		assert_eq!(ListedToken::<Test>::get(0), None);
		assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 0);
	})
}

#[test]
fn withdraw_deposit_unsold_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		run_to_block(20);
		assert_noop!(Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0), Error::<Test>::ListingNotExpired);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 10, 1984));
		assert_noop!(Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 1), Error::<Test>::InvalidIndex);
		run_to_block(100);
		assert_noop!(Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0), Error::<Test>::TokenNotReturned);
	})
}

#[test]
fn withdraw_deposit_unsold_fails_2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [8; 32].into()));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 0, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [0; 32].into()));
		assert_ok!(XcavateWhitelist::add_to_whitelist(RuntimeOrigin::root(), [1; 32].into()));
		assert_ok!(Marketplace::list_object(
			RuntimeOrigin::signed([0; 32].into()),
			0,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_eq!(Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())), 100_000);
		run_to_block(20);
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984));
		run_to_block(100);
		assert_noop!(Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0), Error::<Test>::PropertyAlreadySold);
	})
}