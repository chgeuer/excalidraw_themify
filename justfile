# Build in release mode
build:
    cargo build --release

# Install the binary to ~/.cargo/bin
install: build
    cp target/release/excalidraw_themify ~/.cargo/bin/
