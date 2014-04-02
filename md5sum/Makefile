include ../common.mk

all: ../build/md5sum

CRYPTO_DIR := rust-crypto
CRYPTO_LIB := $(CRYPTO_DIR)/$(shell $(RUSTC) --crate-file-name --crate-type rlib $(CRYPTO_DIR)/src/rust-crypto/lib.rs)

../build/md5sum: md5sum.rs $(CRYPTO_LIB)
	$(RUSTC) $(RUSTFLAGS) -L $(CRYPTO_DIR) -o $@ $<

$(CRYPTO_LIB): $(CRYPTO_DIR)/src/rust-crypto/*.rs
	cd $(CRYPTO_DIR) && make

clean:
	cd $(CRYPTO_DIR) && make clean
