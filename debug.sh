#!/usr/bin/bash

#cargo run --bin qwench

#cargo run --bin tmp1

#cargo build --bin qwench

#cargo build --message-format short --bin qwench && alacritty -e target/debug/qwench
#cargo build --message-format short --bin qwench && alacritty --hold -e target/debug/qwench
#cargo build --message-format short --bin qwench && alacritty -e bash -c "./target/debug/qwench; exec bash"
#cargo build --message-format short --bin qwench && alacritty -e bash -c "./target/debug/qwench 48 144; read -n 1 -s -r -p \"Press any key to continue...\""

#RUSTFLAGS="-Awarnings" cargo build --bin qwench && alacritty -e bash -c "RUST_BACKTRACE=1 ./target/debug/qwench; read -n 1 -s -r -p \"Press any key to continue...\""

#RUSTFLAGS="-Awarnings" cargo build --bin qwench && alacritty -e bash -c "RUST_BACKTRACE=full ./target/debug/qwench; read -n 1 -s -r -p \"Press any key to continue...\""

RUSTFLAGS="-Awarnings" cargo build --bin qwench && alacritty -e bash -c "./target/debug/qwench; read -n 1 -s -r -p \"Press any key to continue...\""



