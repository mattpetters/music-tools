function hex() {
echo ''$1'' | perl -0777pe 's|([0-9a-zA-Z]{2}+(?![^\(]*\)))|\\x${1}|gs'
}

function prep() {
sudo xattr -cr "$1"
sudo xattr -r -d com.apple.quarantine "$1"
sudo codesign --force --sign - "$1"
}

prep "$1"
