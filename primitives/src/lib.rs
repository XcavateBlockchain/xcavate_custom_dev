#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use frame_support::traits::VariantCount;

#[derive(
    Encode, Decode, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, MaxEncodedLen, TypeInfo, RuntimeDebug,
)]
pub enum TestId {
    Marketplace,
}

impl VariantCount for TestId {
    const VARIANT_COUNT: u32 = 1; // Update this to match the actual number of variants
}