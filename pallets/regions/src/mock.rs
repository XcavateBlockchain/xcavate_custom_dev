use super::*;

use frame_support::{derive_impl, parameter_types, traits::AsEnsureOriginWithArg};
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, ConstU128, ConstU32, IdentifyAccount, Verify},
    BuildStorage, MultiSignature,
};

use pallet_nfts::PalletFeatures;

pub type Block = frame_system::mocking::MockBlock<Test>;

pub type BlockNumber = u64;

pub type Balance = u128;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// Configure a mock runtime to test the pallet.
#[frame_support::runtime]
mod test_runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Test;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(2)]
    pub type Nfts = pallet_nfts;
    #[runtime::pallet_index(3)]
    pub type XcavateWhitelist = pallet_xcavate_whitelist;
    #[runtime::pallet_index(4)]
    pub type Regions = crate;
}

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type RuntimeCall = RuntimeCall;
    type Nonce = u32;
    type Block = Block;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = frame_support::traits::Everything;
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type RuntimeTask = ();
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub Features: PalletFeatures = PalletFeatures::all_enabled();
    pub const ApprovalsLimit: u32 = 20;
    pub const ItemAttributesApprovalsLimit: u32 = 20;
    pub const MaxTips: u32 = 10;
    pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
    pub const MaxAttributesPerCall: u32 = 10;
}

impl pallet_nfts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type Locker = ();
    type CollectionDeposit = ConstU128<0>;
    type ItemDeposit = ConstU128<0>;
    type MetadataDepositBase = ConstU128<0>;
    type AttributeDepositBase = ConstU128<0>;
    type DepositPerByte = ConstU128<0>;
    type StringLimit = ConstU32<50>;
    type KeyLimit = ConstU32<50>;
    type ValueLimit = ConstU32<50>;
    type WeightInfo = ();
    type ApprovalsLimit = ApprovalsLimit;
    type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
    type MaxTips = MaxTips;
    type MaxDeadlineDuration = MaxDeadlineDuration;
    type MaxAttributesPerCall = MaxAttributesPerCall;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = AccountPublic;
    type BlockNumberProvider = System;
}

parameter_types! {
    pub const MaxWhitelistUsers: u32 = 1000000;
}

impl pallet_xcavate_whitelist::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_xcavate_whitelist::weights::SubstrateWeight<Test>;
    type WhitelistOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type MaxUsersInWhitelist = MaxWhitelistUsers;
}

parameter_types! {
    pub const MarketplacePalletId: PalletId = PalletId(*b"py/nftxc");
    pub const Postcode: u32 = 10;
    pub const LocationDepositAmount: Balance = 1_000;
    pub const MaximumListingDuration: BlockNumber = 10_000;
    pub const RegionVotingTime: BlockNumber = 30;
    pub const RegionAuctionTime: BlockNumber = 30;
    pub const RegionThreshold: Percent = Percent::from_percent(75);
    pub const RegionProposalCooldown: BlockNumber = 28;
    pub const RegionOperatorVotingTime: BlockNumber = 30;
    pub const RegionOwnerChangeTime: BlockNumber = 300;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const RegionOwnerNoticeTime: BlockNumber = 100;
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::SubstrateWeight<Test>;
    type Balance = u128;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = Nfts;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type PalletId = MarketplacePalletId;
    type MaxListingDuration = MaximumListingDuration;
    type PostcodeLimit = Postcode;
    type LocationDeposit = LocationDepositAmount;
    type RegionVotingTime = RegionVotingTime;
    type RegionAuctionTime = RegionAuctionTime;
    type RegionThreshold = RegionThreshold;
    type RegionProposalCooldown = RegionProposalCooldown;
    type RegionOperatorOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type RegionOperatorVotingTime = RegionOperatorVotingTime;
    type MaxProposalsForBlock = ConstU32<100>;
    type RegionSlashingAmount = ConstU128<10_000>;
    type TreasuryId = TreasuryPalletId;
    type RegionOwnerChangePeriod = RegionOwnerChangeTime;
    type Slash = ();
    type RegionOwnerNoticePeriod = RegionOwnerNoticeTime;
    type RegionOwnerDisputeDeposit = ConstU128<1_000>;
    type MinimumRegionDeposit = ConstU128<10_000>;
    type RegionProposalDeposit = ConstU128<5_000>;
    type MinimumVotingAmount = ConstU128<100>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut test = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            ([0; 32].into(), 200_000),
            ([1; 32].into(), 150_000),
            ([2; 32].into(), 300_000),
            ([3; 32].into(), 5_000),
            ([8; 32].into(), 400_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    test.into()
}
