use std::fmt::Debug;

#[cfg(feature = "image")]
use image::ImageBuffer;

#[allow(unused_imports)]
use crate::{ColorMode, Decoder, Error, PIXEL_BYTES};

/// An animation frame containing data and metadata produced by [`Decoder`]
///
/// Getting metadata:
/// ```rust
/// # use webp_animation::{Decoder, ColorMode};
/// #
/// # let buffer = std::fs::read("./data/animated.webp").unwrap();
/// # let decoder = Decoder::new(&buffer).unwrap();
/// # let frame = decoder.into_iter().next().unwrap();
/// #
/// assert_eq!(frame.dimensions(), (400, 400));
/// assert_eq!(frame.color_mode(), ColorMode::Rgba);
/// ```
///
/// Accessing frame data in raw [`ColorMode`] -encoded bytes:
/// ```rust
/// # use webp_animation::{Decoder, ColorMode};
/// #
/// # let buffer = std::fs::read("./data/animated.webp").unwrap();
/// # let decoder = Decoder::new(&buffer).unwrap();
/// # let frame = decoder.into_iter().next().unwrap();
/// #
/// assert_eq!(frame.data().len(), (400 * 400 * 4));
/// assert_eq!(frame.data()[0..4], [0, 0, 0, 255]);
/// ```
///
/// If `image` feature is enabled, frame can be converted into [`image::ImageBuffer`]:
/// ```rust
/// # use webp_animation::{Decoder, ColorMode};
/// #
/// # let buffer = std::fs::read("./data/animated.webp").unwrap();
/// # let decoder = Decoder::new(&buffer).unwrap();
/// # let frame = decoder.into_iter().next().unwrap();
/// #
/// ##[cfg(feature = "image")]
/// let image = frame.into_image().unwrap();
/// ##[cfg(feature = "image")]
/// assert_eq!(image.dimensions(), (400, 400));
/// ##[cfg(feature = "image")]
/// assert_eq!(image.height(), 400);
/// // image.save("frame.png");
/// ```
pub struct Frame {
    timestamp: i32,
    frame_data: Vec<u8>,

    #[allow(dead_code)]
    color_mode: ColorMode,
    dimensions: (u32, u32),
}

impl Frame {
    pub(crate) fn new_from_decoder(
        timestamp: i32,
        color_mode: ColorMode,
        frame_data: Vec<u8>,
        dimensions: (u32, u32),
    ) -> Self {
        Self {
            timestamp,
            color_mode,
            frame_data,
            dimensions,
        }
    }

    /// Get dimensions of the frame (`width`, `height`)
    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    /// Get [`ColorMode`] of the frame (consistent accross frames)
    pub fn color_mode(&self) -> ColorMode {
        self.color_mode
    }

    /// Get timestamp of the frame in milliseconds
    pub fn timestamp(&self) -> i32 {
        self.timestamp
    }

    /// Get decoded frame data, size `width` * `height` * 4, pixels in [`ColorMode`] format
    pub fn data(&self) -> &[u8] {
        &self.frame_data
    }

    /// Convert the frame to [`image::ImageBuffer`] in `Rgba<u8>` format
    ///
    /// Must have [`ColorMode`] set to [`ColorMode::Rgba`] (default) when creating
    /// [`Decoder`]
    ///
    /// Requires feature `image` to be enabled
    ///
    /// ```
    /// # use webp_animation::{Decoder, DecoderOptions, ColorMode};
    /// #
    /// let buffer = std::fs::read("./data/animated.webp").unwrap();
    /// let decoder = Decoder::new(&buffer).unwrap();
    /// let frame = decoder.into_iter().next().unwrap();
    /// let _image = frame.into_image().unwrap();
    /// // _image.save("my_frame.jpg");
    /// ```
    #[cfg(feature = "image")]
    pub fn into_image(self) -> Result<ImageBuffer<image::Rgba<u8>, Vec<u8>>, Error> {
        self.into_rgba_image()
    }

    /// Convert the frame to [`image::ImageBuffer`] in `Rgba<u8>` format
    ///
    /// Must have [`ColorMode`] set to [`ColorMode::Rgba`] (default) when creating
    /// [`Decoder`]
    #[cfg(feature = "image")]
    pub fn into_rgba_image(self) -> Result<ImageBuffer<image::Rgba<u8>, Vec<u8>>, Error> {
        if self.color_mode != ColorMode::Rgba {
            return Err(Error::WrongColorMode(self.color_mode, ColorMode::Rgba));
        }

        Ok(ImageBuffer::from_vec(self.dimensions.0, self.dimensions.1, self.frame_data).unwrap())
    }

    /// Convert the frame to [`image::ImageBuffer`] in `Bgra<u8>` format
    ///
    /// Must have [`ColorMode`] set to [`ColorMode::Bgra`] when creating [`Decoder`]
    #[cfg(feature = "image")]
    pub fn into_bgra_image(self) -> Result<ImageBuffer<image::Bgra<u8>, Vec<u8>>, Error> {
        if self.color_mode != ColorMode::Bgra {
            return Err(Error::WrongColorMode(self.color_mode, ColorMode::Bgra));
        }
        Ok(ImageBuffer::from_vec(self.dimensions.0, self.dimensions.1, self.frame_data).unwrap())
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Frame {{ timestamp: {}, frame_data: {}b }}",
            self.timestamp,
            self.frame_data.len()
        )
    }
}
