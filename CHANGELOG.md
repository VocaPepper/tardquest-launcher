# Changelog

All notable changes to the TQ Launcher are documented in this file.

## 2.3.2608

### Fixed
- Switching editions now re-scans the install directory so the new edition's installed builds are detected immediately, defaults the Build dropdown to an installed version when one exists, and resets the build list while its channel loads instead of showing stale options.
- Edition and Build dropdown boxes keep a consistent width and truncate long version numbers instead of stretching for the "(Installed)" tag.
- The white Play button stays white on hover instead of being overridden by the generic button highlight.

## 2.2.2608

### Fixed
- Edition and Build dropdowns no longer render white-on-white on Linux.
- Launcher window is now resizable so the action buttons are not cut off on smaller displays.
- Right-hand panel scrolls internally when the window is short.

## 2.1.2608

### Changed
- Swapped the Release Notes and Activity Log tabs; Release Notes is now the default view.
- Privacy Policy now opens in a dedicated in-app window instead of an external page.
- Launcher now refuses to run on Windows builds older than 10 1809 (build 17763), where WebView2 is unavailable.

### Fixed
- Centered the Privacy Policy button vertically in the header.
- Vertically centered the Privacy Policy text.

## 2.0.2608

### Added
- Initial Tauri v2 launcher for TardQuest and TardQuest Online II.

### Changed
- Self-update via GitHub Releases with signed updater artifacts.
