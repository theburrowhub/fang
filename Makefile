## ── Fang Makefile ─────────────────────────────────────────────────────────────
##
## Usage:
##   make <target>
##
## Targets:

BINARY   := fang
VERSION  := $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
COMMIT   := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
DATE     := $(shell date -u +%Y-%m-%dT%H:%M:%SZ)

CARGO    := cargo
RUSTFMT  := rustfmt
CLIPPY   := cargo clippy

RELEASE_DIR := target/release
DEBUG_DIR   := target/debug

# Cross-compilation targets for release builds
TARGETS_LINUX  := x86_64-unknown-linux-musl aarch64-unknown-linux-musl
TARGETS_MACOS  := x86_64-apple-darwin aarch64-apple-darwin
TARGETS_WIN    := x86_64-pc-windows-gnu

.DEFAULT_GOAL := help

.PHONY: help build release test check fmt lint clean install uninstall \
        build-all dist bench doc open-doc

## ── Development ───────────────────────────────────────────────────────────────

build: ## Build debug binary
	$(CARGO) build

release: ## Build optimised release binary
	$(CARGO) build --release

run: ## Run debug binary in current directory
	$(CARGO) run -- .

run-release: ## Run release binary in current directory
	$(CARGO) run --release -- .

## ── Quality ───────────────────────────────────────────────────────────────────

test: ## Run full test suite
	$(CARGO) test

test-verbose: ## Run tests with output captured (--nocapture)
	$(CARGO) test -- --nocapture

check: ## Fast type-check without codegen
	$(CARGO) check

fmt: ## Format source code
	$(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	$(CARGO) fmt -- --check

lint: ## Run Clippy linter (warnings as errors)
	$(CLIPPY) -- -D warnings

audit: ## Audit dependencies for known vulnerabilities
	$(CARGO) audit

## ── Distribution ──────────────────────────────────────────────────────────────

build-linux: ## Build static Linux binaries (requires cross)
	@for target in $(TARGETS_LINUX); do \
		echo "Building $$target..."; \
		cross build --release --target $$target; \
	done

build-macos: ## Build macOS binaries (run on macOS)
	@for target in $(TARGETS_MACOS); do \
		echo "Building $$target..."; \
		$(CARGO) build --release --target $$target; \
	done

build-windows: ## Build Windows binary (requires mingw or cross)
	@for target in $(TARGETS_WIN); do \
		echo "Building $$target..."; \
		cross build --release --target $$target; \
	done

dist: release ## Package release binary for current platform
	@mkdir -p dist
	@cp $(RELEASE_DIR)/$(BINARY) dist/$(BINARY)
	@echo "Binary ready at dist/$(BINARY)"

## ── Documentation ─────────────────────────────────────────────────────────────

doc: ## Build and open rustdoc documentation
	$(CARGO) doc --no-deps --open

## ── Installation ──────────────────────────────────────────────────────────────

install: release ## Install binary to ~/.cargo/bin
	$(CARGO) install --path .

uninstall: ## Remove installed binary
	$(CARGO) uninstall $(BINARY) 2>/dev/null || true

## ── Housekeeping ──────────────────────────────────────────────────────────────

clean: ## Remove build artefacts
	$(CARGO) clean
	rm -rf dist

## ── Meta ──────────────────────────────────────────────────────────────────────

version: ## Print version info
	@echo "$(BINARY) $(VERSION) ($(COMMIT)) built $(DATE)"

help: ## Print this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} \
		/^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } \
		/^##/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 4) }' $(MAKEFILE_LIST)
