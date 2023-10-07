use std::{fmt::Debug, mem, pin::Pin};

use libwebp_sys as webp;

use crate::{ColorMode, Error, Frame};

const MAX_CANVAS_SIZE: usize = 3840 * 2160; // 4k

/// An options struct for [`Decoder`]
///
/// For usage, see [`Decoder::new_with_options`]
pub struct DecoderOptions {
    /// If true, use multi-threaded decoding. Enabled by default
    pub use_threads: bool,
    /// Output colorspace. [`ColorMode::Rgba`] by default. Affects [`Frame`] output
    pub color_mode: ColorMode,
}

impl Default for DecoderOptions {
    fn default() -> Self {
        Self {
            use_threads: true,
            color_mode: ColorMode::Rgba,
        }
    }
}

/// A decoder for webp animation data
///
/// Will take a webp buffer, and try to decode it to frame(s)
///
/// ```rust
/// use webp_animation::prelude::*;
///
/// let buffer = std::fs::read("./data/animated.webp").unwrap();
/// let decoder = Decoder::new(&buffer).unwrap();
///
/// for frame in decoder.into_iter() {
///   assert_eq!(frame.dimensions(), (400, 400));
///   assert_eq!(frame.data().len(), 400 * 400 * 4); // w * h * rgba
/// }
/// ```
///
/// See also documentation for the item produced by iterator: [`Frame`]
///
/// If `image` feature is enabled, frames can be produced in [`image::ImageBuffer`]
/// format:
/// ```rust
/// use webp_animation::prelude::*;
/// #
/// # let buffer = std::fs::read("./data/animated.webp").unwrap();
/// # let decoder = Decoder::new(&buffer).unwrap();
/// #
/// for frame in decoder.into_iter() {
///   ##[cfg(feature = "image")]
///   assert_eq!(frame.into_image().unwrap().dimensions(), (400, 400));
/// }
/// ```
pub struct Decoder<'a> {
    buffer: &'a [u8],
    decoder_wr: DecoderWrapper,
    info: webp::WebPAnimInfo,
    options: DecoderOptions,
}

impl<'a> Decoder<'a> {
    /// Construct a new decoder from webp `buffer`
    ///
    /// Returns an [`Error`] in case of a decoding failure (e.g. malformed input)
    ///
    /// ```
    /// # use webp_animation::prelude::*;
    /// #
    /// let buffer = std::fs::read("./data/animated.webp").unwrap();
    /// let decoder = Decoder::new(&buffer).unwrap();
    /// ```
    pub fn new(buffer: &'a [u8]) -> Result<Self, Error> {
        Decoder::new_with_options(buffer, Default::default())
    }

    /// Construct a new decoder from webp `buffer`
    ///
    /// Returns an [`Error`] in case of a decoding failure (e.g. malformed input)
    ///
    /// ```
    /// # use webp_animation::prelude::*;
    /// #
    /// let buf = std::fs::read("./data/animated.webp").unwrap();
    /// let decoder = Decoder::new_with_options(&buf, DecoderOptions {
    ///   use_threads: false,
    ///   color_mode: ColorMode::Bgra
    /// }).unwrap();
    /// ```
    pub fn new_with_options(buffer: &'a [u8], options: DecoderOptions) -> Result<Self, Error> {
        if buffer.is_empty() {
            return Err(Error::ZeroSizeBuffer);
        }

        let mut decoder_options = Box::pin(unsafe {
            let mut options = mem::zeroed();

            if webp::WebPAnimDecoderOptionsInit(&mut options) != 1 {
                return Err(Error::OptionsInitFailed);
            }

            options
        });

        decoder_options.use_threads = if options.use_threads { 1 } else { 0 };
        decoder_options.color_mode = match options.color_mode {
            ColorMode::Rgba => libwebp_sys::MODE_RGBA,
            ColorMode::Bgra => libwebp_sys::MODE_BGRA,
            ColorMode::Rgb => libwebp_sys::MODE_RGB,
            ColorMode::Bgr => libwebp_sys::MODE_BGR,
        };

        // pin data (& options above) because decoder takes reference to them
        let data = Box::pin(webp::WebPData {
            bytes: buffer.as_ptr(),
            size: buffer.len(),
        });

        let decoder_wr = DecoderWrapper::new(data, decoder_options)?;

        let info = unsafe {
            let mut info = mem::zeroed();
            if webp::WebPAnimDecoderGetInfo(decoder_wr.decoder, &mut info) != 1 {
                return Err(Error::DecoderGetInfoFailed);
            }
            info
        };

        // prevent too large allocations
        if info.canvas_width * info.canvas_height > MAX_CANVAS_SIZE as u32 {
            return Err(Error::TooLargeCanvas(
                info.canvas_width,
                info.canvas_height,
                MAX_CANVAS_SIZE,
            ));
        }

        log::trace!("Decoder initialized. {:?}", info);

        Ok(Self {
            buffer,
            decoder_wr,
            info,
            options,
        })
    }

