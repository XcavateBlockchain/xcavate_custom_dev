#!/bin/bash

pallets=(
    pallet_marketplace
)

# Generate weights
for pallet_name in "${pallets[@]}"; do
    ./target/release/parachain-template-node benchmark pallet \
        --pallet $pallet_name \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output ./pallets/marketplace/src/weights.rs
done