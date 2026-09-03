PREFIX ?= /usr/local
DESTDIR ?=
BINDIR := $(DESTDIR)$(PREFIX)/bin

.PHONY: build release test install uninstall deb rpm
build:
	cargo build
release:
	cargo build --release
test:
	cargo fmt --check
	cargo test --all-targets
install: release
	install -Dm755 target/release/kally $(BINDIR)/kally
uninstall:
	rm -f $(BINDIR)/kally
deb: release
	@test -n "$(VERSION)" || (echo "use: make deb VERSION=0.1.0"; exit 2)
	rm -rf dist/deb
	mkdir -p dist/deb/DEBIAN dist/deb$(PREFIX)/bin
	install -Dm755 target/release/kally dist/deb$(PREFIX)/bin/kally
	printf 'Package: kally\nVersion: $(VERSION)\nArchitecture: amd64\nMaintainer: Kalcite Engine\nDescription: Git-first package manager for Kalcite\n' > dist/deb/DEBIAN/control
	dpkg-deb --build dist/deb dist/kally_$(VERSION)_amd64.deb
rpm: release
	@echo "Use packaging/rpm/kally.spec with rpmbuild -bb; generated RPMs belong in dist/."
