#!/bin/bash

# Check if exactly one argument (the directory) is provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 /path/to/directory/with/command/files"
    exit 1
fi

# Directory containing the .command files
DIR="$1"

# Check if the provided argument is a directory
if [ ! -d "$DIR" ]; then
    echo "Error: $DIR is not a valid directory."
    exit 1
fi

# Change to the directory with the .command files
cd "$DIR"

# Flag to check if any .command files are found
found=0

# Loop through each .command file in the directory and execute it
for FILE in *.command; do
  # If the glob doesn't get expanded, skip it
  [ -f "$FILE" ] || continue

  # Set the flag indicating we've found at least one .command file
  found=1

  # Execute the .command file
  echo "Executing $FILE..."
  sudo sh "$FILE"
done

# Check if we didn't find any .command files
if [ "$found" -eq 0 ]; then
    echo "No .command files found in $DIR"
else
    echo "All .command files executed successfully."
fi

