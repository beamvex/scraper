#!/bin/bash

PLAYWRIGHT_CACHE=~/Library/Caches/ms-playwright

LATEST=$(ls -d "$PLAYWRIGHT_CACHE"/chromium-* 2>/dev/null | sort -t- -k2 -n | tail -1)

if [ -z "$LATEST" ]; then
    echo "No Chromium found in $PLAYWRIGHT_CACHE" >&2
    exit 1
fi

CHROMIUM=$(find "$LATEST" -type f \( -name "Chromium" -o -name "Google Chrome for Testing" \) -path "*/MacOS/*" 2>/dev/null | head -1)

if [ -z "$CHROMIUM" ]; then
    echo "Chromium binary not found in $LATEST" >&2
    exit 1
fi

echo "$CHROMIUM"
