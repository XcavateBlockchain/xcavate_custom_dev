// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org>

mod xcm_config;

// Substrate and Polkadot dependencies
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    instances::Instance1,
    parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse,
        EnsureOriginWithArg, InstanceFilter, OriginTrait, TransformOrigin, VariantCountOf,
    },
    weights::{ConstantMultiplier, Weight},
    BoundedVec, PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureRootWithSuccess, EnsureSigned,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::{
    impls::AssetsToBlockAuthor,
    message_queue::{NarrowOriginToSibling, ParaIdToSibling},
};
use polkadot_runtime_common::{
    xcm_sender::NoPriceForMessageDelivery, BlockHashCount, SlowAdjustingFeeUpdate,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{
    traits::{BlakeTwo256, ConvertInto, Verify},
    MultiSignature, Perbill, Percent, RuntimeDebug,
};
use sp_version::RuntimeVersion;
use xcm::latest::prelude::BodyId;

// Local module imports
use super::{
    deposit,
    weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
    AccountId, Assets, AssetsHolder, Aura, Balance, Balances, Block, BlockNumber,
    CollatorSelection, ConsensusHook, Hash, MessageQueue, Nfts, Nonce, OriginCaller, PalletInfo,
    ParachainSystem, RealEstateAsset, RealEstateAssets, Runtime, RuntimeCall, RuntimeEvent,
    RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask, Session, SessionKeys,
    System, WeightToFee, XcavateWhitelist, XcmpQueue, AVERAGE_ON_INITIALIZE_RATIO, DAYS,
    EXISTENTIAL_DEPOSIT, HOURS, MAXIMUM_BLOCK_WEIGHT, MICROUNIT, NORMAL_DISPATCH_RATIO,
    SLOT_DURATION, UNIT, VERSION,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pallet_nfts::PalletFeatures;
use primitives::MarketplaceHoldReason;
use scale_info::TypeInfo;
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;

    // This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
    //  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
    // `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
    // the lazy contract deletion.
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 42;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`ParaChainDefaultConfig`](`struct@frame_system::config_preludes::ParaChainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The index type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The block type.
    type Block = Block;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The action to take on a Runtime Upgrade
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type DoneSlashHandler = ();
}

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
    type ConsensusHook = ConsensusHook;
    type SelectCore = cumulus_pallet_parachain_system::DefaultCoreSelector<Runtime>;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
    pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
        cumulus_primitives_core::AggregateMessageOrigin,
    >;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MessageProcessor = xcm_builder::ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
        RuntimeCall,
    >;
    type Size = u32;
    // The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
    type MaxStale = sp_core::ConstU32<8>;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = ();
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = ();
    // Enqueue XCMP messages from siblings for later processing.
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = sp_core::ConstU32<1_000>;
    type MaxActiveOutboundChannels = ConstU32<128>;
    type MaxPageSize = ConstU32<{ 1 << 16 }>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = ();
    type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    // Essentially just Aura, but let's be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type DisablingStrategy = ();
    type WeightInfo = ();
}

#[docify::export(aura_config)]
impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100_000>;
    type AllowMultipleBlocksPerSlot = ConstBool<true>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const SessionLength: BlockNumber = 6 * HOURS;
    // StakingAdmin pluralistic body.
    pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
    EnsureRoot<AccountId>,
    EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
>;

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type PotId = PotId;
    type MaxCandidates = ConstU32<100>;
    type MinEligibleCollators = ConstU32<4>;
    type MaxInvulnerables = ConstU32<20>;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxProxies: u32 = 32;
    pub const MaxPending: u32 = 32;
    pub const ProxyDepositBase: Balance = deposit(1, 40);
    pub const AnnouncementDepositBase: Balance = deposit(1, 48);
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
/// If you are adding new pallets, consider adding new ProxyType variant
#[derive(
    Copy,
    Clone,
    Decode,
    DecodeWithMemTracking,
    Default,
    Encode,
    Eq,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    RuntimeDebug,
    TypeInfo,
)]
pub enum ProxyType {
    /// Allows to proxy all calls
    #[default]
    Any,
    /// Allows all non-transfer calls
    NonTransfer,
    /// Allows to finish the proxy
    CancelProxy,
    /// Allows to operate with collators list (invulnerables, candidates, etc.)
    Collator,
}

impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => !matches!(c, RuntimeCall::Balances { .. }),
            ProxyType::CancelProxy => matches!(
                c,
                RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                    | RuntimeCall::Multisig { .. }
            ),
            ProxyType::Collator => {
                matches!(
                    c,
                    RuntimeCall::CollatorSelection { .. } | RuntimeCall::Multisig { .. }
                )
            }
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
    type CallHasher = BlakeTwo256;
    type Currency = Balances;
    type MaxPending = MaxPending;
    type MaxProxies = MaxProxies;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type ProxyType = ProxyType;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 20;
}

impl pallet_multisig::Config for Runtime {
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = ();
}

parameter_types! {
    pub const AssetDeposit: Balance = 10 * UNIT;
    pub const AssetAccountDeposit: Balance = deposit(1, 16);
    pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = deposit(1, 68);
    pub const MetadataDepositPerByte: Balance = deposit(0, 1);
    pub const RemoveItemsLimit: u32 = 1000;
    pub const ZeroDeposit: Balance = 0;
    pub RootAccountId: AccountId = AccountId::from([0xffu8; 32]);
}

impl pallet_assets::Config<pallet_assets::Instance1> for Runtime {
    type ApprovalDeposit = ApprovalDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type AssetDeposit = ZeroDeposit;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Balance = Balance;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type CallbackHandle = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Currency = Balances;
    type Extra = ();
    type ForceOrigin = EnsureRoot<AccountId>;
    type Freezer = ();
    type Holder = ();
    type MetadataDepositBase = ZeroDeposit;
    type MetadataDepositPerByte = ZeroDeposit;
    type RemoveItemsLimit = RemoveItemsLimit;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = StringLimit;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = ();
}

impl pallet_assets::Config<pallet_assets::Instance2> for Runtime {
    type ApprovalDeposit = ApprovalDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type AssetDeposit = AssetDeposit;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Balance = Balance;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type CallbackHandle = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Currency = Balances;
    type Extra = ();
    type ForceOrigin = EnsureRoot<AccountId>;
    type Freezer = ();
    type Holder = AssetsHolder;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type RemoveItemsLimit = RemoveItemsLimit;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = StringLimit;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = ();
}

impl pallet_assets_holder::Config<pallet_assets::Instance2> for Runtime {
    type RuntimeHoldReason = MarketplaceHoldReason;
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub Features: PalletFeatures = PalletFeatures::all_enabled();
    pub const MaxAttributesPerCall: u32 = 10;
    pub const CollectionDeposit: Balance = UNIT;
    pub const ItemDeposit: Balance = UNIT;
    pub const KeyLimit: u32 = 32;
    pub const ValueLimit: u32 = 256;
    pub const ApprovalsLimit: u32 = 20;
    pub const ItemAttributesApprovalsLimit: u32 = 20;
    pub const MaxTips: u32 = 10;
    pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;

    pub const UserStringLimit: u32 = 5;

}

impl pallet_nfts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type CollectionDeposit = CollectionDeposit;
    type ItemDeposit = ItemDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type AttributeDepositBase = MetadataDepositBase;
    type DepositPerByte = MetadataDepositPerByte;
    type StringLimit = StringLimit;
    type KeyLimit = KeyLimit;
    type ValueLimit = ValueLimit;
    type ApprovalsLimit = ApprovalsLimit;
    type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
    type MaxTips = MaxTips;
    type MaxDeadlineDuration = MaxDeadlineDuration;
    type MaxAttributesPerCall = MaxAttributesPerCall;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = <Signature as Verify>::Signer;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
    //type CreateOrigin = AsEnsureOriginWithArg<EnsureSignedBy<CollectionCreationOrigin, AccountId>>;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type Locker = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

parameter_types! {
    pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
    pub NewAssetSymbol: BoundedVec<u8, StringLimit> = (*b"BRIX").to_vec().try_into().unwrap();
    pub NewAssetName: BoundedVec<u8, StringLimit> = (*b"Brix").to_vec().try_into().unwrap();
    pub const Deposit: Balance = UNIT;
}

impl pallet_nft_fractionalization::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Deposit = Deposit;
    type Currency = Balances;
    type NewAssetSymbol = NewAssetSymbol;
    type NewAssetName = NewAssetName;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type AssetBalance = <Self as pallet_balances::Config>::Balance;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type Assets = RealEstateAssets;
    type Nfts = Nfts;
    type PalletId = NftFractionalizationPalletId;
    type WeightInfo = ();
    type StringLimit = StringLimit;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type RuntimeHoldReason = RuntimeHoldReason;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetTxHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_asset_tx_payment::BenchmarkHelperTrait<AccountId, u32, u32> for AssetTxHelper {
    fn create_asset_id_parameter(_id: u32) -> (u32, u32) {
        unimplemented!("Penpal uses default weights");
    }
    fn setup_balances_and_pool(_asset_id: u32, _account: AccountId) {
        unimplemented!("Penpal uses default weights");
    }
}

impl pallet_asset_tx_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Fungibles = Assets;
    type OnChargeAssetTransaction = pallet_asset_tx_payment::FungiblesAdapter<
        pallet_assets::BalanceToAssetBalance<
            Balances,
            Runtime,
            ConvertInto,
            pallet_assets::Instance2,
        >,
        AssetsToBlockAuthor<Runtime, pallet_assets::Instance2>,
    >;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = AssetTxHelper;
}

parameter_types! {
    pub const MarketplacePalletId: PalletId = PalletId(*b"py/nftxc");
    pub const MinPropertyTokens: u32 = 100;
    pub const MaxPropertyTokens: u32 = 250;
    pub const ListingDepositAmount: Balance = 10 * MICROUNIT;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const PropertyFundingAmount: Balance = 10 * UNIT;
    pub const MarketplaceFeePercent: Balance = 1;
    pub const MaximumAcceptedAssets: u32 = 2;
    pub const AcceptedPaymentAssets: [u32; 2] = [1337, 1984];
    pub const LawyerVotingDuration: BlockNumber = 20;
    pub const LegalProcessDuration: BlockNumber = 30;
}

/// Configure the pallet-marketplace in pallets/marketplace.
impl pallet_marketplace::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_marketplace::weights::SubstrateWeight<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type LocalCurrency = RealEstateAssets;
    type ForeignCurrency = Assets;
    type ForeignAssetsHolder = AssetsHolder;
    type PalletId = MarketplacePalletId;
    type MinPropertyToken = MinPropertyTokens;
    type MaxPropertyToken = MaxPropertyTokens;
    type TreasuryId = TreasuryPalletId;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type ListingDeposit = ListingDepositAmount;
    type MarketplaceFeePercentage = MarketplaceFeePercent;
    type AcceptedAssets = AcceptedPaymentAssets;
    type MaxAcceptedAssets = MaximumAcceptedAssets;
    type PropertyToken = RealEstateAsset;
    type LawyerVotingTime = LawyerVotingDuration;
    type LegalProcessTime = LegalProcessDuration;
    type Whitelist = XcavateWhitelist;
    type PermissionOrigin = EnsurePermission<Self>;
}

parameter_types! {
    pub const MaxWhitelistUsers: u32 = 1000;
}

/// Configure the pallet-xcavate-whitelist in pallets/xcavate-whitelist.
impl pallet_xcavate_whitelist::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_xcavate_whitelist::weights::SubstrateWeight<Runtime>;
    type WhitelistOrigin = EnsureRoot<Self::AccountId>;
    type MaxUsersInWhitelist = MaxWhitelistUsers;
}

use pallet_xcavate_whitelist::{self as whitelist, HasRole};

pub struct EnsurePermission<T>(core::marker::PhantomData<T>);

impl<T: whitelist::Config> EnsureOriginWithArg<T::RuntimeOrigin, whitelist::Role>
    for EnsurePermission<T>
{
    type Success = T::AccountId;

    fn try_origin(
        origin: T::RuntimeOrigin,
        role: &whitelist::Role,
    ) -> Result<Self::Success, T::RuntimeOrigin> {
        let Some(who) = origin.clone().into_signer() else {
            return Err(origin);
        };
        if whitelist::Pallet::<T>::has_role(&who, role.clone()) {
            Ok(who)
        } else {
            Err(origin)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_role: &whitelist::Role) -> Result<T::RuntimeOrigin, ()> {
        let account = frame_benchmarking::whitelisted_caller();
        Ok(frame_system::RawOrigin::Signed(account).into())
    }
}

parameter_types! {
    pub const MinimumStakingAmount: Balance = 1000 * UNIT;
    pub const MaxProperty: u32 = 1000;
    pub const MaxLettingAgent: u32 = 100;
    pub const MaxLocation: u32 = 100;
    pub const LettingAgentVotingDuration: BlockNumber = 20;
}

/// Configure the pallet-property-management in pallets/property-management.
impl pallet_property_management::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_property_management::weights::SubstrateWeight<Runtime>;
    type Balance = Balance;
    type RuntimeHoldReason = RuntimeHoldReason;
    type NativeCurrency = Balances;
    type ForeignCurrency = Assets;
    type MarketplacePalletId = MarketplacePalletId;
    type AgentOrigin = EnsureRoot<Self::AccountId>;
    type LettingAgentDeposit = MinimumStakingAmount;
    type MaxProperties = MaxProperty;
    type MaxLettingAgents = MaxLettingAgent;
    type MaxLocations = MaxLocation;
    type AcceptedAssets = AcceptedPaymentAssets;
    type PropertyToken = RealEstateAsset;
    type LettingAgentVotingTime = LettingAgentVotingDuration;
    type PermissionOrigin = EnsurePermission<Self>;
}

parameter_types! {
    pub const PropertyVotingTime: BlockNumber = 20;
    pub const PropertySaleVotingTime: BlockNumber = 20;
    pub const MaxVoteForBlock: u32 = 100;
    pub const MinimumSlashingAmount: Balance = 10 * UNIT;
    pub const VotingThreshold: Percent = Percent::from_percent(51);
    pub const HighVotingThreshold: Percent = Percent::from_percent(67);
    pub const LowProposal: Balance = 500 * UNIT;
    pub const HighProposal: Balance = 10_000 * UNIT;
    pub const SalesProposalThreshold: Percent = Percent::from_percent(90);
    pub const AuctionDuration: BlockNumber = 28;
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_property_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_property_governance::weights::SubstrateWeight<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type LocalCurrency = RealEstateAssets;
    type ForeignCurrency = Assets;
    type ForeignAssetsHolder = AssetsHolder;
    type VotingTime = PropertyVotingTime;
    type SaleVotingTime = PropertySaleVotingTime;
    type MaxVotesForBlock = MaxVoteForBlock;
    type MinSlashingAmount = MinimumSlashingAmount;
    type Threshold = VotingThreshold;
    type HighThreshold = HighVotingThreshold;
    type LowProposal = LowProposal;
    type HighProposal = HighProposal;
    type MarketplacePalletId = MarketplacePalletId;
    type SaleApprovalYesThreshold = SalesProposalThreshold;
    type AuctionTime = AuctionDuration;
    type Slash = ();
    type AcceptedAssets = AcceptedPaymentAssets;
    type TreasuryId = TreasuryPalletId;
    type PropertyToken = RealEstateAsset;
    type PermissionOrigin = EnsurePermission<Self>;
}

parameter_types! {
    pub const Postcode: u32 = 10;
    pub const LocationDepositAmount: Balance = 10_000 * UNIT;
    pub const MaximumListingDuration: BlockNumber = 30 * DAYS;
    pub const RegionVotingTime: BlockNumber = 20;
    pub const RegionAuctionTime: BlockNumber = 20;
    pub const RegionOperatorVotingTime: BlockNumber = 20;
    pub const RegionThreshold: Percent = Percent::from_percent(75);
    pub const RegionProposalCooldown: BlockNumber = 100;
    pub const MaxProposalForBlock: u32 = 100;
    pub const RegionSlashingAmount: Balance = 10 * UNIT;
    pub const RegionOwnerChangeTime: BlockNumber = 400;
    pub const RegionOwnerNoticeTime: BlockNumber = 50;
    pub const RegionOwnerDisputeDepositAmount: Balance = 1_000 * UNIT;
    pub const MinimumRegionDepositAmount: Balance = 100_000 * UNIT;
    pub const RegionProposalDepositAmount: Balance = 5_000 * UNIT;
    pub const MinimumVotingPower: Balance = 100 * UNIT;
    pub const MaximumRegionVoters: u32 = 250;
    pub const LawyerDepositAmount: Balance = 10_000 * UNIT;
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_regions::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_regions::weights::SubstrateWeight<Runtime>;
    type Balance = Balance;
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
    type RegionOperatorVotingTime = RegionOperatorVotingTime;
    type MaxProposalsForBlock = MaxProposalForBlock;
    type RegionSlashingAmount = RegionSlashingAmount;
    type TreasuryId = TreasuryPalletId;
    type RegionOwnerChangePeriod = RegionOwnerChangeTime;
    type Slash = ();
    type RegionOwnerNoticePeriod = RegionOwnerNoticeTime;
    type RegionOwnerDisputeDeposit = RegionOwnerDisputeDepositAmount;
    type MinimumRegionDeposit = MinimumRegionDepositAmount;
    type RegionProposalDeposit = RegionProposalDepositAmount;
    type MinimumVotingAmount = MinimumVotingPower;
    type MaxRegionVoters = MaximumRegionVoters;
    type PermissionOrigin = EnsurePermission<Self>;
    type LawyerDeposit = LawyerDepositAmount;
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_real_estate_asset::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type Nfts = Nfts;
    type PalletId = MarketplacePalletId;
    type LocalCurrency = RealEstateAssets;
    type FractionalizeCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type FractionalizeItemId = <Self as pallet_nfts::Config>::ItemId;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type PropertyAccountFundingAmount = PropertyFundingAmount;
    type MaxPropertyToken = MaxPropertyTokens;
}
