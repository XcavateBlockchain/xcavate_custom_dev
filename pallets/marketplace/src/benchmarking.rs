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
    assert_ok!(Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), signer.clone()));
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
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(&signer, total_funds));

    assert_ok!(Regions::<T>::propose_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region.clone()
    ));
    assert_ok!(Regions::<T>::vote_on_region_proposal(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        Vote::Yes
    ));
    assert_ok!(Regions::<T>::add_regional_operator(
        RawOrigin::Root.into(),
        signer.clone()
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
    assert!(pallet_regions::LocationRegistration::<T>::contains_key(region_id, &location));

    (region_id, location)
}

fn add_buyers_to_listing<T: Config>(
    buyers: u32,
    payment_asset: u32,
    property_price: <T as pallet::Config>::Balance,
) {
    let deposit_amount = property_price
        .saturating_mul(T::ListingDeposit::get()) / 100u128.into();

    for i in 0..buyers {
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
        assert_ok!(Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), buyer.clone()));
        assert_ok!(Marketplace::<T>::buy_property_token(RawOrigin::Signed(buyer).into(), 0, 1, payment_asset_buyers));
        }
}

#[benchmarks]
mod benchmarks {
    use super::*;
    #[benchmark]
    fn list_object(
        t: Linear<{ <T as pallet::Config>::MinPropertyToken::get() }, {<T as pallet::Config>::MaxPropertyToken::get()}>,
        m: Linear<0, {<T as pallet_nfts::Config>::StringLimit::get()}>,
    ) {
        let signer: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(signer.clone());
        let token_amount: u32 = t;
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount = property_price
            .saturating_mul(T::ListingDeposit::get()) / 100u128.into();
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
        assert_eq!(ListedToken::<T>::get(listing_id).unwrap(), token_amount);
        assert_eq!(ListingDeposits::<T>::get(listing_id).unwrap().0, signer);
        let listing = OngoingObjectListing::<T>::get(listing_id).unwrap();
        assert_eq!(listing.token_price, token_price);
        assert_eq!(listing.tax_paid_by_developer, tax_paid_by_developer);
    }

    #[benchmark]
    fn buy_property_token_single_token(
        a: Linear<1, {<T as pallet::Config>::MaxPropertyToken::get().saturating_sub(1)}>,
        b: Linear<0, {<T as pallet::Config>::MaxPropertyToken::get().saturating_sub(2)}>,
    ) {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount = property_price
            .saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![42u8; <T as pallet_nfts::Config>::StringLimit::get() as usize]);

        let tax_paid_by_developer = true;
        assert_ok!(
            Marketplace::<T>::list_object(
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
        assert_ok!(Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), buyer.clone()));
        let amount: u32 = a.min(token_amount - b - 1);

        #[extrinsic_call]
        buy_property_token(
            RawOrigin::Signed(buyer.clone()),
            listing_id,
            amount,
            payment_asset,
        );

