default:
    echo "Look at the .justfile"

run *args:
    cargo run -- {{args}}

web:
    cargo geng run --platform web

deploy:
    cargo geng build --platform web --release
    butler push target/geng kuviman/scaling-over-it:html5
