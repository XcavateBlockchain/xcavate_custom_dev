use crate::{mock::*, Error, HasRole, Role};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

#[test]
fn assign_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::root(), 1, Role::Lawyer,));
        assert_eq!(Whitelist::has_role(&1, Role::Lawyer), true);
        assert_eq!(Whitelist::has_role(&1, Role::LettingAgent), false);
    });
}

#[test]
fn assign_role_fails_when_user_already_added() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::root(), 1, Role::LettingAgent));
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::root(), 1, Role::LettingAgent),
            Error::<Test>::RoleAlreadyAssigned
        );
    });
}

#[test]
fn assign_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::signed(2), 1, Role::LettingAgent),
            BadOrigin
        );
    });
}

#[test]
fn remove_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::root(), 1, Role::RealEstateInvestor));
        assert_ok!(Whitelist::remove_role(RuntimeOrigin::root(), 1, Role::RealEstateInvestor));
        assert_eq!(Whitelist::has_role(&1, Role::RealEstateInvestor), false);
    });
}

#[test]
fn remove_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::root(), 1, Role::RealEstateInvestor));
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::signed(2), 1, Role::RealEstateInvestor),
            BadOrigin
        );
    });
}

#[test]
fn remove_role_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::root(), 1, Role::RealEstateInvestor),
            Error::<Test>::RoleNotAssigned
        );
    });
}
