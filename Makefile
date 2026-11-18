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
    tools/c_tests_22.sh tools/c_tests_23.sh tools/c_tests_24.sh tools/c_tests_25.sh \
    tools/c_tests_26.sh tools/c_tests_27.sh tools/c_tests_28.sh tools/c_tests_29.sh \
    tools/c_tests_30.sh

.PHONY: check setup deb check-deb source-archive install install-bin install-icons install-desktop \
    install-appicon install-metainfo install-licenses install-license-texts install-notices

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

# make install-licenses puts the Flatpak's license files in
# share/licenses/<app ID>, where Flatpak keeps them: Puron's LICENSE, the
# GPL-3.0 text, the source offer and the third-party notices. The
# lm-sensors license is written there by the Flatpak's libsensors module.
# It is not part of install, because the Debian package keeps these files
# in /usr/share/doc/puron, written by tools/make-deb.sh. Its first part
# needs no build and no vendored crates, so a check can run it alone.
#
# The source offer is the Debian package's, followed by a paragraph on the
# lm-sensors library that the Flatpak bundles. The lm-sensors tag and
# commit come from the Flatpak manifest through tools/lm-sensors-ref.sh,
# so that they are written in one file only. When the Flatpak is built on
# the COSMIC base app, the notice on that app's icon themes joins them.
#
# install-notices needs VENDOR_DIR, the folder of the vendored crates; it
# runs the same notices generator as the Debian package.
LICENSE_DIR := $(PREFIX)/share/licenses/$(APP_ID)
ICON_THEMES_NOTICE := $(wildcard flatpak/icon-themes-notice.txt)

install-licenses: install-license-texts install-notices

install-license-texts:
	install -d $(DESTDIR)$(LICENSE_DIR)
	install -m 644 LICENSE data/licenses/GPL-3.0.txt $(DESTDIR)$(LICENSE_DIR)/
	ref=$$(sh tools/lm-sensors-ref.sh) || exit 1; \
	version=$$(sed -n 's/^version = "\(.*\)"$$/\1/p' Cargo.toml | head -n 1); \
	[ -n "$$version" ] || { echo "install-license-texts: no version in Cargo.toml"; exit 1; }; \
	{ sed "s|@VERSION@|$$version|g" debian/source-offer.in && echo && \
	    sed -e "s|@LMS_TAG@|$${ref% *}|g" -e "s|@LMS_COMMIT@|$${ref#* }|g" \
	        flatpak/source-offer-libsensors.in; } > $(DESTDIR)$(LICENSE_DIR)/SOURCE-OFFER
	chmod 644 $(DESTDIR)$(LICENSE_DIR)/SOURCE-OFFER
	$(if $(ICON_THEMES_NOTICE),install -m 644 $(ICON_THEMES_NOTICE) \
	    $(DESTDIR)$(LICENSE_DIR)/ICON-THEMES-NOTICE.txt)

install-notices:
	@[ -n "$(VENDOR_DIR)" ] || { echo "install-notices: set VENDOR_DIR"; exit 1; }
	install -d $(DESTDIR)$(LICENSE_DIR) $(TARGET_DIR)
	cargo metadata --offline --locked --format-version 1 > $(TARGET_DIR)/notices-metadata.json
	python3 -I tools/gen-notices.py $(VENDOR_DIR) $(TARGET_DIR)/notices-metadata.json \
	    $(DESTDIR)$(LICENSE_DIR)/THIRD-PARTY-NOTICES.txt
	chmod 644 $(DESTDIR)$(LICENSE_DIR)/THIRD-PARTY-NOTICES.txt

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
