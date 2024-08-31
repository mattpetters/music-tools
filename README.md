# Music Tools

This repo is a collection of scripts I wrote to help install music plugins faster (ie; when getting a new computer). Setting up DAW environments is a pain in the ass and takes forever so this at least automates the install and patching of your plugins.

## Usage

For ease of use probably add the `bin` folder to your shell `$PATH`

# For bash users
cd music-tools
echo 'export PATH="$PWD/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# For zsh users
cd music-tools
echo 'export PATH="$PWD/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc


# Install packages
./install_pkgs /path/to/pkg/directory

# Run patchers
./run_patchers /path/to/command/directory
