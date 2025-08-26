use crate::{mock::*, Error, Event, *};
use crate::{
    OngoingObjectListing, OngoingOffers, PropertyLawyer, RefundToken, TokenListings, TokenOwner, RefundClaimedToken
};
use frame_support::{
    assert_noop, assert_ok,
    traits::{
        fungible::Inspect as FungibleInspect,
        fungible::InspectHold,
        fungibles::InspectHold as FungiblesInspectHold,
        fungibles::{Inspect, InspectFreeze},
        OnFinalize, OnInitialize,
    },
};
use pallet_real_estate_asset::{
    Error as RealEstateAssetError, NextAssetId, NextNftId, PropertyAssetInfo, PropertyOwner,
    PropertyOwnerToken,
};
use pallet_regions::{RealEstateLawyer, RegionDetails, RegionIdentifier};
use sp_runtime::{traits::BadOrigin, Permill, TokenError};

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
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        pallet_regions::Vote::Yes,
        10_000
    ));
    run_to_block(31);
    assert_ok!(Regions::bid_on_region(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        100_000
    ));
    run_to_block(61);
    assert_ok!(Regions::create_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        30,
        Permill::from_percent(3)
    ));
}

#[test]
fn adjust_listing_duration_works() {
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
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().listing_duration, 30);
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Regions::adjust_listing_duration(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            50,
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
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry,
            91
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(1).unwrap().listing_expiry,
            111
        );
        run_to_block(92);
        assert_noop!(
            Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
            Error::<Test>::ListingExpired
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            30,
            1984
        ));
    })
}

// list_property function
#[test]
fn list_property_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
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
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            100
        );
        assert_eq!(NextNftId::<Test>::get(0), 1);
        assert_eq!(NextNftId::<Test>::get(1), 0);
        assert_eq!(NextAssetId::<Test>::get(), 1);
        assert_eq!(OngoingObjectListing::<Test>::get(0).is_some(), true);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_some(), true);
        assert_eq!(
            Nfts::owner(0, 0).unwrap(),
            Marketplace::property_account_id(0)
        );
    })
}

#[test]
fn list_property_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
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
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                100,
                bvec![22, 22],
                false
            ),
            Error::<Test>::LocationUnknown
        );
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                251,
                bvec![22, 22],
                false
            ),
            Error::<Test>::TooManyToken
        );
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                99,
                bvec![22, 22],
                false
            ),
            Error::<Test>::TokenAmountTooLow
        );
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [14; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([14; 32].into()),
            3,
            bvec![10, 10],
            10_000_000_000_000_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([6; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            70
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([6; 32].into(), 0)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(Balances::free_balance(&([6; 32].into())), 5_000);
        assert_eq!(
            ForeignAssets::total_balance(1984, &[6; 32].into()),
            1_500_000_000_000_000_000
        );
        assert_eq!(
            ForeignAssets::balance(1984, &[6; 32].into()),
            1_188_000_000_000_000_000
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[6; 32].into()),
            312_000_000_000_000_000
        );
        System::assert_last_event(
            Event::PropertyTokenBought {
                listing_index: 0,
                asset_id: 0,
                buyer: [6; 32].into(),
                amount_purchased: 30,
                price_paid: 300_000_000_000_000_000,
                tax_paid: 9_000_000_000_000_000,
                payment_asset: 1984,
                new_tokens_remaining: 70,
            }
            .into(),
        );
    })
}

#[test]
fn buy_property_token_works_developer_covers_fees() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            true
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            70
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(
            ForeignAssets::total_balance(1984, &[1; 32].into()),
            1_500_000
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_197_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            303_000
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([6; 32].into(), 0),
            None
        );
        /*         assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(9_000)
        ); */
        System::assert_last_event(
            Event::PropertyTokenBought {
                listing_index: 0,
                asset_id: 0,
                buyer: [1; 32].into(),
                amount_purchased: 30,
                price_paid: 300_000,
                tax_paid: 0,
                payment_asset: 1984,
                new_tokens_remaining: 70,
            }
            .into(),
        );
    })
}

#[test]
fn buy_property_token_doesnt_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
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
            Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 50, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_noop!(
            Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1985),
            Error::<Test>::PaymentAssetNotSupported
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1984),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        run_to_block(92);
        assert_noop!(
            Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
            Error::<Test>::ListingExpired
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            1_000,
            250,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 125, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 1, 124, 1984));
    })
}

