#!/bin/sh

# Download binary built by travis inside plugin directory
wget https://github.com/Psykopear/neovim-package-info/releases/download/0.1.0/neovim-package-info -O plugin/neovim-package-info
chmod +x plugin/neovim-package-info

# Remove things we don't want
rm -rf target images examples Cargo.lock Cargo.toml README.md src
