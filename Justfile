# JSON parsing benchmark runner

# List targets 
default: 
    @just --list

# Run all three benchmarks
all: go rust goose
    @echo "=== All benchmarks complete ==="

# Go benchmark
go:
    @echo "=== Running Go benchmark ==="
    @go run main.go

# Rust benchmark
rust:
    @echo "=== Running Rust benchmark ==="
    @cargo run --release

# Goose benchmark
goose:
    @echo "=== Running Goose benchmark ==="
    @goose run main.goose

