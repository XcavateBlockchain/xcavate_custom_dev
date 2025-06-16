use super::*;

use crate as pallet_property_management;
use frame_support::{parameter_types, traits::AsEnsureOriginWithArg, BoundedVec, derive_impl};
use sp_core::{ConstU32, ConstU128};
use sp_runtime::{
	traits::{AccountIdLookup, BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature,
};

use frame_system::EnsureRoot;

use sp_runtime::BuildStorage;

use pallet_nfts::PalletFeatures;

use pallet_assets::{Instance1, Instance2};

use primitives::MarketplaceHoldReason;

pub type Block = frame_system::mocking::MockBlock<Test>;

pub type BlockNumber = u64;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;

/* let id = [0: u32].into();

pub const ALICE: AccountId = id;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DAVE: AccountId = 4;  */

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Nfts: pallet_nfts::{Pallet, Call, Storage, Event<T>},
		PropertyManagement: pallet_property_management,
		NftFractionalization: pallet_nft_fractionalization,
		NftMarketplace: pallet_nft_marketplace,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		LocalAssets: pallet_assets::<Instance1>,
		ForeignAssets: pallet_assets::<Instance2>,
		XcavateWhitelist: pallet_xcavate_whitelist,
		AssetsHolder: pallet_assets_holder::<Instance2>,
		Region: pallet_region,
	}
);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
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
	type MaxConsumers = frame_support::traits::ConstU32<1024>;
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
	type CollectionDeposit = ConstU128<2>;
	type ItemDeposit = ConstU128<1>;
	type MetadataDepositBase = ConstU128<1>;
	type AttributeDepositBase = ConstU128<1>;
	type DepositPerByte = ConstU128<1>;
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
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");

}

impl pallet_assets::Config<Instance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = ConstU128<1>;
	type AssetAccountDeposit = ConstU128<1>;
	type MetadataDepositBase = ConstU128<1>;
	type MetadataDepositPerByte = ConstU128<1>;
	type ApprovalDeposit = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<1000>;
}

impl pallet_assets::Config<Instance2> for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = ConstU128<1>;
	type AssetAccountDeposit = ConstU128<1>;
	type MetadataDepositBase = ConstU128<1>;
	type MetadataDepositPerByte = ConstU128<1>;
	type ApprovalDeposit = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type Holder = AssetsHolder;
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<1000>;
}

impl pallet_assets_holder::Config<pallet_assets::Instance2> for Test {
	type RuntimeHoldReason = MarketplaceHoldReason;
	type RuntimeEvent = RuntimeEvent;
} 

parameter_types! {
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	pub NewAssetSymbol: BoundedVec<u8, ConstU32<50>> = (*b"FRAC").to_vec().try_into().unwrap();
	pub NewAssetName: BoundedVec<u8, ConstU32<50>> = (*b"Frac").to_vec().try_into().unwrap();
}

impl pallet_nft_fractionalization::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Deposit = ConstU128<1>;
	type Currency = Balances;
	type NewAssetSymbol = NewAssetSymbol;
	type NewAssetName = NewAssetName;
	type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type NftId = <Self as pallet_nfts::Config>::ItemId;
	type AssetBalance = <Self as pallet_balances::Config>::Balance;
	type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
	type Assets = LocalAssets;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type WeightInfo = ();
	type StringLimit = ConstU32<50>;
	type RuntimeHoldReason = RuntimeHoldReason;
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
	pub const Postcode: u32 = 10;
	pub const RegionDepositAmount: Balance = 100_000;
	pub const LocationDepositAmount: Balance = 10_000;
	pub const MaximumListingDuration: u64 = 10_000;
}

impl pallet_region::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type NativeCurrency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Nfts = Nfts;
	type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type NftId = <Self as pallet_nfts::Config>::ItemId;
	type RegionDeposit = RegionDepositAmount;
	type PalletId = NftMarketplacePalletId;
	type MaxListingDuration = MaximumListingDuration;
	type PostcodeLimit = Postcode;
	type LocationDeposit = LocationDepositAmount;
}

