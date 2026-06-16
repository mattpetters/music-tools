#!/bin/sh
# SUDO_ASKPASS helper for mtui.
#
# Reads the prompt from $1 and pops the macOS native password dialog.
# Echoes the entered password on stdout; exits 0 on submit, 0 with empty
# output on cancel. Sudo treats an empty password as auth failure.
exec /usr/bin/osascript - "$1" <<'APPLESCRIPT'
on run argv
    set thePrompt to item 1 of argv
    try
        set theResult to text returned of (display dialog thePrompt ¬
            with title "mtui needs permission" ¬
            default answer "" ¬
            with hidden answer ¬
            with icon caution)
        return theResult
    on error number -128
        -- User clicked Cancel
        return ""
    end try
end run
APPLESCRIPT
