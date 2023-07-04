# webp-animation &emsp; [![Build Status]][actions] [![Latest Version]][crates.io] [![Docs Version]][docs] [![Lines of Code]][github]

[Build Status]: https://img.shields.io/github/workflow/status/blaind/webp-animation/test
[actions]: https://github.com/blaind/webp-animation/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/webp-animation.svg
[crates.io]: https://crates.io/crates/webp-animation
[Lines of Code]: https://tokei.rs/b1/github/blaind/webp-animation?category=code
[github]: https://github.com/blaind/webp-animation
[Docs Version]: https://docs.rs/webp-animation/badge.svg
[docs]: https://docs.rs/webp-animation

A high-level Rust wrapper for decoding and encoding
[WebP](https://en.wikipedia.org/wiki/WebP) animations

![Example](data/example.gif)

_See `examples/encode_animation.rs` for source code of encoding the above image - example converted to gif for all-browser support, see the [example.webp file](data/example.webp)_

Underlying WebP format processing is handled by C-based
[libwebp](https://developers.google.com/speed/webp/docs/container-api) library,
which is interfaced through Rust [libwebp-sys2](https://crates.io/crates/libwebp-sys2)
crate.

Functional Goals:

- Easy-to-use API that looks like Rust
- Enable decoding and encoding of WebP streams
- All configuration flags provided by `libwebp` should be usable

Non-functional Goals:

- High performance (approach `libwebp` performance without large overhead)
- Write compherensive test cases, and test by automation
- Ensure safety (no memory leaks or UB). Fuzz the API's. Safe to use for end users

Non-goals

- Provide other WebP/libwebp -related functionality (such as image en/decoding or muxing). For this functionality, see e.g. [libwebp-image](https://crates.io/crates/libwebp-image) or [webp](https://crates.io/crates/webp)

## Examples

### Decoding

Will take a webp buffer, and try to decode it to frame(s)

```rust
use webp_animation::prelude::*;

let buffer = std::fs::read("./data/animated.webp").unwrap();
let decoder = Decoder::new(&buffer).unwrap();

for frame in decoder.into_iter() {
  assert_eq!(frame.dimensions(), (400, 400));

  // w * h * rgba
  assert_eq!(frame.data().len(), 400 * 400 * 4);

  // if feature `image` is enabled (not by default),
  // one can convert data to [`Image::ImageBuffer`]
  assert_eq!(
    frame.into_image().unwrap().dimensions(),
    (400, 400)
  );
}
```

It is also possible to supply more decoding options through `Decoder::new_with_options`.

### Encoding

Will take `n` frames as an input. WebP binary data is output at the end
(wrapped into `WebPData` which acts as a `&[u8]`)

```rust
use webp_animation::prelude::*;

// setup
let dimensions = (64, 32);
let bright_frame = [255, 255, 255, 255].repeat(64 * 32);
let dark_frame = [0, 0, 0, 255].repeat(64 * 32);

// init encoder. uses by default lossless encoding,
// for other alternatives see documentation about
// `new_with_options`
let mut encoder = Encoder::new(dimensions).unwrap();

// insert frames to specific (increasing) timestamps
for frame_idx in 0..5 {
  let rgba_data = if frame_idx % 2 == 0 {
    &bright_frame
  } else {
    &dark_frame
  };

  // (presentation) timestamp of the frame, should be in increasing order. represented in milliseconds
  let frame_timestamp_ms = frame_idx * 170;

  encoder.add_frame(rgba_data, frame_timestamp_ms).unwrap();
}

// final timestamp in milliseconds, until to the last frame is shown
let final_timestamp_ms = 1_000;

// get encoded webp data
let webp_data = encoder.finalize(final_timestamp_ms).unwrap();
std::fs::write("my_animation.webp", webp_data).unwrap();
```

See the [documentation](https://docs.rs/webp-animation/latest/webp_animation) for other encoding options, e.g.
for lossy encoding. For tuning the options, use the [`Encoder::new_with_options`](https://docs.rs/webp-animation/latest/webp_animation/struct.Encoder.html#method.new_with_options) method.

## Future plans

Keep up with upstream `libwebp` changes.

Possibly provide a compherensive CLI for working with WebP animations in future (conversions, optimizations, etc.)

## License

Licensed under either of

- <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or
- <a href="LICENSE-MIT">MIT license</a>

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the software by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
