//! Benchmarking setup for pallet-marketplace
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Marketplace;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_support::traits::Get;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use pallet_regions::Pallet as Regions;
use pallet_regions::{RegionIdentifier, Vote};
use pallet_xcavate_whitelist::Pallet as Whitelist;
use scale_info::prelude::vec;

fn create_whitelisted_user<T: Config>() -> T::AccountId {
    let signer: T::AccountId = account("signer", 0, 0);
    assert_ok!(Whitelist::<T>::add_to_whitelist(
        RawOrigin::Root.into(),
        signer.clone()
    ));
    signer
}

fn create_a_new_region<T: Config>(signer: T::AccountId) -> (u16, LocationId<T>) {
    let region = RegionIdentifier::France;
    let region_id = region.clone().into_u16();

    let deposit = T::RegionProposalDeposit::get();
    let auction_amount = T::MinimumRegionDeposit::get();
    let total_funds = deposit
        .saturating_mul(1000u32.into())
        .saturating_add(auction_amount.saturating_mul(100u32.into()));
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(
        &signer,
        total_funds
    ));

    assert_ok!(Regions::<T>::add_regional_operator(
        RawOrigin::Root.into(),
        signer.clone()
    ));
    assert_ok!(Regions::<T>::propose_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region.clone()
    ));
    assert_ok!(Regions::<T>::vote_on_region_proposal(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        Vote::Yes
    ));

    let bid_amount = auction_amount.saturating_mul(10u32.into());

    let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);

    assert_ok!(Regions::<T>::bid_on_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        bid_amount
    ));

    let auction_expiry = frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
    frame_system::Pallet::<T>::set_block_number(auction_expiry);
    assert_ok!(Regions::<T>::create_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        T::MaxListingDuration::get(),
        Permill::from_percent(5)
    ));

    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();
    assert_ok!(Regions::<T>::create_new_location(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        location.clone()
    ));

    // Verify region and location
    assert!(pallet_regions::RegionDetails::<T>::contains_key(region_id));
    assert!(pallet_regions::LocationRegistration::<T>::contains_key(
        region_id, &location
    ));

    (region_id, location)
}

fn list_and_sell_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
) -> T::AccountId {
    let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
    let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
    let property_price = token_price.saturating_mul((token_amount as u128).into());
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
    assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
        &seller,
        deposit_amount.saturating_mul(20u32.into())
    ));

    let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
        BoundedVec::truncate_from(vec![
            42u8;
            <T as pallet_nfts::Config>::StringLimit::get() as usize
        ]);

    let tax_paid_by_developer = true;
    assert_ok!(Marketplace::<T>::list_object(
        RawOrigin::Signed(seller).into(),
        region_id,
        location,
        token_price,
        token_amount,
        metadata,
        tax_paid_by_developer,
    ));
    let listing_id = 0;
    assert!(OngoingObjectListing::<T>::contains_key(listing_id));
    let payment_asset = T::AcceptedAssets::get()[0];
    let buyer: T::AccountId = account("buyer", 0, 0);
    assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
        &buyer,
        deposit_amount.saturating_mul(20u32.into())
    ));
    assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
        payment_asset,
        &buyer,
        property_price.saturating_mul(100u32.into())
    ));
    assert_ok!(Whitelist::<T>::add_to_whitelist(
        RawOrigin::Root.into(),
        buyer.clone()
    ));
    add_buyers_to_listing::<T>(token_amount - 1, payment_asset, property_price);

    assert_ok!(Marketplace::<T>::buy_property_token(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
        1,
        payment_asset,
    ));
    assert_ok!(Marketplace::<T>::claim_property_token(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
    ));
    claim_buyers_property_token::<T>(token_amount - 1, listing_id);
    buyer
}

fn create_registered_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
) -> T::AccountId {
    let token_owner = list_and_sell_property::<T>(seller.clone(), region_id, location);
    let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
    let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        region_id,
        lawyer_1.clone()
    ));
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        region_id,
        lawyer_2.clone()
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        0,
        crate::LegalProperty::RealEstateDeveloperSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::approve_developer_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        0,
        crate::LegalProperty::SpvSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
        RawOrigin::Signed(token_owner.clone()).into(),
        0,
        types::Vote::Yes
    ));
    let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);
    assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
        RawOrigin::Signed(token_owner.clone()).into(),
        0,
    ));

    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_1).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_2).into(),
        0,
        true
    ));
    token_owner
}

