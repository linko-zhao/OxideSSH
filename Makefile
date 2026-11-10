# OxideSSH Build Rules
# Usage: make [target]
#
# Targets:
#   build        Build desktop binary (debug)
#   run          Build and run desktop preview on the current host
#   package-win  Package Windows installer (.msi) — requires Windows host or CI
#   package-mac  Package macOS disk image (.app + .dmg) — requires macOS host or CI
#   test         Run all tests
#   lint         Check formatting and clippy
#   audit        Security audit (requires cargo-audit)
#   check-all    Run lint, test, and audit
#   clean        Remove build artifacts
#   help         Show this help

CARGO      := cargo
PACKAGER   := $(CARGO) packager
TARGET_WIN = x86_64-pc-windows-msvc
TARGET_MAC = aarch64-apple-darwin
TARGET_IOS = aarch64-apple-ios

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------
.PHONY: help build run release test lint audit check-all package-win package-mac check-ios clean
help:
	@echo "OxideSSH Build Targets"
	@echo ""
	@echo "  build        Build desktop binary (debug)"
	@echo "  run          Build and run desktop preview on the current host"
	@echo "  release      Build desktop binary (release)"
	@echo "  package-win  Package Windows installer (.msi) [requires Windows host]"
	@echo "  package-mac  Package macOS disk image (.app + .dmg) [requires macOS host]"
	@echo "  test         Run all tests"
	@echo "  lint         Check formatting and clippy"
	@echo "  audit        Security audit"
	@echo "  check-all    Run lint, test, and audit"
	@echo "  clean        Remove build artifacts"

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
build:
	$(CARGO) build -p oxide-ssh-desktop --locked

ifeq ($(OS),Windows_NT)
run:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/build-local.ps1 -Run
else
run:
	bash scripts/run-local.sh
endif

release:
	$(CARGO) build -p oxide-ssh-desktop --release --locked

# ---------------------------------------------------------------------------
# Test & Lint
# ---------------------------------------------------------------------------
test:
	$(CARGO) test --workspace --all-targets --locked

lint:
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

audit:
	$(CARGO) audit

check-all: lint test audit

# ---------------------------------------------------------------------------
# Packaging
# ---------------------------------------------------------------------------

package-win:
	$(CARGO) build -p oxide-ssh-desktop --release --target $(TARGET_WIN) --locked
	$(PACKAGER) --release --target $(TARGET_WIN) --formats wix

package-mac:
	sh packaging/package-macos.sh


# ---------------------------------------------------------------------------
# iOS check (no packaging, compile-check only)
# ---------------------------------------------------------------------------
check-ios:
	rustup target add $(TARGET_IOS) --toolchain 1.97.1
	$(CARGO) check -p oxide-ssh-core -p oxide-ssh-terminal --target $(TARGET_IOS) --locked

# ---------------------------------------------------------------------------
# Clean
# ---------------------------------------------------------------------------
clean:
	$(CARGO) clean -p oxide-ssh-desktop
	rm -rf target/package
