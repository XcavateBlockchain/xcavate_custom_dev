#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Role enum.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(
        Encode,
        Decode,
        DecodeWithMemTracking,
        Clone,
        PartialEq,
        Eq,
        MaxEncodedLen,
        RuntimeDebug,
        TypeInfo,
    )]
    pub enum Role {
        RegionalOperator,
        RealEstateInvestor,
        RealEstateDeveloper,
        Lawyer,
        LettingAgent,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;
        /// Origin who can add and remove users to the whitelist.
        type WhitelistOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Max users allowed in the whitelist.
        #[pallet::constant]
        type MaxUsersInWhitelist: Get<u32>;
    }

    /// Mapping of the accounts to the assigned roles.
    #[pallet::storage]
    pub type AccountRoles<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        Role,
        (),
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new role has been assigned to a user.
        RoleAssigned { user: T::AccountId, role: Role },
        /// A role has been removed from a user.
        RoleRemoved { user: T::AccountId, role: Role },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The role has already been assigned to the usser.
        RoleAlreadyAssigned,
        /// The role has not been assigned to the user.
        RoleNotAssigned,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Adds a role to a user.
        ///
        /// The origin must be the sudo.
        ///
        /// Parameters:
        /// - `user`: The address of the accounts that gets a new role.
        /// - `role`: The role that is getting assigned to the user.
        ///
        /// Emits `RoleAssigned` event when succesfful
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn assign_role(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
        ) -> DispatchResult {
            T::WhitelistOrigin::ensure_origin(origin)?;
            ensure!(
                !AccountRoles::<T>::contains_key(&user, &role),
                Error::<T>::RoleAlreadyAssigned
            );
            AccountRoles::<T>::insert(&user, role.clone(), ());
            Self::deposit_event(Event::<T>::RoleAssigned { user, role });
            Ok(())
        }

        /// Removes a role from a user.
        ///
        /// The origin must be the sudo.
        ///
        /// Parameters:
        /// - `user`: The address of the accounts that gets a role removed.
        /// - `role`: The role that is getting removed from the user.
        ///
        /// Emits `UserRemoved` event when succesfful
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn remove_role(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
        ) -> DispatchResult {
            T::WhitelistOrigin::ensure_origin(origin)?;
            ensure!(
                AccountRoles::<T>::contains_key(&user, &role),
                Error::<T>::RoleNotAssigned
            );
            AccountRoles::<T>::remove(&user, role.clone());
            Self::deposit_event(Event::<T>::RoleRemoved { user, role });
            Ok(())
        }
    }
}

pub trait HasRole<AccountId> {
    fn has_role(account: &AccountId, role: Role) -> bool;
}

impl<T: Config> HasRole<T::AccountId> for Pallet<T> {
    fn has_role(account: &T::AccountId, role: Role) -> bool {
        AccountRoles::<T>::contains_key(account, role)
    }
}
