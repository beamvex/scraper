#!/bin/bash

export RUST_LOG="scraper=info"
export HOME="/home/robert"
export CARGO_PATH="$HOME/.cargo/bin/cargo"
cd $HOME/dev/scraper/app
$CARGO_PATH run
