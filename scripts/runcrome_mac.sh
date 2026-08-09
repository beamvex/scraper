#!/bin/bash

CHROMIUM=$(bash "$(dirname "$0")/find_chromium.sh") || exit 1

echo "Starting: $CHROMIUM"
"$CHROMIUM" --remote-debugging-port=9222 --no-sandbox &