#[test]
fn buy_property_token_fails_insufficient_balance() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [14; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([14; 32].into()),
            3,
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
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[6; 32].into()),
            0
        );
    })
}

#[test]
fn listing_and_selling_multiple_objects() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
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
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
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
            1,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            1,
            30,
            1984
        ));
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            1
        ));
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), true);
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            1,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            true,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            1,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes,
            40,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            crate::Vote::Yes,
            20,
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            30,
            1984
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            33,
            1984
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ),);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            1,
            true,
        ));
        System::assert_last_event(
            Event::DocumentsConfirmed {
                signer: [10; 32].into(),
                listing_id: 1,
                legal_side: LegalProperty::RealEstateDeveloperSide,
                approve: true,
            }
            .into(),
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            1,
            true,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            67
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(2)
                .unwrap()
                .listed_token_amount,
            50
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(3)
                .unwrap()
                .listed_token_amount,
            100
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 2)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 1),
            None
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(1, [1; 32].into()),
            40
        );
    });
}

// claim_property_token function
#[test]
fn claim_property_token_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone(),
            None
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            416_000
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(400_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_fees
                .get(&1984)
                .copied(),
            Some(4_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(12_000)
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            416_000
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            60
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &412_000_u128
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1337),
            None
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[2; 32].into())
                .clone(),
            None
        );
        assert_eq!(PropertyLawyer::<Test>::get(0).is_some(), false);
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(1_000_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_fees
                .get(&1984)
                .copied(),
            Some(10_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(30_000)
        );
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 30);
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            1040_000
        );
        assert_eq!(PropertyLawyer::<Test>::get(0).is_some(), true);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[2; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &309_000_u128
        );
        System::assert_last_event(
            Event::PropertyTokenClaimed {
                listing_id: 0,
                asset_id: 0,
                owner: [30; 32].into(),
                amount: 30,
            }
            .into(),
        );
    })
}

#[test]
fn claim_property_token_works_2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone(),
            None
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            416_000
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(700_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_fees
                .get(&1984)
                .copied(),
            Some(7_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(21_000)
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            40
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            728_000
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 30);
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            30
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &412_000_u128
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1337),
            None
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[2; 32].into())
                .clone(),
            None
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([30; 32].into(), 0),
            None
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([30; 32].into(), 0).unwrap().token_amount,
            10
        );
        assert_ok!(Marketplace::withdraw_unclaimed(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(800_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_fees
                .get(&1984)
                .copied(),
            Some(8_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(24_000)
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            40
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [2; 32].into()),
            20
        );
    })
}

#[test]
fn claim_property_token_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::TokenNotForSale
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
            48,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            26,
            1984
        ));
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([6; 32].into()),
            0,
            1,
            1984
        ));
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([3; 32].into()), 0,),
            BadOrigin
        );
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::TokenOwnerNotFound
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::TokenOwnerNotFound
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ClaimWindowExpired
        );
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_noop!(
            Marketplace::claim_property_token(RuntimeOrigin::signed([6; 32].into()), 0,),
            Error::<Test>::NoValidTokenToClaim
        );
    })
}

#[test]
fn relist_unclaim_property_token_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            0
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .unclaimed_token_amount,
            30
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            30
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry,
            frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get()
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([6; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([6; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(1_000_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_fees
                .get(&1984)
                .copied(),
            Some(10_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(30_000)
        );
        assert_eq!(LocalAssets::balance(0, &[6; 32].into()), 30);
    })
}

// finalize_claim_window function
#[test]
fn finalize_claim_window_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            0
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .unclaimed_token_amount,
            30
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().buyers.len(),
            1
        );
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            30
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry,
            frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get()
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().relist_count,
            1
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .unclaimed_token_amount,
            0
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().buyers.len(),
            0
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            15,
            1984
        ));
    })
}

#[test]
fn finalize_claim_window_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
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
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ClaimWindowNotExpired
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().relist_count,
            1
        );
        assert_ok!(Marketplace::withdraw_unclaimed(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().relist_count,
            2
        );
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::TokenGettingRefunded
        );
    })
}

// create_spv function
#[test]
fn create_spv_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            45,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap().spv_created,
            false
        );
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
    })
}

#[test]
fn create_spv_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_noop!(
            Marketplace::create_spv(RuntimeOrigin::signed([2; 32].into()), 0,),
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
            48,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            26,
            1984
        ));
        assert_noop!(
            Marketplace::create_spv(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::PropertyHasNotBeenSoldYet
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_noop!(
            Marketplace::create_spv(RuntimeOrigin::signed([1; 32].into()), 0,),
            BadOrigin
        );
    })
}

