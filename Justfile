dev:
    @just watch-rust &
    @just watch-frontend

watch-rust:
    find src migration templates Cargo.toml Cargo.lock \( -name '*.rs' -o -name '*.toml' -o -name '*.html' \) | SECRET_KEY=abcdefghijklmnopqrstuvwxyz1234567890 entr -r cargo run

watch-frontend:
    find templates \( -name '*.html' -o -name '*.css' \) | entr -n npm run build

build:
    npm run build
    cargo build --release

format:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged
    npm run format

lint:
    cargo clippy -- -D warnings
