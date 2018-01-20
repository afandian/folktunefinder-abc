# Run cleanup over a 'so far' ABC file that implements all features currently available.
set -e


RUSTFLAGS="$RUSTFLAGS -Awarnings" cargo build
cat test_resources/butterfly.abc | RUST_BACKTRACE=1 target/debug/abctool typeset > /tmp/typeset.svg