// lawyer_claim_property function
#[test]
fn claim_property_works1() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            45,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_eq!(
            ProposedLawyers::<Test>::get(0).unwrap().lawyer,
            [10; 32].into()
        );
        assert_eq!(ProposedLawyers::<Test>::get(0).unwrap().costs, 4_000);
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            None
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(
            SpvLawyerProposal::<Test>::get(0).unwrap().lawyer,
            [11; 32].into()
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).unwrap().expiry_block, 91);
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_some(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
    })
}

#[test]
fn claim_property_works2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [9; 32].into(),
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
            10_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([9; 32].into()),
            0,
            98,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            50,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            51,
            1337
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([9; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            15_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            16_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            1
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([9; 32].into()),
            0,
            crate::Vote::Yes,
            98
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .spv_lawyer_costs
                .get(&1984)
                .unwrap(),
            &0u128
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .spv_lawyer_costs
                .get(&1337)
                .unwrap(),
            &16_000u128
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1984)
                .unwrap(),
            &0u128
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1337)
                .unwrap(),
            &15_000u128
        );
    })
}

#[test]
fn claim_property_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            29,
            1984
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([9; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            BadOrigin
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                11_000,
            ),
            Error::<Test>::CostsTooHigh
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_eq!(
            ProposedLawyers::<Test>::get(0).unwrap().lawyer,
            [10; 32].into()
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            Error::<Test>::LawyerProposalOngoing
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::SpvSide,
                4_000,
            ),
            RealEstateAssetError::<Test>::SpvNotCreated
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
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::SpvSide,
                4_000,
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            Error::<Test>::LawyerJobTaken
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::SpvSide,
                4_000,
            ),
            Error::<Test>::NoPermission
        );
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::France
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            pallet_regions::Vote::Yes,
            10_000
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            100_000
        ));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            bvec![20, 10]
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            2,
            bvec![20, 10],
            1_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            1,
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                1,
                crate::LegalProperty::RealEstateDeveloperSide,
                400,
            ),
            Error::<Test>::WrongRegion
        );
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([12; 32].into()),
            2,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([12; 32].into()),
            1,
            crate::LegalProperty::SpvSide,
            400,
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([12; 32].into()),
                1,
                crate::LegalProperty::RealEstateDeveloperSide,
                400,
            ),
            Error::<Test>::NoPermission
        );
    })
}

// approve_developer_lawyer function
#[test]
fn approve_developer_lawyer_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
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
            1,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
        ));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            0
        );
        assert_eq!(ProposedLawyers::<Test>::get(0).is_none(), true);
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            None
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            3_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            1
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            1,
            crate::LegalProperty::RealEstateDeveloperSide,
            300,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            true
        ));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            2
        );
        assert_eq!(ProposedLawyers::<Test>::get(0).is_none(), true);
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1984)
                .unwrap(),
            &3_000u128
        );
    })
}

#[test]
fn approve_developer_lawyer_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
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
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([0; 32].into()), 0, true),
            Error::<Test>::InvalidIndex
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
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([0; 32].into()), 0, true),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([0; 32].into()), 0, true),
            Error::<Test>::NoLawyerProposed
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([10; 32].into()), 0, false),
            Error::<Test>::NoPermission
        );
    })
}

// vote_on_spv_lawyer function
#[test]
fn vote_on_spv_lawyer_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[1; 32].into()
            ),
            40
        );
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[2; 32].into()
            ),
            40
        );
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::No,
            20
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[30; 32].into()
            ),
            20
        );
        assert_eq!(
            OngoingLawyerVoting::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 40,
                no_voting_power: 60,
            }
        );
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into())
                .unwrap()
                .vote,
            crate::Vote::No
        );
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[1; 32].into()
            ),
            20
        );
        assert_eq!(
            OngoingLawyerVoting::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 60,
                no_voting_power: 20,
            }
        );
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into())
                .unwrap()
                .vote,
            crate::Vote::Yes
        );
    })
}

#[test]
fn vote_on_spv_lawyer_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::No,
                20,
            ),
            Error::<Test>::NoLawyerProposed
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::No,
                40
            ),
            Error::<Test>::NoLawyerProposed
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::Vote::No,
                100
            ),
            Error::<Test>::NotEnoughToken
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[0; 32].into()
            ),
            0
        );
        run_to_block(100);
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::Vote::No,
                12
            ),
            Error::<Test>::VotingExpired
        );
    })
}

