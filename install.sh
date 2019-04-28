#!/bin/sh

cargo build --release
chmod +x target/release/package-info-rs
cp target/release/package-info-rs ~/.local/bin/
cp plugin/package-info-rs.vim ~/.config/nvim/plugin/
