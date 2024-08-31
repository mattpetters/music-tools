#!/bin/bash

# Check if exactly one argument (the directory) is provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 /path/to/directory/with/pkg/files"
    exit 1
fi

# Directory containing the .pkg files
DIR="$1"

# Check if the provided argument is a directory
if [ ! -d "$DIR" ]; then
    echo "Error: $DIR is not a valid directory."
    exit 1
fi

# Change to the directory with the .pkg files
cd "$DIR"

# Loop through each .pkg file in the directory and install it
for PKG in *.pkg; do
  # Check if the file exists before attempting to install
  if [ -f "$PKG" ]; then
    echo "Installing $PKG..."
    sudo installer -pkg "$PKG" -target /
  else
    echo "No .pkg files found in $DIR"
    break
  fi
done

echo "All packages installed successfully."
