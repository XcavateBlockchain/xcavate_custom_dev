#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use frame_support::traits::VariantCount;

#[derive(
    Encode, Decode, DecodeWithMemTracking, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, MaxEncodedLen, TypeInfo, RuntimeDebug,
)]
pub enum MarketplaceHoldReason {
    Marketplace,
    Listing,
    Auction,
}

impl VariantCount for MarketplaceHoldReason {
	// Intentionally set below the actual count of variants, to allow testing for `can_freeze`
	const VARIANT_COUNT: u32 = 2;
}