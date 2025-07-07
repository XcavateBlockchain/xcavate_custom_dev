use crate::{mock::*, Error};
use frame_support::traits::Currency;
use frame_support::BoundedVec;
use frame_support::{
    assert_noop, assert_ok,
    traits::{OnFinalize, OnInitialize},
};

use crate::{InvestorFunds, LettingInfo, LettingStorage};

use sp_runtime::{Permill, TokenError};

use pallet_marketplace::types::LegalProperty;

use pallet_regions::RegionIdentifier;

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
    assert_ok!(Regions::add_regional_operator(
        RuntimeOrigin::root(),
        [6; 32].into()
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

#[test]
fn add_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [0; 32].into(),
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        let location: BoundedVec<u8, Postcode> = bvec![10, 10];
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations[0],
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
                RuntimeOrigin::signed([6; 32].into()),
                0,
                bvec![10, 10],
                [0; 32].into(),
            ),
            Error::<Test>::RegionUnknown
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([6; 32].into()),
                3,
                bvec![10, 10],
                [0; 32].into(),
            ),
            Error::<Test>::LocationUnknown
        );
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [0; 32].into(),
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([6; 32].into()),
                3,
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [0; 32].into(),
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .deposited,
            false
        );
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([0; 32].into())
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .deposited,
            true
        );
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
    });
}

#[test]
fn let_letting_agent_deposit_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [0; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([0; 32].into())
        ));
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
                RuntimeOrigin::signed([6; 32].into()),
                3,
                bvec![10, 10],
                [x; 32].into(),
            ));
            Balances::make_free_balance_be(&[x; 32].into(), 200);
            assert_ok!(PropertyManagement::letting_agent_deposit(
                RuntimeOrigin::signed([x; 32].into())
            ));
        }
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [100; 32].into(),
        ));
        Balances::make_free_balance_be(&[100; 32].into(), 200);
    });
}

#[test]
fn let_letting_agent_deposit_not_enough_funds() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![9, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![9, 10],
            [0; 32].into(),
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([0; 32].into())
        ));
        assert_ok!(PropertyManagement::add_letting_agent_to_location(
            RuntimeOrigin::signed([6; 32].into()),
            bvec![10, 10],
            [0; 32].into()
        ));
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
                RuntimeOrigin::signed([6; 32].into()),
                bvec![10, 10],
                [0; 32].into()
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![9, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [0; 32].into(),
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(),
            true
        );
        assert_noop!(
            PropertyManagement::add_letting_agent_to_location(
                RuntimeOrigin::signed([7; 32].into()),
                bvec![10, 10],
                [0; 32].into()
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyManagement::add_letting_agent_to_location(
                RuntimeOrigin::signed([6; 32].into()),
                bvec![10, 10],
                [0; 32].into()
            ),
            Error::<Test>::NotDeposited
        );
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([0; 32].into())
        ));
        assert_noop!(
            PropertyManagement::add_letting_agent_to_location(
                RuntimeOrigin::signed([6; 32].into()),
                bvec![5, 10],
                [0; 32].into()
            ),
            Error::<Test>::LocationUnknown
        );
        assert_noop!(
            PropertyManagement::add_letting_agent_to_location(
                RuntimeOrigin::signed([6; 32].into()),
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(Marketplace::list_object(
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
        assert_ok!(Marketplace::list_object(
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
            100,
            1984
        ));
        assert_ok!(Marketplace::list_object(
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
            2,
            100,
            1984
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [2; 32].into(),
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [3; 32].into(),
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [4; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([2; 32].into())
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([3; 32].into())
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([4; 32].into())
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            1
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            2
        ));
        assert_ok!(Marketplace::list_object(
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
            3,
            100,
            1984
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            3
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());
        assert_eq!(LettingStorage::<Test>::get(1).unwrap(), [3; 32].into());
        assert_eq!(LettingStorage::<Test>::get(2).unwrap(), [4; 32].into());
        assert_eq!(LettingStorage::<Test>::get(3).unwrap(), [2; 32].into());
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::add_regional_operator(
            RuntimeOrigin::root(),
            [0; 32].into()
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            pallet_regions::Vote::Yes
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            100_000
        ));
        run_to_block(61);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [1; 32].into()
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            [0; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([0; 32].into())
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_889_900);
        assert_noop!(
            PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::NoObjectFound
        );
        assert_ok!(Marketplace::list_object(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            100,
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
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_noop!(
            PropertyManagement::set_letting_agent(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::LettingAgentAlreadySet
        );
        for x in 1..100 {
            assert_ok!(Marketplace::list_object(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                1_000,
                100,
                bvec![22, 22],
                false
            ));
            assert_ok!(XcavateWhitelist::add_to_whitelist(
                RuntimeOrigin::root(),
                [(x + 1); 32].into()
            ));
            Balances::make_free_balance_be(&[x + 1; 32].into(), 100_000);
            assert_ok!(ForeignAssets::mint(
                RuntimeOrigin::signed([0; 32].into()),
                1984.into(),
                sp_runtime::MultiAddress::Id([(x + 1); 32].into()),
                1_000_000,
            ));
            assert_ok!(Marketplace::buy_property_token(
                RuntimeOrigin::signed([(x + 1); 32].into()),
                (x as u32).into(),
                100,
                1984
            ));
            assert_ok!(PropertyManagement::set_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                x.into()
            ));
        }
        assert_ok!(Marketplace::list_object(
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
            100,
            100,
            1984
        ));
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(Marketplace::list_object(
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [2; 32].into()
        ));
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [3; 32].into()
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            [10; 32].into()
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            [11; 32].into()
        ));
        assert_ok!(Marketplace::list_object(
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            LegalProperty::SpvSide,
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [4; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([4; 32].into())
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            0
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
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(Marketplace::list_object(
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
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [4; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([4; 32].into())
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            0
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
fn withdraw_funds_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            [10; 32].into()
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            [11; 32].into()
        ));
        assert_ok!(Marketplace::list_object(
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            LegalProperty::SpvSide,
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [4; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([4; 32].into())
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            0
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
        assert_eq!(Balances::free_balance(&([4; 32].into())), 4900);
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
        assert_ok!(PropertyManagement::withdraw_funds(
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
fn withdraw_funds_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_to_whitelist(
            RuntimeOrigin::root(),
            [6; 32].into()
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
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
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            [10; 32].into()
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            [11; 32].into()
        ));
        assert_ok!(Marketplace::list_object(
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
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            LegalProperty::SpvSide,
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
        assert_eq!(LocalAssets::total_supply(0), 1000);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10],
            [4; 32].into(),
        ));
        assert_ok!(PropertyManagement::letting_agent_deposit(
            RuntimeOrigin::signed([4; 32].into())
        ));
        assert_ok!(PropertyManagement::set_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            0
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
            PropertyManagement::withdraw_funds(RuntimeOrigin::signed([2; 32].into()), 0, 1984),
            Error::<Test>::UserHasNoFundsStored
        );
        assert_noop!(
            PropertyManagement::withdraw_funds(RuntimeOrigin::signed([1; 32].into()), 0, 1),
            Error::<Test>::PaymentAssetNotSupported
        );
    });
}
