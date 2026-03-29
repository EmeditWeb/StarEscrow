.PHONY: build optimize release test lint fmt clean

WASM_IN=target/wasm32-unknown-unknown/release/escrow.wasm
WASM_OPT=target/wasm32-unknown-unknown/release/escrow.optimized.wasm

build:
	stellar contract build

optimize: build
	stellar contract optimize --wasm $(WASM_IN) --wasm-out $(WASM_OPT)
	@echo "WASM size before optimize: $$(wc -c < $(WASM_IN)) bytes"
	@echo "WASM size after optimize:  $$(wc -c < $(WASM_OPT)) bytes"

release: optimize

test:
	cargo test

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean
