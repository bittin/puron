# Changelog

The notable changes of each Puron release. Versions follow
[Semantic Versioning](https://semver.org/), and the format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.7.0] - 2026-11-23

### Added

- Each temperature in the popup is colored by how hot it runs: in the normal text color while it is cool, in the
  theme's warning color when it runs warm, and in the theme's red when it runs hot. The limits suit each kind of
  device, so a CPU or a graphics card may run warmer than a drive before it changes color, and a temperature has the
  same color whether it is shown in Celsius or in Fahrenheit. Only the value is colored, never the name; fan, voltage
  and power readings keep the normal text color.
- Each device name in the popup has the icon of its kind of device beside it, such as the CPU, the graphics card, a
  drive, the Wi-Fi card, a wired network card or a memory module, in that kind's color.

### Changed

- The readings in the popup now have the look of the Settings and About pages, in the desktop theme's colors, so that
  a light and a dark theme both work. Each section heading has a small colored tile with the section's icon. Each
  device's readings sit in a rounded card with a thin divider between two readings, and the fan, voltage and power
  readings of a section share one card. The readings, their order, their texts and what a click does stay the same,
  and so does the panel.
- The check mark of a pinned reading is in the theme's accent color, and "Settings" and "About" each have a small icon
  beside their text.
- On the Settings page, the tile of the section "Generic Sensor Providers" now has the same color as the Temperature
  tile in the popup.
- On the Settings page, a long label or state line under a sensor provider's switch now wraps instead of squeezing
  the switch.

## [1.6.1] - 2026-11-22

### Added

- Puron now speaks Swedish, German, Portuguese (Brazil), French, Spanish and Italian: the popup, its settings and its
  About page are shown in that language when the desktop uses it. A text that a translation lacks is shown in English.
- Anyone can add a language, and a partial translation is welcome: the section "Translating Puron" of the README says
  how. A translation that lacks texts never stops Puron from building.
- Swedish translation by Luna Jernberg (@bittin).
- The German, Portuguese, French, Spanish and Italian translations, and the Swedish texts of the About page, are machine-assisted; corrections are welcome.

### Fixed

- The applet's tests pass on a machine whose desktop uses a language other than English: they choose English
  themselves.

## [1.6.0] - 2026-11-20

### Added

- Each pinned temperature on the panel shows an icon for its kind of device, such as the CPU, the graphics card, a
  drive, the Wi-Fi card, a wired network card or a memory module, so that two pinned temperatures can be told apart by
  the icon alone.
- Puron is also built as a Flatpak, with the app ID io.github.elalonde.Puron, for the COSMIC Flatpak repository. In its
  sandbox it reads the same sensors as the .deb, except liquidctl devices: the liquidctl switch stays, and the About
  page says that liquidctl is not available in the Flatpak.
- Puron has its own app icon. COSMIC Settings shows it for the applet in every package, and the panel shows its
  single-color form when nothing is pinned.
- "About" in the popup now opens a page in the popup with the version, the package (Debian package, Flatpak or built
  from source), the license, the copyright, the state of each sensor source, and links to the homepage, the source
  code, the issue tracker and the third-party notices. The details sit in rounded cards, and the state of each sensor
  source is shown in color, in the desktop theme's colors. A button on that page copies these details for a bug
  report; they hold no host name, user name or serial number.

### Changed

- "Settings" in the popup now opens the settings as a page in the popup, with the same options. Each group of options
  has a heading with a colored icon, and its options sit in a rounded card. Under each sensor provider's switch, a line
  says whether that source is available (with its number of readings), off, not available, or not available in the
  Flatpak. The separate settings program puron-settings is gone.
- Every page of the popup has the same side margins.
- The copyright holder is now Eric Lalonde.

## [1.5.0] - 2026-11-17

### Changed

- The panel button keeps its width when a value gets shorter, so that the other sensors on the button and the applets
  beside Puron do not move when, for example, a fan drops from 1234 to 0 rpm.
- Each value on the panel button sits in a box as wide as the widest normal value of its kind, such as 888°C for a
  temperature or 8888 rpm for a fan, and the value is aligned to the right of its box. Most buttons are therefore wider
  than before.
- A box grows when a value needs more digits, such as a fan at 12000 rpm, and keeps that width. It becomes narrow again
  only when Puron starts, when a setting that changes the value texts or their size changes (the temperature unit, the
  decimals, showing or hiding a unit, or the panel or dock text size), or when the sensor leaves the panel button.
- The same holds on a vertical panel or dock: each box has the same width as on a horizontal panel and is centered in
  the column.

## [1.4.0] - 2026-11-17

### Added

- Display Options has two new settings, "Panel Text Size" and "Dock Text Size", for the size of the values on the panel
  button. The first applies when Puron is on a panel, the second when it is on a dock. Each can hold a fixed size from
  10 to 24 px.

### Changed

- By default the text follows the size of the panel or dock ("Match panel size", "Match dock size"), so that it grows
  with the icons. On the smallest panel it stays 14 px, as before; on a larger panel or dock it is larger after the
  update. Choose a fixed size of 14 px to keep the old look.
- On a horizontal panel, a text taller than the panel is cut evenly at the panel's top and bottom edges, instead of
  making Puron taller than the panel.

## [1.3.2] - 2026-11-16

### Changed

- The program itself does not change: it is the same as in 1.3.1. Only the checks that come with the source tree
  change.
- The commit-message hook, and tools/lint-a.sh, which checks comments and documentation, refuse more forms of
  internal reference numbers and tag names.
- The commit-message hook accepts one more trailer, "Claude-Session:" followed by a web address on the same line.
- One locale test gets ten minutes instead of two, so that it passes on a cold build, which starts from an empty target
  directory.

## [1.3.1] - 2026-11-13

### Changed

- Third-party libraries are updated to their latest compatible releases, bringing upstream bug fixes.

## [1.3.0] - 2026-11-12

### Added

- Sensor Settings has a new switch, "lm-sensors (drives only)", under Drive Sensor Providers. It is on by default. Turn
  it off and lm-sensors stops reading drive temperatures (NVMe drives, and SATA drives with the drivetemp module), so a
  disk that lm-sensors kept awake can spin down. The other lm-sensors readings stay. The switch has an effect only
  while lm-sensors is on.
- The README shows the popup and the panel.

## [1.2.0] - 2026-11-10

### Changed

- The last two entries of the popup are now "Settings" and then "About" (they were "Go to the Puron homepage" and then
  "Sensor Settings").
- "About", and the message that offers the homepage, open the new project page, https://elalonde.dev/projects/puron/.

## [1.1.2] - 2026-11-08

### Fixed

- With every sensor provider switched off, a change in Sensor Settings now updates the panel at once.
- Closing Sensor Settings keeps a poll interval that was typed but not yet confirmed.
- A failed save in Sensor Settings now shows "Settings could not be saved" until a save succeeds.
- Without a user configuration directory, Puron and Sensor Settings no longer read or write settings in the current
  directory.
- The diagnostics log no longer says a liquidctl run did not start when it started but could not be watched to its
  end.

Changes to the repository only, not to the shipped program:

- The checks now verify the names of test time limits.
- A test tool waits correctly for every provider's first report.

## [1.1.1] - 2026-11-08

### Fixed

- A Maximum or CPU pin kept from version 1.0.0 that waits for lm-sensors now has a line in the popup, "CPU: waiting
  for lm-sensors"; click it to remove the pin.
- A click in the popup no longer checks another line when the readings change just before the click.

Changes to the repository only, not to the shipped program:

- The checks now include clippy.
- Publishing a release retries reading back its new draft.

## [1.1.0] - 2026-11-07

### Changed

- The popup shows everything at once, in the sections Temperature, Fan, Voltage and Power. Nothing opens or closes
  any more, so the arrows and the "Group Items" settings are gone.
- Temperatures are listed by device: the CPU first, with every CPU reading under it, then GPUs, drives and other chips,
  each under its name, or the label you set in lm-sensors on a drive's "Composite" sensor.
- A device with several readings also has a "Maximum" line, the highest of them at each poll, and marks one line
  "(default)": the CPU's package reading, a drive's composite reading, otherwise the first.
- A click on a line checks it, and a checked line shows on the panel; several can be checked, and the panel shows them
  in popup order. A new install checks each "(default)" line. The Maximum that version 1.0.0 pinned moves to the CPU's
  default line, once.
- With lm-sensors and the "nvme-cli" switch both on, a drive no longer shows the same reading twice: the lm-sensors
  line stays, and a pin on the other one moves to it.
- The check mark is a small fixed size with a gap before the text, and section headings are drawn stronger.
- Every line stays on one line. The popup grows to fit, and text over 44 characters is shortened with "…". A popup
  taller than its limit scrolls.

Changes to the repository only, not to the shipped program:

- The checks run on GitHub Actions, and the README shows their status.
- This changelog.

## [1.0.0] - 2026-11-05

First release.

- A panel applet for the COSMIC desktop that displays hardware sensor
  readings in the COSMIC Panel and Dock: component temperatures, fan
  speeds, voltages and liquid cooling statistics.
- A settings application, `puron-settings`.
- A Debian package for amd64, `puron_1.0.0_amd64.deb`.
- The complete corresponding source as a release asset,
  `puron-1.0.0-source.tar.xz`, with the locked dependencies vendored for
  an offline build.
