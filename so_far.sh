# Run cleanup over a 'so far' ABC file that implements all features currently available.

cargo build
cat test_resources/so-far.abc |  target/debug/abctool cleanup

