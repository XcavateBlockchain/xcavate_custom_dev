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
        SpvConfirmation,
    }

    /// Access permission enum.
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
    pub enum AccessPermission {
        Revoked,
        Compliant,
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

    /// Mapping of the admin accounts.
    #[pallet::storage]
    pub type AdminAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, (), OptionQuery>;

    /// Mapping of the accounts to the assigned roles.
    #[pallet::storage]
    pub type AccountRoles<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        Role,
        AccessPermission,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new role has been assigned to a user.
        RoleAssigned { user: T::AccountId, role: Role },
        /// A role has been removed from a user.
        RoleRemoved { user: T::AccountId, role: Role },
        /// A new admin has been registered.
        AdminRegistered { admin: T::AccountId },
        /// An admin has been removed.
        AdminRemoved { admin: T::AccountId },
        /// The permission of an account has been updated.
        PermissionUpdated {
            user: T::AccountId,
            role: Role,
            permission: AccessPermission,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The role has already been assigned to the usser.
        RoleAlreadyAssigned,
        /// The role has not been assigned to the user.
        RoleNotAssigned,
        /// The acount is already registered as an admin.
        AlreadyAdmin,
        /// The acount is not registered as an admin.
        AccountNotAdmin,
        /// This permission has already been set.
        PermissionAlreadySet,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Add a new whitelist admin.
        ///
        /// The origin must be the sudo.
        ///
        /// Parameters:
        /// - `user`: The address of the accounts that is added as an admin.
        ///
        /// Emits `RoleAssigned` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn add_admin(origin: OriginFor<T>, admin: AccountIdOf<T>) -> DispatchResult {
            T::WhitelistOrigin::ensure_origin(origin)?;
            ensure!(
                !AdminAccounts::<T>::contains_key(&admin),
                Error::<T>::AlreadyAdmin
            );
            AdminAccounts::<T>::insert(&admin, ());
            Self::deposit_event(Event::<T>::AdminRegistered { admin });
            Ok(())
        }

        /// Remove an existing whitelist admin.
        ///
        /// The origin must be the sudo.
        ///
        /// Parameters:
        /// - `user`: The address of the accounts that is added as an admin.
        ///
        /// Emits `RoleAssigned` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn remove_admin(origin: OriginFor<T>, admin: AccountIdOf<T>) -> DispatchResult {
            T::WhitelistOrigin::ensure_origin(origin)?;
            ensure!(
                AdminAccounts::<T>::contains_key(&admin),
                Error::<T>::AccountNotAdmin
            );
            AdminAccounts::<T>::remove(&admin);
            Self::deposit_event(Event::<T>::AdminRemoved { admin });
            Ok(())
        }

        /// Assign a role to a user with default 'Compliant' permission.
        ///
        /// The origin must be an admin.
        ///
        /// Parameters:
        /// - `user`: The address of the account that gets a new role.
        /// - `role`: The role that is getting assigned to the user.
        ///
        /// Emits `RoleAssigned` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn assign_role(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                AdminAccounts::<T>::contains_key(&signer),
                Error::<T>::AccountNotAdmin
            );
            ensure!(
                !AccountRoles::<T>::contains_key(&user, &role),
                Error::<T>::RoleAlreadyAssigned
            );
            AccountRoles::<T>::insert(&user, role.clone(), AccessPermission::Compliant);
            Self::deposit_event(Event::<T>::RoleAssigned { user, role });
            Ok(())
        }

        /// Remove a role from a user.
        ///
        /// The origin must be an admin.
        ///
        /// Parameters:
        /// - `user`: The address of the account that gets a role removed.
        /// - `role`: The role that is getting removed from the user.
        ///
        /// Emits `RoleRemoved` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn remove_role(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                AdminAccounts::<T>::contains_key(&signer),
                Error::<T>::AccountNotAdmin
            );
            ensure!(
                AccountRoles::<T>::contains_key(&user, &role),
                Error::<T>::RoleNotAssigned
            );
            AccountRoles::<T>::remove(&user, role.clone());
            Self::deposit_event(Event::<T>::RoleRemoved { user, role });
            Ok(())
        }

        /// Update a user's permission for a role.
        ///
        /// The origin must be an admin.
        ///
        /// Parameters:
        /// - `user`: The address of the account that gets the permission updated.
        /// - `role`: The role that is getting the permission updated.
        /// - `permission`: The new permission state.
        ///
        /// Emits `PermissionUpdated` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
        pub fn set_permission(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
            permission: AccessPermission,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(
                AdminAccounts::<T>::contains_key(&signer),
                Error::<T>::AccountNotAdmin
            );
            let current_role =
                AccountRoles::<T>::get(&user, &role).ok_or(Error::<T>::RoleNotAssigned)?;
            ensure!(current_role != permission, Error::<T>::PermissionAlreadySet);

            AccountRoles::<T>::insert(&user, role.clone(), permission.clone());
            Self::deposit_event(Event::<T>::PermissionUpdated {
                user,
                role,
                permission,
            });
            Ok(())
        }
    }
}

pub trait RolePermission<AccountId> {
    fn has_role(account: &AccountId, role: Role) -> bool;

    fn is_compliant(account: &AccountId, role: Role) -> bool;
}

impl<T: Config> RolePermission<T::AccountId> for Pallet<T> {
    fn has_role(account: &T::AccountId, role: Role) -> bool {
        AccountRoles::<T>::contains_key(account, role)
    }

    fn is_compliant(account: &T::AccountId, role: Role) -> bool {
        AccountRoles::<T>::get(account, role)
            .map_or(false, |access| access == AccessPermission::Compliant)
    }
}
