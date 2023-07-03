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

use std::fmt::{self, Display};

mod decoder;
mod encoder;
mod encoder_config;
mod frame;
mod webp_data;

pub use decoder::*;
pub use encoder::*;
pub use encoder_config::*;
pub use frame::*;
pub use webp_data::*;

pub mod prelude {
    // general
    pub use crate::ColorMode;

    // decoder
    pub use crate::{Decoder, DecoderOptions};

    // encoder
    pub use crate::{Encoder, EncoderOptions, EncodingConfig, EncodingType, LossyEncodingConfig};
}

const PIXEL_BYTES: usize = 4;

/// Color Mode that configures the output type of [`Decoder`] [`Frame`]'s
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ColorMode {
    /// Rgb (red, green, blue) -no alpha
    Rgb,
    /// Rgba (red, green, blue, alpha)
    Rgba,
    /// Bgra (blue, green, red, alpha)
    Bgra,
    // Bgr (blue, green, red) - no alpha
    Bgr,
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
    TimestampMustBeHigherThanPrevious(i32, i32),

    /// Timestamp must be higher or equal to the previous frame timestamp
    TimestampMustBeEqualOrHigherThanPrevious(i32, i32),

    /// Encoder webp assembly failed
    EncoderAssmebleFailed,

    /// Supplied dimensions must be positive
    DimensionsMustbePositive,

    /// No frames have been supplied to encoder
    NoFramesAdded,

    /// Supplied zero-sized buffer where bytes where expected
    ZeroSizeBuffer,

    /// Encoder config validation failed
    InvalidEncodingConfig,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OptionsInitFailed => write!(f, "OptionsInitFailed: Initializing webp options failed, internal (memory allocation?) failure"),
            Error::DecodeFailed => write!(f, "DecodeFailed: Could not decode input bytes, possibly malformed data"),
            Error::DecoderGetInfoFailed => write!(f, "DecoderGetInfoFailed: Decoder could not get metadata of webp stream. Corrupt data?"),
            Error::TooLargeCanvas(width, height, max_size) => write!(f, "TooLargeCanvas: Decodable canvas is too large ({} x {} = {} pixels). For now, size is limited to 3840 * 2160 = {} pixels", width, height, width * height, max_size),
            Error::EncoderCreateFailed => write!(f, "EncoderCreateFailed: Encoder create failed. Wrong options combination?"),
            Error::BufferSizeFailed(expected, received) => write!(f, "BufferSizeFailed: Expected (width * height * 4 = {}) bytes as input buffer, got {} bytes", expected, received),
            Error::PictureImportFailed => write!(f, "PictureImportFailed: Raw data could not be converted into webp frame by underlying libwebp library"),
            Error::EncoderAddFailed => write!(f, "EncoderAddFailed: Frame could not be added to webp stream by underlying libwebp library"),
            Error::WrongColorMode(requested, expected) => write!(f, "WrongColorMode: Requested image in {:?} format but underlying is stored as {:?}", expected, requested),
            Error::TimestampMustBeHigherThanPrevious(requested, previous) => write!(f, "TimestampMustBeHigherThanPrevious: Supplied timestamp (got {}) must be higher than {}", requested, previous),
            Error::TimestampMustBeEqualOrHigherThanPrevious(requested, previous) => write!(f, "TimestampMustBeEqualOrHigherThanPrevious: Supplied timestamp (got {}) must be higher or equal to {}", requested, previous),
            Error::EncoderAssmebleFailed => write!(f, "EncoderAssmebleFailed: Encoder webp assembly failed"),
            Error::DimensionsMustbePositive => write!(f, "DimensionsMustbePositive: Supplied dimensions must be positive"),
            Error::NoFramesAdded => write!(f, "NoFramesAdded: No frames have been added yet"),
            Error::ZeroSizeBuffer => write!(f, "ZeroSizeBuffer: Buffer contains no data"),
            Error::InvalidEncodingConfig => write!(f, "InvalidEncodingConfig: encoding configuration validation failed")
        }
    }
}

impl std::error::Error for Error {}
