use crate::{mock::*, Error, HasRole, Role, AdminAccounts};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

#[test]
fn add_admin_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 1));
        assert_eq!(AdminAccounts::<Test>::get(&1).unwrap(), ());
    });
}

#[test]
fn add_admin_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Whitelist::add_admin(RuntimeOrigin::signed(2), 1),
            BadOrigin
        );
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 1));
        assert_noop!(
            Whitelist::add_admin(RuntimeOrigin::root(), 1),
            Error::<Test>::AlreadyAdmin
        );
    });
}

#[test]
fn remove_admin_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 1));
        assert_eq!(AdminAccounts::<Test>::get(&1).unwrap(), ());
        assert_ok!(Whitelist::remove_admin(RuntimeOrigin::root(), 1));
        assert_eq!(AdminAccounts::<Test>::get(&1), None);
    });
}

#[test]
fn remove_admin_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Whitelist::remove_admin(RuntimeOrigin::signed(2), 1),
            BadOrigin
        );
        assert_noop!(
            Whitelist::remove_admin(RuntimeOrigin::root(), 1),
            Error::<Test>::AccountNotAdmin
        );
    });
}

#[test]
fn assign_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::Lawyer));
        assert_eq!(Whitelist::has_role(&1, Role::Lawyer), true);
        assert_eq!(Whitelist::has_role(&1, Role::LettingAgent), false);
    });
}

#[test]
fn assign_role_fails_when_user_already_added() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::LettingAgent));
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::LettingAgent),
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
            Error::<Test>::AccountNotAdmin
        );
    });
}

#[test]
fn remove_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor));
        assert_ok!(Whitelist::remove_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor));
        assert_eq!(Whitelist::has_role(&1, Role::RealEstateInvestor), false);
    });
}

#[test]
fn remove_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor));
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::signed(2), 1, Role::RealEstateInvestor),
            Error::<Test>::AccountNotAdmin
        );
    });
}

#[test]
fn remove_role_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor),
            Error::<Test>::RoleNotAssigned
        );
    });
}
