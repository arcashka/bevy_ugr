#/bin/sh
cargo build
rm sphere
cp target/debug/examples/sphere .
