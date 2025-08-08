use crate::{mock::*, Error};
use crate::{
    traits::{
        PropertyTokenInspect, PropertyTokenManage, PropertyTokenOwnership, PropertyTokenSpvControl,
    },
    PropertyAssetDetails, PropertyAssetInfo, PropertyOwner, PropertyOwnerToken,
};
use frame_support::{
    assert_noop, assert_ok,
    traits::{OnFinalize, OnInitialize},
};
use pallet_regions::RegionIdentifier;
use sp_runtime::{ArithmeticError, Permill, TokenError};

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            RealEstateAsset::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        RealEstateAsset::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RealEstateInvestor
    ));
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        pallet_regions::Vote::Yes
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
    assert_ok!(Regions::create_new_location(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        bvec![10, 10]
    ));
}

#[test]
fn create_property_token_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(
            LocalAssets::balance(0, &RealEstateAsset::property_account_id(0)),
            10
        );
        assert_eq!(
            Nfts::owner(0, 0).unwrap(),
            RealEstateAsset::property_account_id(0)
        );
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                token_amount: 10,
                spv_created: false,
                finalized: false,
            }
        );
    })
}

#[test]
fn burn_property_token_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(
            LocalAssets::balance(0, &RealEstateAsset::property_account_id(0)),
            10
        );
        assert_ok!(RealEstateAsset::burn_property_token(0));
        assert_eq!(
            LocalAssets::balance(0, &RealEstateAsset::property_account_id(0)),
            0
        );
        assert_eq!(Nfts::owner(0, 0).is_none(), true);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
    })
}

#[test]
fn burn_property_token_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_noop!(
            RealEstateAsset::burn_property_token(0),
            Error::<Test>::PropertyAssetNotRegistered
        );
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::do_distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            10
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 10);
        assert_noop!(
            RealEstateAsset::burn_property_token(0),
            TokenError::FundsUnavailable
        );
    })
}

#[test]
fn distribute_property_token_to_owner_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[2; 32].into(),
            6
        ));
        assert_eq!(
            LocalAssets::balance(0, &RealEstateAsset::property_account_id(0)),
            0
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 4);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 6);
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            frame_support::BoundedVec::<_, MaxPropertyTokens>::try_from(vec![
                [1; 32].into(),
                [2; 32].into()
            ])
            .unwrap()
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            4
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [2; 32].into()),
            6
        );
    })
}

#[test]
fn distribute_property_token_to_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(
            LocalAssets::balance(0, &RealEstateAsset::property_account_id(0)),
            10
        );
        assert_noop!(
            RealEstateAsset::distribute_property_token_to_owner(0, &[1; 32].into(), 11),
            ArithmeticError::Underflow
        );
        assert_eq!(
            LocalAssets::balance(0, &RealEstateAsset::property_account_id(0)),
            10
        );
    })
}

#[test]
fn transfer_property_token_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[2; 32].into(),
            6
        ));
        assert_ok!(RealEstateAsset::transfer_property_token(
            0,
            &[2; 32].into(),
            &[2; 32].into(),
            &[3; 32].into(),
            3
        ));
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 3);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 3);
        assert_ok!(RealEstateAsset::transfer_property_token(
            0,
            &[2; 32].into(),
            &[2; 32].into(),
            &[3; 32].into(),
            3
        ));
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            frame_support::BoundedVec::<_, MaxPropertyTokens>::try_from(vec![
                [1; 32].into(),
                [3; 32].into()
            ])
            .unwrap()
        );
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 6);
        assert_ok!(RealEstateAsset::transfer_property_token(
            0,
            &[1; 32].into(),
            &[3; 32].into(),
            &[0; 32].into(),
            3
        ));
        assert_eq!(LocalAssets::balance(0, &[0; 32].into()), 3);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 4);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 3);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [0; 32].into()),
            3
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            1
        );
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [3; 32].into()),
            6
        );
    })
}

#[test]
fn transfer_property_token_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[2; 32].into(),
            6
        ));
        assert_noop!(
            RealEstateAsset::transfer_property_token(
                0,
                &[2; 32].into(),
                &[2; 32].into(),
                &[3; 32].into(),
                7
            ),
            Error::<Test>::NotEnoughToken
        );
        assert_noop!(
            RealEstateAsset::transfer_property_token(
                0,
                &[1; 32].into(),
                &[2; 32].into(),
                &[3; 32].into(),
                6
            ),
            Error::<Test>::NotEnoughToken
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 4);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 6);
    })
}

#[test]
fn take_property_token_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            4
        );
        assert_eq!(RealEstateAsset::take_property_token(0, &[1; 32].into()), 4);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            0
        );
    })
}

#[test]
fn remove_token_ownership_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            4
        );
        assert_eq!(RealEstateAsset::take_property_token(0, &[1; 32].into()), 4);
        assert_eq!(
            PropertyOwnerToken::<Test>::get::<u32, AccountId>(0, [1; 32].into()),
            0
        );
    })
}

#[test]
fn clear_token_owners_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[2; 32].into(),
            6
        ));
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            frame_support::BoundedVec::<_, MaxPropertyTokens>::try_from(vec![
                [1; 32].into(),
                [2; 32].into()
            ])
            .unwrap()
        );
        assert_ok!(RealEstateAsset::clear_token_owners(0));
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            frame_support::BoundedVec::<_, MaxPropertyTokens>::try_from(vec![]).unwrap()
        );
    })
}

#[test]
fn register_spv_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                token_amount: 10,
                spv_created: false,
                finalized: false,
            }
        );
        assert_ok!(RealEstateAsset::register_spv(0));
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                token_amount: 10,
                spv_created: true,
                finalized: false,
            }
        );
    })
}

#[test]
fn register_spv_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_noop!(
            RealEstateAsset::register_spv(0),
            Error::<Test>::PropertyAssetNotRegistered
        );
    })
}

#[test]
fn getter_function_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        new_region_helper();
        assert_ok!(RealEstateAsset::create_property_token(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[1; 32].into(),
            4
        ));
        assert_ok!(RealEstateAsset::distribute_property_token_to_owner(
            0,
            &[2; 32].into(),
            6
        ));
        assert_eq!(
            RealEstateAsset::get_property_asset_info(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                token_amount: 10,
                spv_created: false,
                finalized: false,
            }
        );
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            frame_support::BoundedVec::<_, MaxPropertyTokens>::try_from(vec![
                [1; 32].into(),
                [2; 32].into()
            ])
            .unwrap()
        );
        assert_eq!(RealEstateAsset::take_property_token(0, &[1; 32].into()), 4);
        assert_eq!(RealEstateAsset::get_property_asset_info(1).is_none(), true);
        assert_eq!(
            PropertyOwner::<Test>::get(1),
            frame_support::BoundedVec::<_, MaxPropertyTokens>::try_from(vec![]).unwrap()
        );
        assert_eq!(RealEstateAsset::take_property_token(1, &[3; 32].into()), 0);
    })
}
