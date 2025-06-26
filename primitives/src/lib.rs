#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::traits::VariantCount;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
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
