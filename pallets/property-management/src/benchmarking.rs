//! Benchmarking setup for pallet-property-management
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as PropertyManagement;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
/* use pallet_marketplace::Pallet as Marketplace; */
use frame_support::sp_runtime::{Permill, Saturating};
use frame_support::traits::fungible::Mutate;
use frame_support::BoundedVec;
use frame_support::{assert_ok, traits::Get};
use pallet_marketplace::types::LegalProperty;
use pallet_marketplace::Pallet as Marketplace;
use pallet_regions::Pallet as Regions;
use pallet_regions::{RegionIdentifier, Vote};
use pallet_xcavate_whitelist::Pallet as Whitelist;
use scale_info::prelude::{format, vec, vec::Vec};

pub trait Config: pallet_marketplace::Config + crate::Config {}

impl<T: crate::Config + pallet_marketplace::Config> Config for T {}

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
    let token_amount: u32 = <T as pallet_marketplace::Config>::MaxPropertyToken::get();
    let token_price: <T as pallet_marketplace::Config>::Balance = 1_000u32.into();
    let property_price = token_price.saturating_mul((token_amount as u128).into());
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
    assert_ok!(
        <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
            &seller,
            deposit_amount.saturating_mul(20u32.into())
        )
    );

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
    let payment_asset = <T as pallet_marketplace::Config>::AcceptedAssets::get()[0];
    let buyer: T::AccountId = account("buyer", 0, 0);
    assert_ok!(
        <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        )
    );
    assert_ok!(
        <T as pallet_marketplace::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        )
    );
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
        LegalProperty::RealEstateDeveloperSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        0,
        LegalProperty::SpvSide,
        400_u32.into()
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

