# Changelog

[git_tag_comparison]: https://github.com/bevyengine/bevy/compare/v0.6.0...main

## Version 0.6.0 (2022-04-17)

[Compare changelog](https://github.com/blaind/webp-animation/compare/v0.5.0...v0.6.0)

### Added

- Info/println output into examples
- Testing on Windows & MacOS

### Changed

- Updated `imageproc` dependency from 0.22 to 0.23
- Updated `image` dependency from 0.23 to 0.24
- Removed `Frame::into_bgra_image` (since `image` crate removed Bgra functionality)

## Version 0.5.0 (2021-10-24)

### Added

- Minimum Rust Version 1.47 + automated tests for it

## Version 0.4.0 (2021-10-24)

### Changed

- [Disabled image default features][2]

[2]: https://github.com/blaind/webp-animation/pull/2