parameter_types! {
	pub const NftMarketplacePalletId: PalletId = PalletId(*b"py/nftxc");
	pub const MinNftTokens: u32 = 100;
	pub const MaxNftTokens: u32 = 1000;
	pub const MaxNftsInCollection: u32 = 100;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const AcceptedPaymentAssets: [u32; 2] = [1337, 1984];
}

/// Configure the pallet-xcavate-staking in pallets/xcavate-staking.
impl pallet_nft_marketplace::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_nft_marketplace::weights::SubstrateWeight<Test>;
	type Balance = u128;
	type NativeCurrency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LocalCurrency = LocalAssets;
	type ForeignCurrency = ForeignAssets;
	type ForeignAssetsHolder = AssetsHolder;
	type Nfts = Nfts;
	type PalletId = NftMarketplacePalletId;
	type MinNftToken = MinNftTokens;
	type MaxNftToken = MaxNftTokens;
	type NftId = <Self as pallet_nfts::Config>::ItemId;
	type TreasuryId = TreasuryPalletId;
	type FractionalizeCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type FractionalizeItemId = <Self as pallet_nfts::Config>::ItemId;
	type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
	type ListingDeposit = ConstU128<10>;
	type PropertyAccountFundingAmount = ConstU128<100>;
	type MarketplaceFeePercentage = ConstU128<1>;
	type AcceptedAssets = AcceptedPaymentAssets;
}

parameter_types! {
	pub const MaxProperty: u32 = 100;
	pub const MaxLettingAgent: u32 = 100;
	pub const MaxLocation: u32 = 100;
}

/// Configure the pallet-property-management in pallets/property-management.
impl pallet_property_management::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::SubstrateWeight<Test>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type NativeCurrency = Balances;
	type ForeignCurrency = ForeignAssets;
	type MarketplacePalletId = NftMarketplacePalletId;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = AssetHelper;
	type AgentOrigin = EnsureRoot<Self::AccountId>;
	type LettingAgentDeposit = ConstU128<100>;
	type MaxProperties = MaxProperty;
	type MaxLettingAgents = MaxLettingAgent;
	type MaxLocations = MaxLocation;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut test = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			([0; 32].into(), 20_000_000),
			([1; 32].into(), 15_000_000),
			([2; 32].into(), 1_150_000),
			([3; 32].into(), 1_005_000),
			([4; 32].into(), 5_000),
			([6; 32].into(), 200_000),
			((NftMarketplace::account_id()), 20_000_000),
			((PropertyManagement::property_account_id(0)), 5_000),
		],
		dev_accounts: None,
	}
	.assimilate_storage(&mut test)
	.unwrap();

 	pallet_assets::GenesisConfig::<Test, Instance2> {
		assets: vec![(1984, /* account("buyer", SEED, SEED) */ [0; 32].into(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
		metadata: vec![(1984, "USDT".into(), "USDT".into(), 0)], // Genesis metadata: id, name, symbol, decimals
		accounts: vec![
			(1984, [0; 32].into(), 20_000_000),
			(1984, [1; 32].into(), 1_500_000),
			(1984, [2; 32].into(), 1_150_000),
			(1984, [3; 32].into(), 1_150_000),
			(1984, [4; 32].into(), 5_000),
			(1984, [5; 32].into(), 500),
		], // Genesis accounts: id, account_id, balance
		next_asset_id: None,
	}
	.assimilate_storage(&mut test)
	.unwrap(); 

	pallet_assets::GenesisConfig::<Test, Instance2> {
		assets: vec![(1337, /* account("buyer", SEED, SEED) */ [0; 32].into(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
		metadata: vec![(1337, "USDT".into(), "USDT".into(), 0)], // Genesis metadata: id, name, symbol, decimals
		accounts: vec![
			(1337, [4; 32].into(), 5_000),
		], // Genesis accounts: id, account_id, balance
		next_asset_id: None,
	}
	.assimilate_storage(&mut test)
	.unwrap(); 

	test.into()
}
