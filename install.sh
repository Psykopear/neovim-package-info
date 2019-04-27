#!/bin/zsh

cargo build --release
chmod +x target/release/package-info-rs
cp target/release/package-info-rs /home/docler/.local/bin/
cp plugin/package-info-rs.vim /home/docler/.config/nvim/plugin/
