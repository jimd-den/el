budget:
	python3 tools/check_line_budget.py
test:
	cargo test -q -- --test-threads=1
test-slow:
	cargo test -q -- --test-threads=1 --ignored
lint:
	cargo clippy --all-targets -- -D warnings
typecheck:
	cargo check --all-targets
dogfood:
	cargo run -q -- check --root .
check: budget test lint typecheck dogfood
