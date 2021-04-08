//! # Overview
//!
//! This crate provides a high-level Rust wrapper for decoding and encoding
//! [WebP](https://en.wikipedia.org/wiki/WebP) animations.
//! Underlying WebP format processing is handled by C-based
//! [libwebp](https://developers.google.com/speed/webp/docs/container-api) library by Google,
//! which is interfaced through Rust [libwebp-sys2](https://crates.io/crates/libwebp-sys2)
//! crate
//!
//! # Usage
//! Have a look at [`Decoder`] and [`Encoder`] for use-case specific examples.
mod decoder;
mod encoder;
mod frame;
mod webp_data;

pub use decoder::*;
pub use encoder::*;
pub use frame::*;
pub use webp_data::*;

const PIXEL_BYTES: usize = 4;

/// Color Mode that configures the output type of [`Decoder`] [`Frame`]'s
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ColorMode {
    /// Rgba (red, green, blue, alpha)
    Rgba,
    /// Bgra (blue, green, red, alpha)
    Bgra,
    // what about MODE_rgbA and MODE_bgrA ?
}

/// Error type produced by `webp_animation` code
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Initializing webp options failed, internal (memory allocation?) failure
    OptionsInitFailed,

    /// Decoder init failed, input contains wrong bytes
    DecodeFailed,

    /// Decoder could not get metadata of webp stream. Corrupt data?
    DecoderGetInfoFailed,

    /// Webp stream contains too large canvas. For now, size is limited to 3840 * 2160 pixels
    /// See `MAX_CANVAS_SIZE` variable from code
    TooLargeCanvas(u32, u32, usize),

    /// Encoder create failed. Wrong options combination?
    EncoderCreateFailed,

    /// Data input buffer size did not match encoder metadata (width * height * 4)
    BufferSizeFailed(usize, usize),

    /// Raw data could not be converted into webp frame by underlying libwebp library
    PictureImportFailed,

    /// Frame could not be added to webp stream by underlying libwebp library
    EncoderAddFailed,

    /// Underlying data is in different color mode
    WrongColorMode(ColorMode, ColorMode),

    /// Timestamp must be higher value than previous frame timestamp
    TimestampMustBeHigherThanPrevious,

    /// Timestamp must be higher or equal to the previous frame timestamp
    TimestampMustBeEqualOrHigherThanPrevious,

    /// Encoder webp assembly failed
    EncoderAssmebleFailed,
}
