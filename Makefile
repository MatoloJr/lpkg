PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share/lpkg
CARGO ?= cargo
TARGET = target/release/lpkg

.PHONY: all build release install uninstall deb clean test help

all: build

help:
	@echo "Targets:"
	@echo "  build     Debug build"
	@echo "  release   Release build"
	@echo "  install   Install binary and data to PREFIX ($(PREFIX))"
	@echo "  uninstall Remove installed files from PREFIX"
	@echo "  deb       Build a .deb package"
	@echo "  test      Run tests"
	@echo "  clean     Remove build artifacts"

build:
	$(CARGO) build

release:
	$(CARGO) build --release

test:
	$(CARGO) test

install: release
	install -d $(DESTDIR)$(BINDIR)
	install -d $(DESTDIR)$(DATADIR)
	install -m 755 $(TARGET) $(DESTDIR)$(BINDIR)/lpkg
	install -m 644 data/aliases.toml $(DESTDIR)$(DATADIR)/aliases.toml
	install -m 644 data/direct-deb-index.json $(DESTDIR)$(DATADIR)/direct-deb-index.json
	@echo "Installed lpkg to $(DESTDIR)$(BINDIR)/lpkg"
	@echo "Data files in $(DESTDIR)$(DATADIR)"

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/lpkg
	rm -rf $(DESTDIR)$(DATADIR)
	@echo "Uninstalled lpkg from $(DESTDIR)$(PREFIX)"

deb: release
	./scripts/build-deb.sh

clean:
	$(CARGO) clean
	rm -rf dist