// vote_on_spv_lawyer function
#[test]
fn finalize_spv_lawyer_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::No,
            20
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 1);
        run_to_block(121);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none(),
            true
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            3_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 2);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        run_to_block(151);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(2, [1; 32].into()).is_none(),
            false
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(ListingSpvProposal::<Test>::get(0).is_none(), true);
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([10; 32].into())
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .spv_lawyer_costs
                .get(&1984)
                .unwrap(),
            &3_000u128
        );
    })
}

#[test]
fn finalize_spv_lawyer_works2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            150,
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
            70,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
        ));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none(),
            true
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            3_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 1);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            15
        ));
        run_to_block(121);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(1, [1; 32].into()).is_none(),
            true
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            3_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            55
        ));
        run_to_block(151);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(ListingSpvProposal::<Test>::get(0).is_none(), true);
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([10; 32].into())
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .spv_lawyer_costs
                .get(&1984)
                .unwrap(),
            &3_000u128
        );
    })
}

#[test]
fn finalize_spv_lawyer_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),
            Error::<Test>::NoLawyerProposed
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
            45,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),
            Error::<Test>::NoLawyerProposed
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            45
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
        run_to_block(91);
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([10; 32].into()), 0,),
            BadOrigin
        );
    })
}

// remove_lawyer_claim function
#[test]
fn remove_lawyer_claim_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
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
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([12; 32].into()),
            3,
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::remove_lawyer_claim(
            RuntimeOrigin::signed([10; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            None
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([12; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([12; 32].into())
        );
    })
}

#[test]
fn remove_lawyer_claim_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_noop!(
            Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 1,),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_noop!(
            Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 0,),
            Error::<Test>::AlreadyConfirmed
        );
    })
}

// lawyer_confirm_documents function
#[test]
fn finalize_property_deal() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            15
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_594_000);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_396_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            4_000
        );
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 4_000);
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            4_000
        );
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 4_000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_240_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 942_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 1_044_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 22_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_292_000);
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 942_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 12_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 45);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 15);
    })
}

#[test]
fn finalize_property_deal_2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1337
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_990_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            8000
        );
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 8000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_084_000);
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 838_000);
        assert_eq!(ForeignAssets::balance(1337, &[30; 32].into()), 888_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 34_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 30);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 30);
    })
}

#[test]
fn finalize_property_deal_3() {
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
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            pallet_regions::Vote::Yes,
            10_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            100_000
        ));
        run_to_block(61);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            30,
            Permill::from_parts(32_500)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            true
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(19_500)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1337)
                .copied(),
            Some(13_000)
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_574_500);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_383_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            4_000
        );
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 4_000);
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            4_000
        );
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 4_000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_298_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 948_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 19_500);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_298_000);
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 948_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 13_000);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
    })
}

#[test]
fn finalize_property_deal_4() {
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
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            pallet_regions::Vote::Yes,
            10_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            100_000
        ));
        run_to_block(61);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            30,
            Permill::from_parts(32_500)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            true
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(
            LocalAssets::balance(40, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1984)
                .copied(),
            Some(19_500)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_tax
                .get(&1337)
                .copied(),
            Some(13_000)
        );
        assert_ok!(ForeignAssets::transfer(
            RuntimeOrigin::signed([1; 32].into()),
            codec::Compact(1337),
            sp_runtime::MultiAddress::Id(Marketplace::property_account_id(0)),
            404_000
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_574_500);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_383_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            4_000
        );
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 4_000);
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            4_000
        );
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 4_000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_096_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 998_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 19_500);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_096_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 13_000);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 20);
    })
}

#[test]
fn reject_contract_and_refund() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
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
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);

        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            0
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1337, &[1; 32].into()),
            0
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_240_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_292_000);
        /*         assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &600000_u128
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap()
                .paid_tax
                .get(&1984)
                .unwrap(),
            &18000_u128
        ); */
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            false,
        ));
        /*         System::assert_last_event(
            Event::DocumentsConfirmed {
                signer: [11; 32].into(),
                listing_id: 0,
                legal_side: LegalProperty::SpvSide,
                approve: false,
            }
            .into(),
        ); */
        assert_eq!(RefundToken::<Test>::get(0).unwrap().refund_amount, 100);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 45);
        /*         assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap().token_amount,
            100
        ); */
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(RefundToken::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            4000
        );
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            2000
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_497_500);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_498_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_147_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 1_197_500);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(
            Balances::free_balance(&(Marketplace::property_account_id(0))),
            0
        );
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into())
                .unwrap()
                .active_cases,
            0
        );
    })
}

