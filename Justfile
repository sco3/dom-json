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

# Goose benchmark (interpreted)
goose:
    @echo "=== Running Goose benchmark ==="
    @goose run main.goose

# Goose benchmarks as optimized native binaries (use these for README numbers)
goose-native:
    @echo "=== Building Goose native binaries ==="
    @goose build main.goose
    @goose build streaming.goose
    @echo "=== Running Goose DOM benchmark (native) ==="
    @./main
    @echo "=== Running Goose streaming benchmark (native) ==="
    @./streaming

# Go investigation: run 1,000,000 iterations comparison across all routines
compare-go:
    @go test -v -run TestCompareAll1M

# Go investigation: micro-benchmarks with memory allocations
bench-go:
    @echo "=== Running Go Benchmarks ==="
    @go test -bench=. -benchmem -count=3 -run=^$


