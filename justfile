# Justfile for tixschema
# Run `just --list` to see all available commands

kotlinx_serialization_version := "1.11.0"
kotlinx_coroutines_version := "1.11.0"
kotlin_libs := env("TIXSCHEMA_KOTLIN_LIBS", home_directory() / ".local/share/tixschema/kotlin-libs")

# Default recipe - runs comprehensive tests
default: test

# Install required tools
install-tools:
    @echo "Installing required tools..."
    cargo install cargo-hack || echo "cargo-hack already installed"
    cargo install just || echo "just already installed"

# Test every combination of the plain features, plus the default set. The
# feature sets are excluded as toggles: each is a name for features already in the powerset.
# Slow and disk-hungry; `test-sets` is what CI runs.
test:
    @echo "Testing all feature combinations..."
    cargo hack test --feature-powerset --exclude-features web,mobile,mongo
    @echo "✅ All feature combinations passed!"

# Test all combinations with verbose output
test-verbose:
    @echo "Testing all feature combinations (verbose)..."
    cargo hack test --feature-powerset --exclude-features web,mobile,mongo --verbose
    @echo "✅ All feature combinations passed!"

# Test the powerset of the feature sets (`web`, `mobile`, `mongo`), plus the default set. Every
# plain feature is reached through its set; the plain-feature powerset stays in `test` for a local
# run before a release.
test-sets:
    @echo "Testing every combination of the feature sets..."
    cargo hack test --feature-powerset --include-features web,mobile,mongo
    @echo "✅ All feature-set combinations passed!"

# Test specific feature combinations manually
test-named-features:
    @echo "Testing key feature combinations..."
    cargo test --no-default-features
    cargo test --no-default-features --features "zod"
    cargo test --no-default-features --features "typescript"
    cargo test --no-default-features --features "typescript,zod"
    cargo test --no-default-features --features "serde,zod"
    cargo test --no-default-features --features "serde,zod,object_id"
    cargo test --all-features
    @echo "✅ Key feature combinations passed!"

# Test with default features
test-default:
    @echo "Testing with default features..."
    cargo test
    @echo "✅ Default features test passed!"

# Test with no features (minimal build)
test-minimal:
    @echo "Testing with no features..."
    cargo test --no-default-features
    @echo "✅ Minimal test passed!"

# Test specific feature combinations individually
test-combinations:
    @echo "Testing individual feature combinations..."
    cargo test --no-default-features --features "serde"
    cargo test --no-default-features --features "zod"
    cargo test --no-default-features --features "jsonschema"
    cargo test --no-default-features --features "object_id"
    cargo test --no-default-features --features "typescript"
    cargo test --no-default-features --features "serde,zod"
    cargo test --no-default-features --features "serde,typescript"
    cargo test --no-default-features --features "zod,typescript"
    cargo test --no-default-features --features "serde,zod,typescript"
    @echo "✅ Individual combinations passed!"

# Quick test - just run default tests
quick:
    @echo "Quick test with default features..."
    cargo test

# Lint Rust (clippy + fmt check). Lint levels are configured in Cargo.toml [lints.clippy].
lint:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

# Lint every feature combination (clippy over the full feature powerset). Fails on any
# warning in any toggle, including feature-gated test code that `lint` (default features) misses.
lint-all:
    @echo "Linting all feature combinations..."
    cargo hack clippy --feature-powerset --exclude-features web,mobile,mongo --all-targets -- -D warnings
    @echo "✅ All feature combinations lint passed!"

# Lint the powerset of the feature sets, the counterpart of `test-sets`; what CI runs.
lint-sets:
    @echo "Linting every combination of the feature sets..."
    cargo hack clippy --feature-powerset --include-features web,mobile,mongo --all-targets -- -D warnings
    @echo "✅ All feature-set combinations lint passed!"

# Type-check the emitted TypeScript bundle with a real compiler, in the build that publishes the
# client and the dispatcher and in the one that publishes neither.
#
# Deliberately outside `all` and `ci`: a fresh clone has no TypeScript compiler, and the type-check
# tests inside `cargo test` stand down when they find none, saying so on stderr. This recipe is the
# one that refuses to stand down — it resolves the compiler up front and names it for the tests,
# where a named compiler that cannot be started is a failure rather than a stand-down. Set
# TIXSCHEMA_TSC to use a compiler that is not on PATH.
typecheck-ts:
    @command -v "${TIXSCHEMA_TSC:-tsc}" >/dev/null 2>&1 || { echo "No TypeScript compiler: put \`tsc\` on PATH, or set TIXSCHEMA_TSC to one." >&2; exit 1; }
    @echo "Type-checking the emitted bundle with $(command -v "${TIXSCHEMA_TSC:-tsc}")..."
    TIXSCHEMA_TSC="$(command -v "${TIXSCHEMA_TSC:-tsc}")" cargo test --test service_schema_typescript_tests type_check
    TIXSCHEMA_TSC="$(command -v "${TIXSCHEMA_TSC:-tsc}")" cargo test --no-default-features --features "serde,typescript" --test service_schema_typescript_tests type_check
    @echo "✅ The emitted bundle type-checks!"