#[test]
fn reject_contract_and_refund_2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([7; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Rejected
        );
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
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(RefundToken::<Test>::get(0).unwrap().refund_amount, 70);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            0
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_497_000);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            315000
        );
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([7; 32].into()),
            0
        ));
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            2000
        );
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::treasury_account_id()),
            4000
        );
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4000);
        assert_eq!(RefundToken::<Test>::get(0).is_none(), true);
        assert!(PropertyLawyer::<Test>::get(0).is_none());
    })
}

// withdraw_legal_process_expired function
#[test]
fn withdraw_legal_process_expired_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
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
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().legal_process_expiry, 161);
        run_to_block(162);
        assert_noop!(
            Marketplace::lawyer_confirm_documents(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                false,
            ),
            Error::<Test>::LegalProcessFailed
        );
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_500_000);
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(PropertyLawyer::<Test>::get(0), None);
        assert_eq!(RefundLegalExpired::<Test>::get(0), None);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 0);
        assert_eq!(RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases, 0);
        assert_eq!(RealEstateLawyer::<Test>::get::<AccountId>([11; 32].into()).unwrap().active_cases, 0);
    })
}

#[test]
fn withdraw_legal_process_expired_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::InvalidIndex
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
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            25,
            1337
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().legal_process_expiry,
            161
        );
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::LegalProcessOngoing
        );
        run_to_block(162);
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([3; 32].into()), 0,),
            BadOrigin
        );
    })
}

#[test]
fn second_attempt_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
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
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some([10; 32].into())
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer,
            Some([11; 32].into())
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
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
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::withdraw_rejected(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            6000
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_496_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_147_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 1_197_000);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
    })
}

#[test]
fn lawyer_confirm_documents_fails() {
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
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 3, bvec![10, 10]));
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
            [30; 32].into(),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
		assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3));
		assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3));
		assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([12; 32].into()), 3));
		assert_ok!(Marketplace::list_property(
			RuntimeOrigin::signed([0; 32].into()),
			3,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1984));
        assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984));
        assert_ok!(Marketplace::buy_property_token(RuntimeOrigin::signed([30; 32].into()), 0, 30, 1984));
        assert_ok!(Marketplace::claim_property_token(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_token(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_token(RuntimeOrigin::signed([30; 32].into()), 0));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
        assert_ok!(Marketplace::approve_developer_lawyer(
			RuntimeOrigin::signed([0; 32].into()),
			0,
            true
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
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
			crate::LegalProperty::SpvSide,
			4_000,
		));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
			RuntimeOrigin::signed([1; 32].into()),
			0,
            crate::Vote::Yes,
            40
		));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
			RuntimeOrigin::signed([2; 32].into()),
			0,
            crate::Vote::Yes,
            30
		));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
			RuntimeOrigin::signed([1; 32].into()),
			0,
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
        run_to_block(200);
        assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		), Error::<Test>::LegalProcessFailed);
	})
}

// list_token function
#[test]
fn relist_a_nft() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1000,
            1
        ));
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_eq!(TokenListings::<Test>::get(1).unwrap().item_id, 0);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 39);
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            1
        );
    })
}

#[test]
fn relist_property_token_not_created_with_marketplace_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_noop!(
            Marketplace::relist_token(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1),
            RealEstateAssetError::<Test>::PropertyNotFound
        );
    })
}

#[test]
fn relist_a_nft_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::relist_token(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 10),
            RealEstateAssetError::<Test>::PropertyNotFinalized
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
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
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_noop!(
            Marketplace::relist_token(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1),
            TokenError::FundsUnavailable
        );
        assert_noop!(
            Marketplace::relist_token(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 0),
            Error::<Test>::AmountCannotBeZero
        );
        assert_noop!(
            Marketplace::relist_token(RuntimeOrigin::signed([1; 32].into()), 0, 0, 1),
            Error::<Test>::InvalidTokenPrice
        );
    })
}

