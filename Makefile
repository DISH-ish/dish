I486_TOOLCHAIN_URL := https://musl.cc/i486-linux-musl-cross.tgz
I486_TOOLCHAIN_DIR := $(CURDIR)/i486-linux-musl-cross
I486_GCC           := $(I486_TOOLCHAIN_DIR)/bin/i486-linux-musl-gcc
I486_LIB           := $(I486_TOOLCHAIN_DIR)/i486-linux-musl/lib
I486_GCC_LIB       := $(I486_TOOLCHAIN_DIR)/lib/gcc/i486-linux-musl/11.2.1

I486_RUSTFLAGS := -Z unstable-options \
	-C linker=$(I486_GCC) \
	-C panic=immediate-abort \
	-L $(I486_LIB) \
	-L $(I486_GCC_LIB)

.PHONY: all build-i486 build-x86_64 clean

all: build-i486 build-x86_64

$(I486_GCC):
	curl -L --fail "$(I486_TOOLCHAIN_URL)" -o /tmp/i486-linux-musl-cross.tgz
	tar xf /tmp/i486-linux-musl-cross.tgz -C "$(CURDIR)"
	ar rcs "$(I486_LIB)/libunwind.a"
	rm /tmp/i486-linux-musl-cross.tgz

build-i486: $(I486_GCC)
	rustup toolchain install nightly
	rustup component add rust-src --toolchain nightly
	RUST_TARGET_PATH=$(CURDIR) RUSTFLAGS="$(I486_RUSTFLAGS)" \
	cargo +nightly build \
		-Z build-std=std \
		-Z unstable-options \
		--target i486-unknown-linux-musl \
		--release

build-x86_64:
	rustup target add x86_64-unknown-linux-musl
	cargo build --target x86_64-unknown-linux-musl --release

clean:
	cargo clean