# Install the jars the emitted Kotlin compiles and runs against into ~/.local/share/tixschema/kotlin-libs: the
# serialization compiler plugin from kotlinc's own lib/, and the pinned JVM library jars from
# Maven Central, each checked against its published SHA-256. Rerunning downloads nothing new.
kotlin-libs:
    #!/usr/bin/env bash
    set -euo pipefail
    kotlinc="$(command -v "${TIXSCHEMA_KOTLINC:-kotlinc}")" || { echo "No kotlinc: put \`kotlinc\` on PATH, or set TIXSCHEMA_KOTLINC to one." >&2; exit 1; }
    libs="{{kotlin_libs}}"
    mkdir -p "$libs"
    cp -f "$(dirname "$(readlink -f "$kotlinc")")/../lib/kotlinx-serialization-compiler-plugin.jar" "$libs/"
    fetch() {
        local artifact="$1" version="$2"
        local jar="$artifact-$version.jar"
        local url="https://repo1.maven.org/maven2/org/jetbrains/kotlinx/$artifact/$version/$jar"
        find "$libs" -maxdepth 1 -name "$artifact-*.jar" ! -name "$jar" -delete
        [ -f "$libs/$jar" ] && return
        curl -fsSL -o "$libs/$jar.part" "$url"
        local expected actual
        expected="$(curl -fsSL "$url.sha256" | cut -c1-64)"
        actual="$(sha256sum "$libs/$jar.part" | cut -c1-64)"
        [ "$expected" = "$actual" ] || { rm -f "$libs/$jar.part"; echo "Checksum mismatch for $jar" >&2; exit 1; }
        mv -f "$libs/$jar.part" "$libs/$jar"
    }
    fetch kotlinx-serialization-json-jvm {{kotlinx_serialization_version}}
    fetch kotlinx-serialization-core-jvm {{kotlinx_serialization_version}}
    fetch kotlinx-coroutines-core-jvm {{kotlinx_coroutines_version}}
    echo "Kotlin jars in $libs"

# Run the emitted clients through their own language's runtime, with a real message object.
#
# What a string test cannot reach: `String(sending)` is well-formed TypeScript that renders every
# object as the constant `[object Object]`, so only running the client shows which URL comes out.
# The groups inside `cargo test` stand down when they find no runtime, saying so on stderr. This
# recipe refuses to stand down — it resolves each runtime up front and names it for the tests,
# where a named runtime that cannot be started is a failure. Set TIXSCHEMA_NODE, TIXSCHEMA_DART,
# TIXSCHEMA_SWIFT, TIXSCHEMA_KOTLINC or TIXSCHEMA_JAVA to use one that is not on PATH. The Kotlin
# leg reads TIXSCHEMA_KOTLIN_LIBS, defaulting to ~/.local/share/tixschema/kotlin-libs, which
# `just kotlin-libs` fills.
test-emitted:
    @command -v "${TIXSCHEMA_NODE:-node}" >/dev/null 2>&1 || { echo "No node: put \`node\` on PATH, or set TIXSCHEMA_NODE to one." >&2; exit 1; }
    @echo "Running the emitted TypeScript client with $(command -v "${TIXSCHEMA_NODE:-node}")..."
    TIXSCHEMA_NODE="$(command -v "${TIXSCHEMA_NODE:-node}")" cargo test --test service_schema_emitted_client_tests run_node::
    @test -n "${TIXSCHEMA_NODE_MODULES:-}" && [ -d "${TIXSCHEMA_NODE_MODULES}/node_modules/ws" ] && [ -d "${TIXSCHEMA_NODE_MODULES}/node_modules/zod" ] || { echo "No ws/zod: set TIXSCHEMA_NODE_MODULES to a directory whose node_modules holds ws and zod." >&2; exit 1; }
    @echo "Running the emitted WebSocket server with $(command -v "${TIXSCHEMA_NODE:-node}")..."
    TIXSCHEMA_NODE="$(command -v "${TIXSCHEMA_NODE:-node}")" cargo test --test service_schema_emitted_client_tests run_node_ws_server
    @test -n "${TIXSCHEMA_NODE_MODULES:-}" && [ -d "${TIXSCHEMA_NODE_MODULES}/node_modules/zod" ] || { echo "No zod: set TIXSCHEMA_NODE_MODULES to a directory whose node_modules holds zod." >&2; exit 1; }
    @echo "Running the emitted TypeScript REST server with $(command -v "${TIXSCHEMA_NODE:-node}")..."
    TIXSCHEMA_NODE="$(command -v "${TIXSCHEMA_NODE:-node}")" cargo test --test service_schema_emitted_client_tests run_node_http_service
    @echo "Running the emitted ws_rpc headers against the Rust twins with $(command -v "${TIXSCHEMA_NODE:-node}")..."
    TIXSCHEMA_NODE="$(command -v "${TIXSCHEMA_NODE:-node}")" cargo test --test service_schema_emitted_client_tests run_node_ws_headers
    @command -v "${TIXSCHEMA_DART:-dart}" >/dev/null 2>&1 || { echo "No Dart SDK: put \`dart\` on PATH, or set TIXSCHEMA_DART to one." >&2; exit 1; }
    @echo "Running the emitted Dart client with $(command -v "${TIXSCHEMA_DART:-dart}")..."
    TIXSCHEMA_DART="$(command -v "${TIXSCHEMA_DART:-dart}")" cargo test --all-features --test service_schema_emitted_client_tests run_dart
    @command -v "${TIXSCHEMA_SWIFT:-swift}" >/dev/null 2>&1 || { echo "No Swift toolchain: put \`swift\` on PATH, or set TIXSCHEMA_SWIFT to one." >&2; exit 1; }
    @echo "Running the emitted Swift client with $(command -v "${TIXSCHEMA_SWIFT:-swift}")..."
    TIXSCHEMA_SWIFT="$(command -v "${TIXSCHEMA_SWIFT:-swift}")" cargo test --all-features --test service_schema_emitted_client_tests run_swift
    @command -v "${TIXSCHEMA_KOTLINC:-kotlinc}" >/dev/null 2>&1 || { echo "No kotlinc: put \`kotlinc\` on PATH, or set TIXSCHEMA_KOTLINC to one." >&2; exit 1; }
    @[ -d "{{kotlin_libs}}" ] || { echo "No Kotlin jars at {{kotlin_libs}}: run \`just kotlin-libs\`, or set TIXSCHEMA_KOTLIN_LIBS to a directory holding them." >&2; exit 1; }
    @echo "Running the emitted Kotlin client with $(command -v "${TIXSCHEMA_KOTLINC:-kotlinc}")..."
    TIXSCHEMA_KOTLIN_LIBS="{{kotlin_libs}}" TIXSCHEMA_KOTLINC="$(command -v "${TIXSCHEMA_KOTLINC:-kotlinc}")" cargo test --all-features --test service_schema_emitted_client_tests run_kotlin
    @echo "✅ The emitted clients build the URLs they claim to!"