        assert_eq!(ListedToken::<T>::get(listing_id).unwrap(), token_amount - amount - b);
        assert!(TokenBuyer::<T>::get(listing_id).contains(&buyer));
        let token_owner = TokenOwner::<T>::get(&buyer, listing_id);
        assert_eq!(token_owner.token_amount, amount);
        assert!(PropertyLawyer::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn buy_property_token_all_token(
        b: Linear<1, {<T as pallet::Config>::MaxPropertyToken::get()}>,
        n: Linear<1, {<T as pallet::Config>::AcceptedAssets::get().len() as u32}>,
    ) {
        let seller: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone());
        let token_amount: u32 = <T as pallet::Config>::MaxPropertyToken::get();
        let token_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = token_price.saturating_mul((token_amount as u128).into());
        let deposit_amount = property_price
            .saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        ));

        let metadata: BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![42u8; <T as pallet_nfts::Config>::StringLimit::get() as usize]);

        let tax_paid_by_developer = true;
        assert_ok!(
            Marketplace::<T>::list_object(
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
        assert_ok!(Whitelist::<T>::add_to_whitelist(RawOrigin::Root.into(), buyer.clone()));

        #[extrinsic_call]
        buy_property_token(
            RawOrigin::Signed(buyer.clone()),
            listing_id,
            amount,
            payment_asset,
        );

        assert_eq!(ListedToken::<T>::get(listing_id), None);
        assert!(TokenBuyer::<T>::get(listing_id).contains(&buyer));
        let token_owner = TokenOwner::<T>::get(&buyer, listing_id);
        assert_eq!(token_owner.token_amount, amount);
        let property_lawyer = PropertyLawyer::<T>::get(listing_id).unwrap();
        assert_eq!(property_lawyer.real_estate_developer_status, DocumentStatus::Pending);
    }

    /*     #[benchmark]
    fn buy_token() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        #[extrinsic_call]
        buy_token(RawOrigin::Signed(caller), 0, 100);

        assert_eq!(
            NftMarketplace::<T>::registered_nft_details::<
                <T as pallet::Config>::CollectionId,
                <T as pallet::Config>::ItemId,
            >(0.into(), 0.into())
            .unwrap()
            .spv_created,
            true
        );
    }

    #[benchmark]
    fn relist_token() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2_000u32.into();
        #[extrinsic_call]
        relist_token(RawOrigin::Signed(caller), 0, 0.into(), listing_value, 80);
        //assert_eq!(NftMarketplace::<T>::listed_nfts().len(), 1);
    }

    #[benchmark]
    fn buy_relisted_token() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2u32.into();
        assert_ok!(NftMarketplace::<T>::relist_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            0.into(),
            listing_value,
            80,
        ));
        let nft_buyer: T::AccountId = whitelisted_caller();
        <T as pallet_nfts::Config>::Currency::make_free_balance_be(
            &nft_buyer,
            DepositBalanceOf::<T>::max_value(),
        );
        let amount: BalanceOf<T> = 1_000_000u32.into();
        let user_lookup = <T::Lookup as StaticLookup>::unlookup(nft_buyer.clone());
        let asset_id = <T as pallet::Config>::Helper::to_asset(1);
        assert_ok!(Assets::<T, Instance1>::mint(
            RawOrigin::Signed(caller.clone()).into(),
            asset_id.clone().into(),
            user_lookup,
            amount.into(),
        ));
        #[extrinsic_call]
        buy_relisted_token(RawOrigin::Signed(nft_buyer), 1, 1);
        //assert_eq!(NftMarketplace::<T>::listed_nfts().len(), 0);
    }

    #[benchmark]
    fn make_offer() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2u32.into();
        assert_ok!(NftMarketplace::<T>::relist_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            0.into(),
            listing_value,
            80,
        ));
        let token_buyer: T::AccountId = whitelisted_caller();
        <T as pallet_nfts::Config>::Currency::make_free_balance_be(
            &token_buyer,
            DepositBalanceOf::<T>::max_value(),
        );
        let amount: BalanceOf<T> = 1_000_000_000u32.into();
        let user_lookup = <T::Lookup as StaticLookup>::unlookup(token_buyer.clone());
        let asset_id = <T as pallet::Config>::Helper::to_asset(1);
        assert_ok!(Assets::<T, Instance1>::mint(
            RawOrigin::Signed(caller.clone()).into(),
            asset_id.clone().into(),
            user_lookup,
            amount.into(),
        ));
        let offer_value: BalanceOf<T> = 100u32.into();
        #[extrinsic_call]
        make_offer(RawOrigin::Signed(token_buyer), 1, offer_value, 10);
        //assert_eq!(NftMarketplace::<T>::listed_nfts().len(), 0);
    }

    #[benchmark]
    fn handle_offer() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2u32.into();
        assert_ok!(NftMarketplace::<T>::relist_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            0.into(),
            listing_value,
            80,
        ));
        let token_buyer: T::AccountId = whitelisted_caller();
        <T as pallet_nfts::Config>::Currency::make_free_balance_be(
            &token_buyer,
            DepositBalanceOf::<T>::max_value(),
        );
        let amount: BalanceOf<T> = 1_000_000_000u32.into();
        let user_lookup = <T::Lookup as StaticLookup>::unlookup(token_buyer.clone());
        let asset_id = <T as pallet::Config>::Helper::to_asset(1);
        assert_ok!(Assets::<T, Instance1>::mint(
            RawOrigin::Signed(caller.clone()).into(),
            asset_id.clone().into(),
            user_lookup,
            amount.into(),
        ));
        let offer_value: BalanceOf<T> = 10u32.into();
        assert_ok!(NftMarketplace::<T>::make_offer(
            RawOrigin::Signed(token_buyer).into(),
            1,
            offer_value,
            10
        ));
        #[extrinsic_call]
        handle_offer(RawOrigin::Signed(caller), 1, 0, crate::Offer::Accept);
        //assert_eq!(NftMarketplace::<T>::listed_nfts().len(), 0);
    }

    #[benchmark]
    fn cancel_offer() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2u32.into();
        assert_ok!(NftMarketplace::<T>::relist_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            0.into(),
            listing_value,
            80,
        ));
        let token_buyer: T::AccountId = whitelisted_caller();
        <T as pallet_nfts::Config>::Currency::make_free_balance_be(
            &token_buyer,
            DepositBalanceOf::<T>::max_value(),
        );
        let amount: BalanceOf<T> = 1_000_000_000u32.into();
        let user_lookup = <T::Lookup as StaticLookup>::unlookup(token_buyer.clone());
        let asset_id = <T as pallet::Config>::Helper::to_asset(1);
        assert_ok!(Assets::<T, Instance1>::mint(
            RawOrigin::Signed(caller.clone()).into(),
            asset_id.clone().into(),
            user_lookup,
            amount.into(),
        ));
        let offer_value: BalanceOf<T> = 100u32.into();
        assert_ok!(NftMarketplace::<T>::make_offer(
            RawOrigin::Signed(token_buyer.clone()).into(),
            1,
            offer_value,
            10
        ));
        #[extrinsic_call]
        cancel_offer(RawOrigin::Signed(token_buyer), 1, 0);
        //assert_eq!(NftMarketplace::<T>::listed_nfts().len(), 0);
    }

    #[benchmark]
    fn upgrade_listing() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2_000u32.into();
        assert_ok!(NftMarketplace::<T>::relist_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            0.into(),
            listing_value,
            80,
        ));
        let new_price: BalanceOf<T> = 5_000u32.into();
        #[extrinsic_call]
        upgrade_listing(RawOrigin::Signed(caller), 1, new_price);
        /* 		assert_eq!(
            NftMarketplace::<T>::ongoing_nft_details::<
                <T as pallet::Config>::CollectionId,
                <T as pallet::Config>::ItemId,
            >(0.into(), 22.into())
            .unwrap()
            .price,
            new_price
        ); */
    }

    #[benchmark]
    fn upgrade_object() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        let new_price: BalanceOf<T> = 300_000u32.into();
        #[extrinsic_call]
        upgrade_object(RawOrigin::Signed(caller), 0, new_price);
        assert_eq!(
            NftMarketplace::<T>::ongoing_object_listing(0)
                .unwrap()
                .token_price,
            new_price
        );
    }

    #[benchmark]
    fn delist_token() {
        let (caller, value) = setup_object_listing::<T>();
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            caller.clone()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        assert_ok!(NftMarketplace::<T>::list_object(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            location,
            value,
            100,
            vec![0; <T as pallet_nfts::Config>::StringLimit::get() as usize]
                .try_into()
                .unwrap(),
        ));
        assert_ok!(NftMarketplace::<T>::buy_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            100
        ));
        let listing_value: BalanceOf<T> = 2_000u32.into();
        assert_ok!(NftMarketplace::<T>::relist_token(
            RawOrigin::Signed(caller.clone()).into(),
            0,
            0.into(),
            listing_value,
            80,
        ));
        #[extrinsic_call]
        delist_token(RawOrigin::Signed(caller), 1);
        //assert_eq!(NftMarketplace::<T>::listed_nfts().len(), 0);
    }

    #[benchmark]
    fn create_new_location() {
        assert_ok!(NftMarketplace::<T>::create_new_region(
            RawOrigin::Root.into()
        ));
        let location = vec![0; <T as pallet::Config>::PostcodeLimit::get() as usize]
            .try_into()
            .unwrap();
        #[extrinsic_call]
        create_new_location(RawOrigin::Root, 0, location);
    }

    #[benchmark]
    fn create_new_region() {
        #[extrinsic_call]
        create_new_region(RawOrigin::Root);
    } */

    impl_benchmark_test_suite!(
        Marketplace,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
