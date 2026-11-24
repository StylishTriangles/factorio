.PHONY: build test serve

build:
	mkdir -p dist
	cp site/index.html site/style.css site/favicon.svg dist/
	wasm-pack build --target web --release --out-dir dist/pkg --out-name planner --no-pack -- --locked

test:
	cargo test --locked

serve: build
	python3 -m http.server 8000 --bind 127.0.0.1 --directory dist