# Check code without running tests
check:
    @echo "Checking code..."
    cargo check
    just lint
    @echo "✅ Code check passed!"

# Check all feature combinations without running tests
check-all:
    @echo "Checking all feature combinations..."
    cargo hack check --feature-powerset
    @echo "✅ All feature combinations check passed!"

# Format code
fmt:
    @echo "Formatting code..."
    cargo fmt
    @echo "✅ Code formatted!"

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    @echo "✅ Build artifacts cleaned!"

# Full pipeline (standardized `all` entry point across tixena repos): lints and key feature
# combinations, each gate a single build. The exhaustive powerset lives in `all-powerset`.
all: lint lint-all-features test-named-features
    @echo "All checks completed successfully!"

# Exhaustive pipeline - the feature-powerset gates over every plain-feature combination (what `all` ran
# before). Slow; run before a release or after touching feature gates.
all-powerset: lint lint-all test
    @echo "All powerset checks completed successfully!"

# Lint with every feature on at once - one build that reaches the feature-gated code (dart,
# chrono, object_id) the default-features `lint` misses, without the powerset's 128 builds.
lint-all-features:
    cargo clippy --all-targets --all-features -- -D warnings

# Full CI pipeline - what CI would run
ci: clean check-all lint-all test fmt
    @echo "Full CI pipeline completed successfully!"

# Build documentation
docs:
    @echo "Building documentation..."
    cargo doc --no-deps --all-features
    @echo "✅ Documentation built!"

# Open documentation in browser
docs-open: docs
    @echo "Opening documentation..."
    cargo doc --no-deps --all-features --open

# Run specific tests by name
test-name TEST_NAME:
    @echo "Running specific test: {{TEST_NAME}}"
    cargo test {{TEST_NAME}}

# Generate code coverage report (requires cargo-llvm-cov)
test-coverage:
    @echo "Generating code coverage report..."
    cargo llvm-cov --workspace --all-features

# Generate code coverage HTML report (requires cargo-llvm-cov)
test-coverage-html:
    @echo "Generating code coverage HTML report..."
    cargo llvm-cov --workspace --all-features --html
    @echo "Coverage report generated in target/llvm-cov/html/"

# Benchmark tests
bench:
    @echo "Running benchmarks..."
    cargo bench
    @echo "✅ Benchmarks completed!"

# List all available commands
help:
    @just --list

# Run all tests in different modes for comprehensive validation
test-comprehensive: test-minimal test-default test-named-features test
    @echo "Comprehensive testing completed!"
    @echo "✅ All tests passed in all modes!" 