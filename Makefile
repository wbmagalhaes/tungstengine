FEATURES ?= 2d 3d dialog scene

.PHONY: fmt lint test features wasm check

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

features:
	@for f in $(FEATURES); do \
		echo "== $$f"; \
		cargo clippy -p tungstengine --no-default-features --features $$f -- -D warnings || exit 1; \
	done

wasm:
	cargo check -p tungstengine --no-default-features --features 2d --target wasm32-unknown-unknown

check: fmt lint test features wasm
