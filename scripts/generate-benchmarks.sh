#!/bin/bash

pallets=(
    pallet_property_governance
)

# Generate weights
for pallet_name in "${pallets[@]}"; do
    ./target/release/parachain-template-node benchmark pallet \
        --pallet $pallet_name \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output ./pallets/property-governance/src/weights.rs
done