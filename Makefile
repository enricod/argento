# Argento build/test/run/install helper.
#
# Common usage:
#   make build
#   make run
#   make check
#   make flatpak                     # build and install the Flatpak for this user

CARGO ?= cargo

APP_ID := io.github.enricod.Argento
FLATPAK_MANIFEST := build-aux/flatpak/$(APP_ID).yml

.PHONY: help build run test check clean flatpak flatpak-bundle flatpak-sources

help:
	@echo "help            - show this list of targets"
	@echo "build           - cargo build --release"
	@echo "run             - build, then run ./target/release/argento"
	@echo "test            - cargo test"
	@echo "check           - cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test"
	@echo "clean           - cargo clean"
	@echo "flatpak         - build and install the Flatpak for this user (needs flatpak-builder)"
	@echo "flatpak-bundle  - build argento.flatpak, a single installable file"
	@echo "flatpak-sources - regenerate build-aux/flatpak/cargo-sources.json after Cargo.lock changes"

build:
	$(CARGO) build --release

run: build
	./target/release/argento

test:
	$(CARGO) test

check:
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) test

clean:
	$(CARGO) clean

flatpak-sources:
	python3 build-aux/flatpak/gen-cargo-sources.py Cargo.lock build-aux/flatpak/cargo-sources.json

flatpak:
	flatpak-builder --user --install --install-deps-from=flathub --force-clean \
		build-flatpak $(FLATPAK_MANIFEST)

flatpak-bundle:
	flatpak-builder --user --install-deps-from=flathub --force-clean \
		--repo=repo-flatpak build-flatpak $(FLATPAK_MANIFEST)
	flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
		repo-flatpak argento.flatpak $(APP_ID)
