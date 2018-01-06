# Run cleanup over a 'so far' ABC file that implements all features currently available.
set -e

cargo build
cat test_resources/so-far-bad.abc | RUST_BACKTRACE=1 target/debug/abctool check

