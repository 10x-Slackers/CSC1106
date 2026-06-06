dev:
    @just watch-rust &
    @just watch-frontend

watch-rust:
    find src migration templates Cargo.toml Cargo.lock \( -name '*.rs' -o -name '*.toml' -o -name '*.html' \) | entr -r cargo run

watch-frontend:
    find templates \( -name '*.html' -o -name '*.css' -o -name '*.js' \) | entr -r npm run build
