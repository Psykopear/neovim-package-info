#!/bin/sh

# Build the rust plugin
cargo build --release
# Make it executable
chmod +x target/release/package-info-rs
# Put result where we can find it later
cp -f target/release/package-info-rs ~/.local/bin/
# This should be done by the plugin manager
# cp plugin/package-info-rs.vim ~/.config/nvim/plugin/