// buy_relisted_token function
#[test]
fn buy_relisted_token_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            3,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            47,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            3
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            44
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            4
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 20990000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            8000
        );
        assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 8000);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_468_800);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            1000,
            3
        ));
        assert_ok!(Marketplace::buy_relisted_token(
            RuntimeOrigin::signed([3; 32].into()),
            1,
            2,
            1984
        ));
        assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 3_000);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 2);
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_ok!(Marketplace::buy_relisted_token(
            RuntimeOrigin::signed([3; 32].into()),
            1,
            1,
            1984
        ));
        assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 2_000);
        assert_eq!(TokenListings::<Test>::get(1).is_some(), false);
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            500,
            1
        ));
        assert_ok!(Marketplace::buy_relisted_token(
            RuntimeOrigin::signed([3; 32].into()),
            2,
            1,
            1984
        ));
        assert_eq!(TokenListings::<Test>::get(0).is_some(), false);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 5);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            2
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [3; 32].into()),
            4
        );
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([2; 32].into()),
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
        assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 20990000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            8_000
        );
        assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 8_000);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_084_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
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
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::buy_relisted_token(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1984),
            BadOrigin
        );
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            1_000,
            20
        ));
        assert_eq!(TokenListings::<Test>::get(2).unwrap().amount, 20);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            40
        );
        assert_noop!(
            Marketplace::buy_relisted_token(RuntimeOrigin::signed([1; 32].into()), 2, 1, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
    })
}

// make_offer function
#[test]
fn make_offer_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            500,
            1
        ));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            1,
            1984
        ));
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(),
            true
        );
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
        assert_eq!(
            ForeignAssets::total_balance(1984, &[2; 32].into()),
            1_150_000
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            2000
        );
    })
}

#[test]
fn make_offer_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            200,
            1,
            1984
        ));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([3; 32].into()),
            1,
            300,
            1,
            1984
        ));
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 400, 1, 1984),
            Error::<Test>::OnlyOneOfferPerUser
        );
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into())
                .unwrap()
                .token_price,
            200
        );
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [3; 32].into())
                .unwrap()
                .token_price,
            300
        );
    })
}

// handle_offer function
#[test]
fn handle_offer_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            5000,
            20
        ));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            200,
            1,
            1984
        ));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([3; 32].into()),
            1,
            150,
            1,
            1337
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            200
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1337, &[3; 32].into()),
            150
        );
        assert_ok!(Marketplace::handle_offer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            [2; 32].into(),
            crate::Offer::Reject
        ));
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            0
        );
        assert_ok!(Marketplace::cancel_offer(
            RuntimeOrigin::signed([3; 32].into()),
            1
        ));
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_none(),
            true
        );
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            10,
            1984
        ));
        assert_eq!(
            ForeignAssets::total_balance(1984, &([2; 32].into())),
            1_150_000
        );
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_130_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            20000
        );
        assert_ok!(Marketplace::handle_offer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            [2; 32].into(),
            crate::Offer::Accept
        ));
        assert_eq!(TokenListings::<Test>::get(1).unwrap().amount, 10);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_none(),
            true
        );
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(1)),
            0
        );
        assert_eq!(LocalAssets::balance(0, &([1; 32].into())), 20);
        assert_eq!(LocalAssets::balance(0, &([2; 32].into())), 10);
        assert_eq!(
            ForeignAssets::balance(0, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_103_800);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_130_000);
    })
}

#[test]
fn handle_offer_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            200,
            1,
            1984
        ));
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            500,
            1
        ));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            1,
            1984
        ));
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(),
            true
        );
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
        assert_eq!(
            ForeignAssets::total_balance(1984, &([2; 32].into())),
            1_150_000
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            2000
        );
        assert_ok!(Marketplace::cancel_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1
        ));
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(),
            false
        );
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(1)),
            0
        );
    })
}

#[test]
fn cancel_offer_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            1,
            1984
        ));
        assert_eq!(TokenListings::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(),
            true
        );
        assert_eq!(
            ForeignAssets::total_balance(1984, &([2; 32].into())),
            1_150_000
        );
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            2000
        );
        assert_noop!(
            Marketplace::cancel_offer(RuntimeOrigin::signed([1; 32].into()), 1),
            Error::<Test>::InvalidIndex
        );
    })
}

// upgrade_object function
#[test]
fn upgrade_object_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
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
        assert_ok!(Marketplace::upgrade_object(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            30000
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().token_price,
            30000
        );
    })
}