fn add_buyers_to_listing<T: Config>(
    buyers: u32,
    payment_asset: u32,
    property_price: <T as pallet::Config>::Balance,
) {
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();

    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        let payment_asset_buyers = T::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));
        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(buyer).into(),
            0,
            1,
            payment_asset_buyers
        ));
    }
}

fn claim_buyers_property_token<T: Config>(
    buyers: u32,
    listing_id: ListingId
) {
    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        assert_ok!(Marketplace::<T>::claim_property_token(
            RawOrigin::Signed(buyer).into(),
            listing_id
        ));
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;
    #[benchmark]
    fn list_object(
        m: Linear<0, { <T as pallet_nfts::Config>::StringLimit::get() }>,
    ) {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(signer.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &signer,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![42u8; m as usize]);

        let tax_paid_by_developer = true;
        assert!(!OngoingObjectListing::<T>::contains_key(0));
        #[extrinsic_call]
        list_object(
            RawOrigin::Signed(signer.clone()),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        );

        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_token_amount, token_amount);
        assert_eq!(ListingDeposits::<T>::get(listing_id).unwrap().0, signer);
        let listing = OngoingObjectListing::<T>::get(listing_id).unwrap();
        assert_eq!(listing.token_price, token_price);
        assert_eq!(listing.tax_paid_by_developer, tax_paid_by_developer);
    }

    #[benchmark]
    fn buy_property_token_single_token(
        a: Linear<1, { <T as pallet::Config>::MaxPropertyToken::get().saturating_sub(1) }>,
        b: Linear<0, { <T as pallet::Config>::MaxPropertyToken::get().saturating_sub(2) }>,
    ) {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        let payment_asset = T::AcceptedAssets::get()[0];
        add_buyers_to_listing::<T>(b, payment_asset, property_price);

        let buyer: T::AccountId = account("buyer_final", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));
        let amount: u32 = a.min(token_amount - b - 1);

        #[extrinsic_call]
        buy_property_token(
            RawOrigin::Signed(buyer.clone()),
            listing_id,
            amount,
            payment_asset,
        );

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_token_amount, token_amount - amount - b);
        let token_owner = TokenOwner::<T>::get(&buyer, listing_id).unwrap();
        assert_eq!(token_owner.token_amount, amount);
        assert!(PropertyLawyer::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn buy_property_token_all_token(
        b: Linear<1, { <T as pallet::Config>::MaxPropertyToken::get() }>,
        n: Linear<1, { <T as pallet::Config>::AcceptedAssets::get().len() as u32 }>,
    ) {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        assert!(OngoingObjectListing::<T>::contains_key(0));
        let payment_asset = T::AcceptedAssets::get()[0];
        let listing_id = 0;
        add_buyers_to_listing::<T>(b - 1, payment_asset, property_price);

        let buyer: T::AccountId = account("buyer_final", 0, 0);
        let amount: u32 = token_amount - (b - 1);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));

        #[extrinsic_call]
        buy_property_token(
            RawOrigin::Signed(buyer.clone()),
            listing_id,
            amount,
            payment_asset,
        );

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_token_amount, 0);
        //assert!(TokenBuyer::<T>::get(listing_id).contains(&buyer));
        let token_owner = TokenOwner::<T>::get(&buyer, listing_id).unwrap();
        assert_eq!(token_owner.token_amount, amount);
        let property_lawyer = PropertyLawyer::<T>::get(listing_id).unwrap();
        assert_eq!(
            property_lawyer.real_estate_developer_status,
            DocumentStatus::Pending
        );
    }
    
    #[benchmark]
    fn claim_property_token() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());

        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));
        add_buyers_to_listing::<T>(token_amount - 1, payment_asset, property_price);

        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            1,
            payment_asset,
        ));
        claim_buyers_property_token::<T>(token_amount - 1, listing_id);

        #[extrinsic_call]
        claim_property_token(
            RawOrigin::Signed(buyer.clone()),
            listing_id
        );

        assert!(TokenOwner::<T>::get(&buyer, listing_id).is_none());
        assert_eq!(
            OngoingObjectListing::<T>::get(listing_id).unwrap()
                .investor_funds
                .get(&buyer)
                .clone()
                .unwrap()
                .paid_funds
                .get(&payment_asset),
            Some(token_price).as_ref()
        );
    }

    #[benchmark]
    fn relist_token() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        #[extrinsic_call]
        relist_token(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            price,
            amount,
        );

        let listing = TokenListings::<T>::get(1).unwrap();
        assert_eq!(listing.seller, token_owner);
        assert_eq!(listing.token_price, price);
        assert_eq!(listing.amount, amount);
        assert_eq!(NextListingId::<T>::get(), 2);
    }

    #[benchmark]
    fn buy_relisted_token() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_token(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let relist_buyer: T::AccountId = account("relist_buyer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &relist_buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &relist_buyer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            relist_buyer.clone()
        ));

        #[extrinsic_call]
        buy_relisted_token(
            RawOrigin::Signed(relist_buyer.clone()),
            1,
            amount,
            payment_asset,
        );

        assert!(!TokenListings::<T>::contains_key(1));
        assert!(pallet_real_estate_asset::PropertyOwner::<T>::get(asset_id).contains(&relist_buyer));
        assert!(!pallet_real_estate_asset::PropertyOwner::<T>::get(asset_id).contains(&token_owner));
        assert_eq!(
            pallet_real_estate_asset::PropertyOwnerToken::<T>::get(asset_id, &relist_buyer),
            amount
        );
        assert_eq!(
            pallet_real_estate_asset::PropertyOwnerToken::<T>::get(asset_id, &token_owner),
            0
        );
    }

    #[benchmark]
    fn cancel_property_purchase() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        let payment_asset = T::AcceptedAssets::get()[0];
        add_buyers_to_listing::<T>(token_amount - 2, payment_asset, property_price);

        let buyer: T::AccountId = account("buyer_final", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));
        let amount = 1;

        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            amount,
            payment_asset,
        ));

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_token_amount, 1);
        //assert!(TokenBuyer::<T>::get(listing_id).contains(&buyer));

        #[extrinsic_call]
        cancel_property_purchase(RawOrigin::Signed(buyer.clone()), 0);

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_token_amount, 2);
        //assert!(!TokenBuyer::<T>::get(listing_id).contains(&buyer));
    }

    #[benchmark]
    fn make_offer() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_token(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let offerer: T::AccountId = account("offerer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &offerer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &offerer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            offerer.clone()
        ));

        let offer_price = price - 1u32.into();

        #[extrinsic_call]
        make_offer(
            RawOrigin::Signed(offerer.clone()),
            1,
            offer_price,
            amount,
            payment_asset,
        );

        assert_eq!(
            OngoingOffers::<T>::get(1, offerer).unwrap().token_price,
            offer_price
        );
    }

    #[benchmark]
    fn handle_offer() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_token(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let offerer: T::AccountId = account("offerer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &offerer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &offerer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            offerer.clone()
        ));

        let offer_price = price - 1u32.into();

        assert_ok!(Marketplace::<T>::make_offer(
            RawOrigin::Signed(offerer.clone()).into(),
            1,
            offer_price,
            amount,
            payment_asset
        ));

        #[extrinsic_call]
        handle_offer(
            RawOrigin::Signed(token_owner),
            1,
            offerer.clone(),
            Offer::Accept,
        );

        assert_eq!(OngoingOffers::<T>::get(1, offerer).is_none(), true);
    }

    #[benchmark]
    fn cancel_offer() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_token(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let offerer: T::AccountId = account("offerer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &offerer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &offerer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            offerer.clone()
        ));

        let offer_price = price - 1u32.into();

        assert_ok!(Marketplace::<T>::make_offer(
            RawOrigin::Signed(offerer.clone()).into(),
            1,
            offer_price,
            amount,
            payment_asset
        ));

        #[extrinsic_call]
        cancel_offer(RawOrigin::Signed(offerer.clone()), 1);

        assert_eq!(OngoingOffers::<T>::get(1, offerer).is_none(), true);
        assert!(TokenListings::<T>::contains_key(1));
    }

    #[benchmark]
    fn withdraw_rejected() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));

        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            token_amount,
            payment_asset,
        ));
        assert_ok!(Marketplace::<T>::claim_property_token(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer_1.clone()
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer_2.clone()
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::approve_developer_lawyer(
            RawOrigin::Signed(seller).into(),
            0,
            true
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(buyer.clone()).into(),
            0,
            types::Vote::Yes,
        ));
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
            RawOrigin::Signed(buyer.clone()).into(),
            0,
        ));

        assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
            RawOrigin::Signed(lawyer_1).into(),
            0,
            false
        ));
        assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
            RawOrigin::Signed(lawyer_2).into(),
            0,
            false
        ));

        assert!(RefundToken::<T>::get(listing_id).is_some());

        #[extrinsic_call]
        withdraw_rejected(RawOrigin::Signed(buyer.clone()), listing_id);

        //assert_eq!(TokenOwner::<T>::get(&buyer, listing_id).token_amount, 0);
        assert!(RefundToken::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn withdraw_expired() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            buyer.clone()
        ));

        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            10,
            payment_asset,
        ));

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::MaxListingDuration::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        withdraw_expired(RawOrigin::Signed(buyer.clone()), listing_id);

        //assert_eq!(TokenOwner::<T>::get(&buyer, listing_id).token_amount, 0);
        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn withdraw_deposit_unsold() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::MaxListingDuration::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        withdraw_deposit_unsold(RawOrigin::Signed(seller.clone()), listing_id);

        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
        //assert_eq!(TokenBuyer::<T>::get(listing_id).len(), 0);
    }

    #[benchmark]
    fn upgrade_listing() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_token(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let new_price = 7_000u32.into();

        #[extrinsic_call]
        upgrade_listing(RawOrigin::Signed(token_owner.clone()), 1, new_price);

        assert_eq!(TokenListings::<T>::get(1).unwrap().token_price, new_price);
    }

    #[benchmark]
    fn upgrade_object() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));

        let new_price = 2_000u32.into();

        #[extrinsic_call]
        upgrade_object(RawOrigin::Signed(seller.clone()), listing_id, new_price);

        assert_eq!(
            OngoingObjectListing::<T>::get(listing_id)
                .unwrap()
                .token_price,
            new_price
        );
    }

    #[benchmark]
    fn delist_token() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_token(
            RawOrigin::Signed(token_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        #[extrinsic_call]
        delist_token(RawOrigin::Signed(token_owner.clone()), 1);

        assert!(!TokenListings::<T>::contains_key(1));
        assert!(pallet_real_estate_asset::PropertyOwner::<T>::get(asset_id).contains(&token_owner));
    }

    #[benchmark]
    fn lawyer_claim_property() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let _ = list_and_sell_property::<T>(seller.clone(), region_id, location.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer.clone()
        ));

        #[extrinsic_call]
        lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        );

        assert_eq!(SpvLawyerProposal::<T>::get(0).unwrap().lawyer, lawyer);
    }

    #[benchmark]
    fn vote_on_spv_lawyer() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_holder = list_and_sell_property::<T>(seller.clone(), region_id, location.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer.clone()
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        ));

        #[extrinsic_call]
        vote_on_spv_lawyer(RawOrigin::Signed(token_holder), 0, types::Vote::Yes);

        assert_eq!(SpvLawyerProposal::<T>::get(0).unwrap().lawyer, lawyer);
        assert_eq!(
            OngoingLawyerVoting::<T>::get(0).unwrap().yes_voting_power,
            1
        );
    }

    #[benchmark]
    fn approve_developer_lawyer() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let _ = list_and_sell_property::<T>(seller.clone(), region_id, location.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer.clone()
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            400_u32.into(),
        ));

        #[extrinsic_call]
        approve_developer_lawyer(RawOrigin::Signed(seller), 0, true);

        assert!(ProposedLawyers::<T>::get(0).is_none());
        assert_eq!(
            PropertyLawyer::<T>::get(0)
                .unwrap()
                .real_estate_developer_lawyer,
            Some(lawyer.clone())
        );
    }

    #[benchmark]
    fn finalize_spv_lawyer() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_holder = list_and_sell_property::<T>(seller.clone(), region_id, location.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer.clone()
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        ));

        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(token_holder.clone()).into(),
            0,
            types::Vote::Yes
        ));
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        finalize_spv_lawyer(RawOrigin::Signed(token_holder), 0);

        assert!(SpvLawyerProposal::<T>::get(0).is_none());
        assert!(OngoingLawyerVoting::<T>::get(0).is_none());
        assert_eq!(
            PropertyLawyer::<T>::get(0).unwrap().spv_lawyer,
            Some(lawyer.clone())
        );
    }

    #[benchmark]
    fn remove_from_case() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_holder = list_and_sell_property::<T>(seller.clone(), region_id, location.clone());

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer_1.clone()
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer_2.clone()
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::approve_developer_lawyer(
            RawOrigin::Signed(seller).into(),
            0,
            true
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(token_holder.clone()).into(),
            0,
            types::Vote::Yes,
        ));
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
            RawOrigin::Signed(token_holder).into(),
            0,
        ));

        #[extrinsic_call]
        remove_from_case(RawOrigin::Signed(lawyer_2.clone()), 0);

        assert_eq!(PropertyLawyer::<T>::get(0).unwrap().spv_lawyer, None);
    }

    #[benchmark]
    fn lawyer_confirm_documents(
        a: Linear<1, { <T as pallet::Config>::MaxPropertyToken::get() }>,
    ) {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();

        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet_nfts::Config>::StringLimit::get() as usize
            ]);

        let tax_paid_by_developer = true;
        assert_ok!(Marketplace::<T>::list_object(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            location,
            token_price,
            token_amount,
            metadata,
            tax_paid_by_developer,
        ));
        let listing_id = 0;
        assert!(OngoingObjectListing::<T>::contains_key(listing_id));
        let payment_asset = T::AcceptedAssets::get()[0];
        let token_holder: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &token_holder,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &token_holder,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            token_holder.clone()
        ));
        add_buyers_to_listing::<T>(a - 1, payment_asset, property_price);

        assert_ok!(Marketplace::<T>::buy_property_token(
            RawOrigin::Signed(token_holder.clone()).into(),
            listing_id,
            token_amount - a + 1,
            payment_asset,
        ));
        assert_ok!(Marketplace::<T>::claim_property_token(
            RawOrigin::Signed(token_holder.clone()).into(),
            listing_id,
        ));
        claim_buyers_property_token::<T>(a - 1, listing_id);

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        let listing_id = 0;

        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer_1.clone()
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(seller.clone()).into(),
            region_id,
            lawyer_2.clone()
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            listing_id,
            crate::LegalProperty::RealEstateDeveloperSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::approve_developer_lawyer(
            RawOrigin::Signed(seller).into(),
            0,
            true
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            listing_id,
            crate::LegalProperty::SpvSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(token_holder.clone()).into(),
            0,
            types::Vote::Yes,
        ));
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
            RawOrigin::Signed(token_holder).into(),
            0,
        ));

        assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
            RawOrigin::Signed(lawyer_1).into(),
            listing_id,
            true
        ));

        #[extrinsic_call]
        lawyer_confirm_documents(RawOrigin::Signed(lawyer_2.clone()), listing_id, true);

        assert!(PropertyLawyer::<T>::get(listing_id).is_none());
        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
    }
 
    #[benchmark]
    fn send_property_token() {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_owner = create_registered_property::<T>(seller.clone(), region_id, location);

        let new_owner: T::AccountId = account("new_owner", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            new_owner.clone()
        ));
        let deposit_amount = T::ListingDeposit::get();

        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &new_owner,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let asset_id = 0;
        let token_amount = 1;

        #[extrinsic_call]
        send_property_token(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            new_owner.clone(),
            token_amount,
        );

        assert_eq!(
            pallet_real_estate_asset::PropertyOwnerToken::<T>::get(asset_id, &seller),
            0
        );
        assert_eq!(
            pallet_real_estate_asset::PropertyOwnerToken::<T>::get(asset_id, &new_owner.clone()),
            token_amount
        );
        assert!(pallet_real_estate_asset::PropertyOwner::<T>::get(asset_id).contains(&new_owner));
        assert!(!pallet_real_estate_asset::PropertyOwner::<T>::get(asset_id).contains(&seller));
    }

    impl_benchmark_test_suite!(Marketplace, crate::mock::new_test_ext(), crate::mock::Test);
}
