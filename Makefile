.PHONY: init
init:
	./scripts/init.sh

.PHONY: check
check:
	SKIP_WASM_BUILD=1 cargo check --all

.PHONY: clippy
clippy:
	# Build with target=wasm32 as workaround for substrate issue
	pushd pallets/sp-mvm && \
	cargo clippy -p=sp-mvm --target=wasm32-unknown-unknown --no-default-features

.PHONY: bench
bench:
	pushd node && \
	cargo run --release --features=runtime-benchmarks -- \
		benchmark \
		--dev \
		-lsp_mvm=trace \
		--pallet=sp_mvm \
		--extrinsic='*' \
		--execution=wasm \
		--wasm-execution=compiled \
		--steps=20 --repeat=10 \
		--output=./target/sp-bench/

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --all --no-fail-fast -- --nocapture --test-threads=1

.PHONY: run
run:
	WASM_BUILD_TOOLCHAIN=`cat rust-toolchain` cargo run --release -- --dev --tmp -lsp_mvm=trace

.PHONY: build
build:
	WASM_BUILD_TOOLCHAIN=`cat rust-toolchain` cargo build --release
