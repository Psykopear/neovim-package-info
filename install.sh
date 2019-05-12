#!/bin/sh

# Build the rust plugin
cargo build --release
# Make it executable
chmod +x target/release/package-info-rs
# Put exec where we can find it later
mkdir -p ~/.local/bin/
cp -f target/release/package-info-rs ~/.local/bin/

# Remove things we don't want
rm -rf target images examples Cargo.lock Cargo.toml README.md src
