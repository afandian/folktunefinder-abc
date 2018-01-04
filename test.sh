# Run tests and show STDOUT.
# Include test function argument.
RUST_BACKTRACE=full cargo test -- --nocapture $1 