    /// Returns dimensions for webp frames (`width`, `height`)
    ///
    /// ```
    /// # use webp_animation::prelude::*;
    /// #
    /// let buffer = std::fs::read("./data/animated.webp").unwrap();
    /// let decoder = Decoder::new(&buffer).unwrap();
    /// assert_eq!(decoder.dimensions(), (400, 400));
    /// ```
    pub fn dimensions(&self) -> (u32, u32) {
        (self.info.canvas_width, self.info.canvas_height)
    }

    fn has_more_frames(&self) -> bool {
        let frames = unsafe { webp::WebPAnimDecoderHasMoreFrames(self.decoder_wr.decoder) };
        frames > 0
    }
}

impl<'a> Debug for Decoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let info = &self.info;

        write!(f, "Decoder {{ buffer: {}b, info: {{ w: {}, h: {}, loop_cnt: {}, bgcolor: 0x{:x}, frame_count: {} }} }}", self.buffer.len(), info.canvas_width, info.canvas_height, info.loop_count, info.bgcolor, info.frame_count)
    }
}

struct DecoderWrapper {
    decoder: *mut webp::WebPAnimDecoder,

    #[allow(dead_code)]
    data: Pin<Box<webp::WebPData>>,
    #[allow(dead_code)]
    options: Pin<Box<webp::WebPAnimDecoderOptions>>,
}

impl DecoderWrapper {
    pub fn new(
        data: Pin<Box<webp::WebPData>>,
        options: Pin<Box<webp::WebPAnimDecoderOptions>>,
    ) -> Result<Self, Error> {
        let decoder = unsafe { webp::WebPAnimDecoderNew(&*data, &*options) };
        if decoder.is_null() {
            return Err(Error::DecodeFailed);
        }

        Ok(Self {
            decoder,
            data,
            options,
        })
    }
}

impl Drop for DecoderWrapper {
    fn drop(&mut self) {
        unsafe { webp::WebPAnimDecoderDelete(self.decoder) };
    }
}

impl<'a> IntoIterator for Decoder<'a> {
    type Item = Frame;

    type IntoIter = DecoderIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DecoderIterator::new(self)
    }
}

/// An iterator that produces decoded [`Frame`]'s from webp data
pub struct DecoderIterator<'a> {
    animation_decoder: Decoder<'a>,
}

impl<'a> DecoderIterator<'a> {
    fn new(animation_decoder: Decoder<'a>) -> Self {
        Self { animation_decoder }
    }
}

