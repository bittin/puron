# make check runs rustfmt (cargo fmt --check) and clippy with warnings as
# errors, builds the workspace, runs its tests, and then runs the check
# scripts in SCRIPT_CHECKS in order, stopping at the first failure. Each
# script runs under a time limit: 600 s for the cargo deny checks
# (c_tests_2.sh, c_tests_3.sh, c_applet_app_19.sh) and read-only-run.sh,
# 1800 s for c_engine_15.sh (its release build has no limit of its own,
# its run 30 s), and 120 s for the others. c_applet_panel_15.sh runs with
# no outer limit (t=0 disables timeout): the release build of its
# make install has none, and the script limits its later checks to 120 s
# itself.
SCRIPT_CHECKS := tools/lint-a.sh tools/c_tests_1.sh tools/c_tests_2.sh tools/c_tests_3.sh \
    tools/c_tests_6.sh tools/c_src_registry_7.sh \
    tools/c_engine_15.sh tools/c_i18n_10.sh tools/c_i18n_15.sh \
    tools/c_tests_7.sh tools/c_applet_panel_10.sh tools/c_applet_app_19.sh \
    tools/c_i18n_11.sh tools/c_settings_window_15.sh tools/c_settings_window_17.sh \
    tools/read-only-run.sh tools/c_applet_panel_15.sh \
    tools/c_tests_18.sh tools/c_tests_19.sh tools/c_tests_20.sh tools/c_tests_21.sh \
    tools/c_tests_22.sh

.PHONY: check setup deb check-deb source-archive install install-bin install-icons install-desktop \
    install-appicon install-metainfo

# make install puts the two binaries, Puron's icon files, the desktop
# entry, the app icon and the AppStream metainfo under
# $(DESTDIR)$(PREFIX). It is the one install list of both packages: the
# Debian package runs it with PREFIX=/usr and the Flatpak with
# PREFIX=/app, so a file that either package needs gets a target here
# and neither package copies files of its own. The desktop entry is
# written from its template with Exec taken from PREFIX (DESTDIR is not
# part of it). tools/c_applet_panel_10.sh runs install-icons and
# tools/c_applet_panel_15.sh runs install, each into a temporary root.
PREFIX ?= /usr
ICON_DIR := $(PREFIX)/share/icons/hicolor/scalable/status
APP_DIR := $(PREFIX)/share/applications
APP_ID := io.github.elalonde.Puron
DESKTOP := $(APP_ID).desktop
ICONS := $(wildcard data/icons/hicolor/scalable/status/*.svg)
TARGET_DIR := $(or $(CARGO_TARGET_DIR),target)

install: install-bin install-icons install-desktop install-appicon install-metainfo

install-bin:
	cargo build --release --locked -p puron-applet -p puron-settings
	install -d $(DESTDIR)$(PREFIX)/bin
	install -m 755 $(TARGET_DIR)/release/puron-applet $(TARGET_DIR)/release/puron-settings \
	    $(DESTDIR)$(PREFIX)/bin/

install-icons:
	install -d $(DESTDIR)$(ICON_DIR)
	install -m 644 $(ICONS) $(DESTDIR)$(ICON_DIR)/

install-desktop:
	install -d $(DESTDIR)$(APP_DIR)
	sed 's|@PREFIX@|$(PREFIX)|' data/$(DESKTOP).in > $(DESTDIR)$(APP_DIR)/$(DESKTOP)
	chmod 644 $(DESTDIR)$(APP_DIR)/$(DESKTOP)

# The app icon is the picture that software stores and COSMIC Settings
# show for Puron, in full color and in a symbolic form. Flatpak exports
# only icons whose names start with the app ID, which is why it is not
# one of the panel's icons above.
install-appicon:
	install -d $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps \
	    $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps
	install -m 644 data/icons/hicolor/scalable/apps/$(APP_ID).svg \
	    $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/
	install -m 644 data/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg \
	    $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/

# The metainfo is what software stores read: name, description, links
# and release notes. Its release notes are written from CHANGELOG.md by
# tools/metainfo-releases.py, never by hand.
install-metainfo:
	install -d $(DESTDIR)$(PREFIX)/share/metainfo
	install -m 644 data/$(APP_ID).metainfo.xml $(DESTDIR)$(PREFIX)/share/metainfo/

check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo build --workspace --locked
	cargo test --workspace --locked
	@for s in $(SCRIPT_CHECKS); do \
	    t=120; case $$s in tools/c_tests_[23].sh|tools/c_tests_7.sh|tools/c_applet_app_19.sh|tools/read-only-run.sh) t=600;; tools/c_engine_15.sh) t=1800;; tools/c_applet_panel_15.sh) t=0;; esac; \
	    echo "== $$s"; timeout -k 5 $$t sh "$$s" || exit 1; \
	done


# make setup enables the tracked commit-msg hook (.githooks/) for this
# clone.
setup:
	git config core.hooksPath .githooks

# make deb builds target/deb/puron_<version>_<arch>.deb from a staged
# make install with PREFIX=/usr; make check-deb reads it back.
deb:
	sh tools/make-deb.sh

check-deb:
	sh tools/check-deb.sh

# make source-archive builds target/source/puron-<version>-source.tar.xz;
# tools/check-source-archive.sh builds that archive offline.
source-archive:
	sh tools/make-source-archive.sh