fn add_buyers_to_listing<T: Config + pallet_marketplace::Config>(
    buyers: u32,
    payment_asset: u32,
    property_price: <T as pallet_marketplace::Config>::Balance,
) {
    let deposit_amount = property_price
        .saturating_mul(<T as pallet_marketplace::Config>::ListingDeposit::get())
        / 100u128.into();

    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        let payment_asset_buyers = <T as pallet_marketplace::Config>::AcceptedAssets::get()[0];
        assert_ok!(
            <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
                &buyer,
                deposit_amount.saturating_mul(20u32.into())
            )
        );
        assert_ok!(
            <T as pallet_marketplace::Config>::ForeignCurrency::mint_into(
                payment_asset,
                &buyer,
                property_price
            )
        );
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

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn add_letting_agent() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            letting_agent.clone()
        ));

        #[extrinsic_call]
        add_letting_agent(
            RawOrigin::Signed(region_owner),
            region_id,
            location,
            letting_agent.clone(),
        );

        assert!(LettingInfo::<T>::contains_key(letting_agent));
    }

    #[benchmark]
    fn letting_agent_deposit() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            letting_agent.clone()
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &letting_agent,
            deposit
        ));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(region_owner).into(),
            region_id,
            location,
            letting_agent.clone()
        ));

        #[extrinsic_call]
        letting_agent_deposit(RawOrigin::Signed(letting_agent.clone()));

        assert_eq!(
            LettingInfo::<T>::get(letting_agent).unwrap().deposited,
            true
        );
    }

    #[benchmark]
    fn add_letting_agent_to_location() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            letting_agent.clone()
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &letting_agent,
            deposit
        ));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(region_owner.clone()).into(),
            region_id,
            location,
            letting_agent.clone()
        ));
        assert_ok!(PropertyManagement::<T>::letting_agent_deposit(
            RawOrigin::Signed(letting_agent.clone()).into()
        ));

        let max_locations = T::MaxLocations::get();

        for x in 2..max_locations {
            let location_str = format!("loc{}", x); // Dynamically include x in the string
            let new_location = BoundedVec::try_from(location_str.as_bytes().to_vec()).unwrap();
            assert_ok!(Regions::<T>::create_new_location(
                RawOrigin::Signed(region_owner.clone()).into(),
                region_id,
                new_location.clone()
            ));
            assert_ok!(PropertyManagement::<T>::add_letting_agent_to_location(
                RawOrigin::Signed(region_owner.clone()).into(),
                new_location,
                letting_agent.clone()
            ));
        }

        let new_location = BoundedVec::try_from("location".as_bytes().to_vec()).unwrap();

        assert_ok!(Regions::<T>::create_new_location(
            RawOrigin::Signed(region_owner.clone()).into(),
            region_id,
            new_location.clone()
        ));

        #[extrinsic_call]
        add_letting_agent_to_location(
            RawOrigin::Signed(region_owner.clone()),
            new_location,
            letting_agent.clone(),
        );

        assert_eq!(
            LettingInfo::<T>::get(letting_agent)
                .unwrap()
                .locations
                .len(),
            T::MaxLocations::get() as usize
        );
    }

    #[benchmark]
    fn set_letting_agent() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let _ = create_registered_property::<T>(region_owner.clone(), region_id, location.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            letting_agent.clone()
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &letting_agent,
            deposit
        ));

        let max_props = T::MaxProperties::get();
        LettingInfo::<T>::insert(
            &letting_agent,
            LettingAgentInfo {
                assigned_properties: BoundedVec::try_from(
                    (1..max_props).map(|i| i).collect::<Vec<_>>(),
                )
                .unwrap(),
                account: letting_agent.clone(),
                region: region_id,
                locations: Default::default(),
                deposited: true,
                active_strikes: Default::default(),
            },
        );

        let asset_id = 0;

        #[extrinsic_call]
        set_letting_agent(RawOrigin::Signed(letting_agent.clone()), 0);

        assert_eq!(
            LettingStorage::<T>::get(asset_id),
            Some(letting_agent.clone())
        );
        assert!(LettingInfo::<T>::get(&letting_agent)
            .unwrap()
            .assigned_properties
            .contains(&asset_id));
    }

    #[benchmark]
    fn distribute_income() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            letting_agent.clone()
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &letting_agent,
            deposit
        ));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(region_owner).into(),
            region_id,
            location,
            letting_agent.clone()
        ));
        assert_ok!(PropertyManagement::<T>::letting_agent_deposit(
            RawOrigin::Signed(letting_agent.clone()).into()
        ));
        assert_ok!(PropertyManagement::<T>::set_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            0
        ));

        let asset_id = 0;

        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &letting_agent,
            200_000_000_000u128.into()
        ));

        let distribution_amount = 100_000u128.into();

        #[extrinsic_call]
        distribute_income(
            RawOrigin::Signed(letting_agent.clone()),
            asset_id,
            distribution_amount,
            payment_asset,
        );

        assert_eq!(
            InvestorFunds::<T>::get((&token_owner, asset_id, payment_asset)),
            distribution_amount / <T as pallet_marketplace::Config>::MaxPropertyToken::get().into()
        );
    }

    #[benchmark]
    fn withdraw_funds() {
        let region_owner: T::AccountId = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone());
        let token_owner =
            create_registered_property::<T>(region_owner.clone(), region_id, location.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::add_to_whitelist(
            RawOrigin::Root.into(),
            letting_agent.clone()
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &letting_agent,
            deposit
        ));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(region_owner).into(),
            region_id,
            location,
            letting_agent.clone()
        ));
        assert_ok!(PropertyManagement::<T>::letting_agent_deposit(
            RawOrigin::Signed(letting_agent.clone()).into()
        ));
        assert_ok!(PropertyManagement::<T>::set_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            0
        ));

        let asset_id = 0;

        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &letting_agent,
            200_000_000_000u128.into()
        ));

        let distribution_amount = 100_000u128.into();

        assert_ok!(PropertyManagement::<T>::distribute_income(
            RawOrigin::Signed(letting_agent.clone()).into(),
            asset_id,
            distribution_amount,
            payment_asset
        ));
        assert_eq!(
            InvestorFunds::<T>::get((&token_owner, asset_id, payment_asset)),
            distribution_amount / <T as pallet_marketplace::Config>::MaxPropertyToken::get().into()
        );

        #[extrinsic_call]
        withdraw_funds(
            RawOrigin::Signed(token_owner.clone()),
            asset_id,
            payment_asset,
        );

        assert_eq!(
            InvestorFunds::<T>::get((&token_owner, asset_id, payment_asset)),
            0u32.into()
        );
    }

    impl_benchmark_test_suite!(
        PropertyManagement,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
