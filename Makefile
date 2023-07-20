#

build-linux-x86:
	cargo build --release --target=x86_64-unknown-linux-musl
build-linux-aarch64:
	cargo build --release --target=aarch64-unknown-linux-musl
build-darwin-x86:
	cargo build --release --target=x86_64-apple-darwin
build-darwin-aarch64:
	cargo build --release --target=aarch64-apple-darwin

build-linux: build-linux-x86 build-linux-aarch64