#[test]
fn upgrade_object_and_distribute_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::upgrade_object(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            20_000
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
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
        assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 21485000);
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::treasury_account_id()),
            13000
        );
        assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 13000);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_084_000);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 318_000);
        assert_eq!(ForeignAssets::balance(1984, &([30; 32].into())), 888_000);

        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
    })
}

#[test]
fn upgrade_object_for_relisted_nft_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            40,
            1984
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
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([0; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
        ),);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([0; 32].into()),
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 300),
            Error::<Test>::TokenNotForSale
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
            RuntimeOrigin::signed([0; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 300),
            Error::<Test>::PropertyAlreadySold
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1000,
            1
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 39);
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            1
        );
        assert_ok!(Marketplace::delist_token(
            RuntimeOrigin::signed([1; 32].into()),
            1
        ));
        assert_eq!(TokenListings::<Test>::get(0), None);
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1000,
            3
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 37);
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            3
        );
        assert_ok!(Marketplace::buy_relisted_token(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            2,
            1984
        ));
        assert_eq!(
            LocalAssets::balance(0, &Marketplace::property_account_id(0)),
            1
        );
        assert_ok!(Marketplace::delist_token(
            RuntimeOrigin::signed([1; 32].into()),
            2
        ));
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 2);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 38);
    })
}

#[test]
fn delist_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::France
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            pallet_regions::Vote::Yes,
            10_000
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            100_000
        ));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::India
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            pallet_regions::Vote::Yes,
            10_000
        ));
        run_to_block(151);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            100_000
        ));
        run_to_block(181);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            4,
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [13; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [14; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [15; 32].into(),
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
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([12; 32].into()),
            2,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([13; 32].into()),
            2,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([14; 32].into()),
            4,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([15; 32].into()),
            4,
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
            2,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            4,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            2,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            2,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            2,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            2,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            2,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([12; 32].into()),
            1,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            true
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([13; 32].into()),
            1,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            crate::Vote::Yes,
            30
        ));
        run_to_block(221);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
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
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            2,
        ));
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
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            2,
            true
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            2,
            crate::Vote::Yes,
            30
        ));
        run_to_block(251);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            2,
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
        assert_eq!(PropertyAssetInfo::<Test>::get(1).unwrap().spv_created, true);
        assert_eq!(PropertyAssetInfo::<Test>::get(2).unwrap().spv_created, true);
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            1000,
            40
        ));
        assert_ok!(Marketplace::buy_relisted_token(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            40,
            1984
        ));
        assert_eq!(LocalAssets::balance(1, &[2; 32].into()), 40);
        assert_eq!(LocalAssets::balance(2, &[2; 32].into()), 40);
    })
}

#[test]
fn cancel_property_purchase_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            40
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
        assert_eq!(
            ForeignAssets::total_balance(1984, &[1; 32].into()),
            1_500_000
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            312_000
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            312_000
        );
        /*         assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(600_000)
        ); */
        assert_ok!(Marketplace::cancel_property_purchase(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        /*         assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(300_000)
        ); */
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            70
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            0
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
    })
}

#[test]
fn cancel_property_purchase_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::InvalidIndex
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::cancel_property_purchase(RuntimeOrigin::signed([3; 32].into()), 0),
            Error::<Test>::TokenOwnerNotFound
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            40,
            1984
        ));
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            70
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
        assert_eq!(
            ForeignAssets::total_balance(1984, &[1; 32].into()),
            1_500_000
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            312_000
        );
        /*         assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .collected_funds
                .get(&1984)
                .copied(),
            Some(300_000)
        ); */
        run_to_block(100);
        assert_ok!(Marketplace::withdraw_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            0
        );
    })
}

#[test]
fn withdraw_expired_works_2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            1_000,
            100,
            bvec![22, 22],
            false
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
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            4,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            46
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0)
                .unwrap()
                .token_amount,
            30
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_468_800);
        assert_eq!(
            ForeignAssets::total_balance(1984, &[1; 32].into()),
            1_500_000
        );
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_129_200);
        assert_eq!(
            ForeignAssets::total_balance(1984, &[2; 32].into()),
            1_150_000
        );
        assert_eq!(ForeignAssets::balance(1984, &[3; 32].into()), 840);
        assert_eq!(ForeignAssets::total_balance(1984, &[3; 32].into()), 5_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            31_200
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            20_800
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[3; 32].into()),
            4_160
        );
        run_to_block(100);
        assert_ok!(Marketplace::withdraw_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            76
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([3; 32].into(), 0)
                .unwrap()
                .token_amount,
            4
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()),
            0
        );
        assert_ok!(Marketplace::withdraw_expired(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_eq!(
            Balances::free_balance(&(Marketplace::property_account_id(0))),
            99
        );
        assert_eq!(
            Balances::balance(&(Marketplace::property_account_id(0))),
            99
        );
        assert_ok!(Marketplace::withdraw_expired(
            RuntimeOrigin::signed([3; 32].into()),
            0
        ));
        assert_eq!(
            Balances::free_balance(&(Marketplace::property_account_id(0))),
            0
        );
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(ForeignAssets::balance(1984, &[3; 32].into()), 5_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[3; 32].into()),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
    })
}