impl<'a> Iterator for DecoderIterator<'a> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.animation_decoder.has_more_frames() {
            return None;
        }

        let mut output_buffer = std::ptr::null_mut();
        let mut timestamp: i32 = 0;

        if unsafe {
            webp::WebPAnimDecoderGetNext(
                self.animation_decoder.decoder_wr.decoder,
                &mut output_buffer,
                &mut timestamp,
            )
        } != 1
        {
            // "False if any of the arguments are NULL, or if there is a parsing or decoding error, or if there are no more frames. Otherwise, returns true."
            log::warn!("webp::WebPAnimDecoderGetNext did not return success - frame parsing failed, parsing/decoding error?");
            return None;
        }

        if output_buffer.is_null() {
            log::error!("webp::WebPAnimDecoderGetNext returned null output ptr, can not decode a frame. This should not happen");
            return None;
        }

        let info = &self.animation_decoder.info;
        let opts = &self.animation_decoder.options;
        let data = unsafe {
            std::slice::from_raw_parts(
                output_buffer,
                info.canvas_width as usize * info.canvas_height as usize * opts.color_mode.size(),
            )
        };

        log::trace!(
            "Decoded a frame, timestamp {}, {} bytes",
            timestamp,
            data.len()
        );

        Some(Frame::new_from_decoder(
            timestamp,
            self.animation_decoder.options.color_mode,
            data.to_vec(),
            self.animation_decoder.dimensions(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn test_decoder_failure() {
        let decoder = Decoder::new(&[]);
        assert_eq!(decoder.unwrap_err(), Error::ZeroSizeBuffer);

        let decoder = Decoder::new(&[0x00, 0x01]);
        assert_eq!(decoder.unwrap_err(), Error::DecodeFailed);

        let mut buffer = Vec::new();
        File::open("./data/animated.webp")
            .unwrap()
            .read_to_end(&mut buffer)
            .unwrap();

        let decoder = Decoder::new(&buffer[..1500]);
        assert_eq!(decoder.unwrap_err(), Error::DecodeFailed);
    }

    fn get_animated_buffer() -> Vec<u8> {
        let mut buffer = Vec::new();
        File::open("./data/animated.webp")
            .unwrap()
            .read_to_end(&mut buffer)
            .unwrap();
        buffer
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_decode_to_image() {
        use std::io::Cursor;

        use image::{codecs::png::PngDecoder, DynamicImage, ImageDecoder as _, ImageOutputFormat};

        let buffer = get_animated_buffer();
        let decoder = Decoder::new(&buffer).unwrap();
        let mut iter = decoder.into_iter();
        let frame = iter.next().unwrap();
        let image = frame.into_image().unwrap();
        assert_eq!(image.dimensions(), (400, 400));

        let mut buf = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut buf, ImageOutputFormat::Png)
            .unwrap();

        let buf = buf.into_inner();

        let png_decoder = PngDecoder::new(&buf[..]).unwrap();
        assert_eq!(png_decoder.dimensions(), (400, 400));
    }

    #[test]
    fn test_decoder_success() {
        // read file
        let buffer = get_animated_buffer();

        // decode frames
        let decoder = Decoder::new_with_options(
            &buffer,
            DecoderOptions {
                color_mode: ColorMode::Rgba,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(decoder.dimensions(), (400, 400));
        let frames: Vec<_> = decoder.into_iter().collect();

        // various asserts
        let timestamps: Vec<_> = frames.iter().map(|f| f.timestamp()).collect();
        assert_eq!(timestamps, [40, 80, 120, 160, 200, 240, 280, 320, 360, 400]);
        assert_eq!(frames[0].data().len(), 400 * 400 * 4);
        assert_eq!(
            frames[2].data()[89394 * 4..89394 * 4 + 4],
            [167, 166, 167, 255]
        );

        assert_eq!(
            frames[2].data().iter().map(|x| *x as usize).sum::<usize>(),
            41668527
        )
    }

    #[test]
    fn test_fuzz_case_1() {
        // initially, this data caused 768MB allocation -> now an error is returned
        let data = [
            0x2f, 0xff, 0xff, 0xff, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let decoder = Decoder::new(&data);
        assert_eq!(
            decoder.unwrap_err(),
            Error::TooLargeCanvas(16384, 12288, MAX_CANVAS_SIZE)
        );
    }
}
