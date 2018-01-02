# Run tests and show STDOUT.
# Include test function argument.
RUST_BACKTRACE=1 cargo test -- --nocapture $1 