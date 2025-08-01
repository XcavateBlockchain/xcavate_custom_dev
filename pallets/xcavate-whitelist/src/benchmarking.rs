//! Benchmarking setup for pallet-whitelist
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Whitelist;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn add_to_whitelist() {
        let user: T::AccountId = account("user", 0, 0);
        #[extrinsic_call]
        add_to_whitelist(RawOrigin::Root, user.clone());

        assert_eq!(Whitelist::<T>::is_whitelisted(&user), true);
    }

    #[benchmark]
    fn remove_from_whitelist() {
        let user: T::AccountId = account("user", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            user.clone()
        ));
        assert_eq!(Whitelist::<T>::is_whitelisted(&user), true);
        #[extrinsic_call]
        remove_from_whitelist(RawOrigin::Root, user.clone());

        assert_eq!(Whitelist::<T>::is_whitelisted(&user), false);
    }

    impl_benchmark_test_suite!(Whitelist, crate::mock::new_test_ext(), crate::mock::Test);
}
