function patch_ozone() {
        find "/Library/Application Support/iZotope" -type f -name "iZOzone12Core" | while read -r binary; do
            sudo xxd -p -c 0 "$binary" - | \
            sed "s/0f8488000000c685/660f1f440000c685/; \
                 s/e8a33cedff83/b80100000083/; \
                 s/c8020034bf83/1f2003d5bf83/; \
                 s/087c83b91f/280080d21f/" | \
            sudo xxd -r -p -c 0 - "$binary"
            sudo codesign -f -s - "$binary" && sudo xattr -c -r "$(dirname "$binary")"
        done
        echo -e "All Done"
    }
    patch_ozone