#[test]
fn withdraw_expired_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::InvalidIndex
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
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ListingNotExpired
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
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
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            39,
            1984
        ));
        run_to_block(100);
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::TokenOwnerNotFound
        );
    })
}

#[test]
fn send_property_token_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
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
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_token(
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
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            40
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
        assert_ok!(Marketplace::send_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            [2; 32].into(),
            20
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 20);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 4);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            20
        );
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 20);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [2; 32].into()),
            20
        );
        assert_ok!(Marketplace::send_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            [3; 32].into(),
            20
        ));
        assert_ok!(Marketplace::send_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            [3; 32].into(),
            20
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [2; 32].into()),
            0
        );
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 40);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [3; 32].into()),
            40
        );
    })
}

#[test]
fn send_property_token_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_noop!(
            Marketplace::send_property_token(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                20
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([31; 32].into()),
            0
        ));
        assert_noop!(
            Marketplace::send_property_token(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                20
            ),
            Error::<Test>::UserNotWhitelisted
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::send_property_token(
                RuntimeOrigin::signed([1; 32].into()),
                1,
                [1; 32].into(),
                20
            ),
            Error::<Test>::NoObjectFound
        );
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
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
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
        assert_noop!(
            Marketplace::send_property_token(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                [1; 32].into(),
                5
            ),
            RealEstateAssetError::<Test>::NotEnoughToken
        );
        assert_noop!(
            Marketplace::send_property_token(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                [1; 32].into(),
                30
            ),
            Error::<Test>::ExceedsMaxOwnership
        );
    })
}

#[test]
fn send_property_token_fails_if_relist() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
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
            10_000,
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
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
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
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
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
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
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            20
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 20);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
        assert_ok!(Marketplace::relist_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1000,
            15
        ));
        assert_noop!(
            Marketplace::send_property_token(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                9
            ),
            RealEstateAssetError::<Test>::NotEnoughToken
        );
    })
}

#[test]
fn withdraw_deposit_unsold_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        run_to_block(100);
        assert_ok!(Marketplace::withdraw_deposit_unsold(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
    })
}

#[test]
fn withdraw_deposit_unsold_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        run_to_block(20);
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::ListingNotExpired
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            10,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 1),
            Error::<Test>::InvalidIndex
        );
        run_to_block(100);
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::TokenNotReturned
        );
    })
}

#[test]
fn withdraw_deposit_unsold_fails_2() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        run_to_block(20);
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        run_to_block(100);
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::PropertyAlreadySold
        );
    })
}

// withdraw_unclaimed function
#[test]
fn withdraw_unclaimed_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .listed_token_amount,
            0
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .unclaimed_token_amount,
            30
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 838_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            312_000
        );
        assert!(TokenOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0).is_some());
        assert_ok!(Marketplace::withdraw_unclaimed(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()),
            0
        );
        assert!(TokenOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0).is_none());
    })
}

#[test]
fn withdraw_unclaimed_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::TokenOwnerNotFound
        );
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_unclaimed(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::TokenOwnerNotFound
        );
    })
}

// withdraw_claiming_expired function
#[test]
fn withdraw_claiming_expired_works() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_unclaimed(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(RefundClaimedToken::<Test>::get(0).unwrap(), 70);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 728_000);
        assert_ok!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(
            TokenOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0),
            None
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
    })
}

#[test]
fn withdraw_claiming_expired_fails() {
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
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
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
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ), Error::<Test>::InvalidIndex);
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
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_token(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_unclaimed(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::buy_property_token(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_noop!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ), Error::<Test>::TokenNotRefunded);
        assert_ok!(Marketplace::finalize_claim_window(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([0; 32].into()),
            0,
        ), BadOrigin);
        assert_ok!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
    })
}