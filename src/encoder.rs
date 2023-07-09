use std::{mem, pin::Pin, ptr};

use libwebp_sys as webp;

use crate::{
    ColorMode, ConfigContainer, EncoderOptions, EncodingConfig, Error, WebPData, PIXEL_BYTES,
};

#[allow(unused_imports)]
use crate::LossyEncodingConfig; // for docs

/// An encoder for creating webp animation
///
/// Will take `n` frames as an input. WebP binary data is output at the end
/// (wrapped into [`WebPData`] which acts as a `&[u8]`)
///
/// # Example without special configuration
///
/// ```rust
/// use webp_animation::prelude::*;
///
/// // setup
/// let dimensions = (64, 32);
/// let bright_frame = [255, 255, 255, 255].repeat(64 * 32);
/// let dark_frame = [0, 0, 0, 255].repeat(64 * 32);
///
/// // init encoder
/// let mut encoder = Encoder::new(dimensions).unwrap();
///
/// // insert frames to specific (increasing) timestamps
/// for i in 0..5 {
///   let rgba_data = if i % 2 == 0 { &bright_frame } else { &dark_frame };
///   let frame_timestamp_ms = i * 170;
///
///   encoder.add_frame(rgba_data, frame_timestamp_ms).unwrap();
/// }
///
/// // get encoded webp data
/// let final_timestamp_ms = 1_000;
/// let webp_data = encoder.finalize(final_timestamp_ms).unwrap();
/// // std::fs::write("my_animation.webp", webp_data);
/// ```
///
/// # Example with configuration
///
/// See [`EncodingConfig`] and [`LossyEncodingConfig`] for per-field explanations.
/// ```rust
/// use webp_animation::prelude::*;
///
/// let mut encoder = Encoder::new_with_options((640, 480), EncoderOptions {
///     kmin: 3,
///     kmax: 5,
///     encoding_config: Some(EncodingConfig {
///         quality: 75.,
///         encoding_type: EncodingType::Lossy(LossyEncodingConfig {
///             segments: 2,
///             alpha_compression: true,
///             ..Default::default()
///         }),
///         ..Default::default()
///     }),
///     ..Default::default()
/// }).unwrap();
/// ```
///
/// # Example with per-frame configuration
///
/// ```rust
/// use webp_animation::prelude::*;
///
/// let mut encoder = Encoder::new_with_options((640, 480), EncoderOptions {
///     kmin: 3,
///     kmax: 5,
///     ..Default::default()
/// }).unwrap();
///
/// encoder.add_frame_with_config(&[0u8; 640 * 480 * 4], 0, &EncodingConfig {
///     quality: 75.,
///     encoding_type: EncodingType::Lossy(LossyEncodingConfig {
///         segments: 2,
///         alpha_compression: true,
///         ..Default::default()
///     }),
///     ..Default::default()
/// }).unwrap();
/// ```
pub struct Encoder {
    encoder_wr: EncoderWrapper,
    frame: PictureWrapper,
    options: EncoderOptions,
    previous_timestamp: i32,
    encoding_config: Option<ConfigContainer>,
}

impl Encoder {
    /// Construct a new encoder with default options for dimensions (`width`, `height`)
    pub fn new(dimensions: (u32, u32)) -> Result<Self, Error> {
        Encoder::new_with_options(dimensions, Default::default())
    }

    /// Construct a new encoder with custom options for dimensions (`width`, `height`)
    pub fn new_with_options(
        dimensions: (u32, u32),
        options: EncoderOptions,
    ) -> Result<Self, Error> {
        if dimensions.0 == 0 || dimensions.1 == 0 {
            return Err(Error::DimensionsMustbePositive);
        }

        let enc_options = convert_options(&options)?;
        let encoder_wr = EncoderWrapper::new(dimensions, enc_options)?;

        log::trace!("Encoder initialized with dimensions {:?}", dimensions);

        let mut encoder = Self {
            encoder_wr,
            options: options.clone(),
            frame: PictureWrapper::new(dimensions)?,
            previous_timestamp: -1,
            encoding_config: None,
        };

        if let Some(config) = options.encoding_config {
            encoder.set_default_encoding_config(config)?;
        }

        Ok(encoder)
    }

    /// Add a new frame to be encoded
    ///
    /// Inputs
    /// * `data` is an array of pixels in [`ColorMode`] format set by [`EncoderOptions`]
    ///   ([`ColorMode::Rgba`] by default)
    /// * `timestamp_ms` of this frame in milliseconds. Duration of a frame would be
    ///   calculated as "timestamp of next frame - timestamp of this frame".
    ///   Hence, timestamps should be in non-decreasing order.
    pub fn add_frame(&mut self, data: &[u8], timestamp_ms: i32) -> Result<(), Error> {
        self.add_frame_internal(data, timestamp_ms, None)
    }

    /// Add a new frame to be encoded with special per-frame configuration ([`EncodingConfig`])
    ///
    /// See [`Encoder::add_frame`] for `data` and `timestamp` explanations
    pub fn add_frame_with_config(
        &mut self,
        data: &[u8],
        timestamp_ms: i32,
        config: &EncodingConfig,
    ) -> Result<(), Error> {
        self.add_frame_internal(data, timestamp_ms, Some(config))
    }

    fn add_frame_internal(
        &mut self,
        data: &[u8],
        timestamp: i32,
        config: Option<&EncodingConfig>,
    ) -> Result<(), Error> {
        if timestamp <= self.previous_timestamp {
            return Err(Error::TimestampMustBeHigherThanPrevious(
                timestamp,
                self.previous_timestamp,
            ));
        }

        self.frame.set_data(data, self.options.color_mode)?;

        if unsafe {
            webp::WebPAnimEncoderAdd(
                self.encoder_wr.encoder,
                self.frame.as_webp_picture_ref(),
                timestamp,
                match config {
                    Some(config) => {
                        let config = config.to_config_container()?;
                        config.as_ptr()
                    }
                    None => match &self.encoding_config {
                        Some(config) => config.as_ptr(),
                        None => std::ptr::null(),
                    },
                },
            )
        } == 0
        {
            return Err(Error::EncoderAddFailed);
        }

        self.previous_timestamp = timestamp;

        log::trace!(
            "Add a frame at timestamp {}ms, {} bytes",
            timestamp,
            data.len()
        );

        Ok(())
    }

    /// Sets the default encoding config
    ///
    /// Usually set in [`EncderOptions`] at constructor ([`Encoder::new_with_options`])
    pub fn set_default_encoding_config(&mut self, config: EncodingConfig) -> Result<(), Error> {
        self.encoding_config = Some(config.to_config_container()?);
        self.options.encoding_config = Some(config);
        Ok(())
    }

    /// Will encode the stream and return encoded bytes in a [`WebPData`] upon success
    ///
    /// `timestamp_ms` behaves as in [`Encoder::add_frame`], and determines the duration of the last frame
    pub fn finalize(self, timestamp_ms: i32) -> Result<WebPData, Error> {
        if self.previous_timestamp == -1 {
            // -1 = no frames added
            return Err(Error::NoFramesAdded);
        }

        if timestamp_ms < self.previous_timestamp {
            return Err(Error::TimestampMustBeEqualOrHigherThanPrevious(
                timestamp_ms,
                self.previous_timestamp,
            ));
        }

        if unsafe {
            webp::WebPAnimEncoderAdd(
                self.encoder_wr.encoder,
                ptr::null_mut(),
                timestamp_ms,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(Error::EncoderAddFailed);
        }

        let mut data = WebPData::new();

        if unsafe { webp::WebPAnimEncoderAssemble(self.encoder_wr.encoder, data.inner_ref()) } == 0
        {
            return Err(Error::EncoderAssmebleFailed);
        }

        log::trace!(
            "Finalize encoding at timestamp {}ms, output binary size {} bytes",
            timestamp_ms,
            data.len()
        );

        Ok(data)
    }
}

fn convert_options(
    options: &EncoderOptions,
) -> Result<Pin<Box<webp::WebPAnimEncoderOptions>>, Error> {
    let mut enc_options = Box::pin(unsafe {
        let mut enc_options = mem::zeroed();
        if webp::WebPAnimEncoderOptionsInit(&mut enc_options) != 1 {
            return Err(Error::OptionsInitFailed);
        }
        enc_options
    });

    enc_options.anim_params.loop_count = options.anim_params.loop_count;

    enc_options.minimize_size = if options.minimize_size { 1 } else { 0 };
    enc_options.kmin = options.kmin as i32;
    enc_options.kmax = options.kmax as i32;
    enc_options.allow_mixed = if options.allow_mixed { 1 } else { 0 };
    enc_options.verbose = if options.verbose { 1 } else { 0 };

    Ok(enc_options)
}

struct EncoderWrapper {
    encoder: *mut webp::WebPAnimEncoder,

    #[allow(dead_code)]
    options: Pin<Box<webp::WebPAnimEncoderOptions>>,
}

impl EncoderWrapper {
    pub fn new(
        dimensions: (u32, u32),
        options: Pin<Box<webp::WebPAnimEncoderOptions>>,
    ) -> Result<Self, Error> {
        let (width, height) = dimensions;

        let encoder = unsafe { webp::WebPAnimEncoderNew(width as i32, height as i32, &*options) };
        if encoder.is_null() {
            return Err(Error::EncoderCreateFailed);
        }

        Ok(Self { encoder, options })
    }
}

impl Drop for EncoderWrapper {
    fn drop(&mut self) {
        unsafe { webp::WebPAnimEncoderDelete(self.encoder) };
    }
}

struct PictureWrapper {
    picture: webp::WebPPicture,
}

impl PictureWrapper {
    pub fn new(dimensions: (u32, u32)) -> Result<Self, Error> {
        let mut picture = unsafe {
            let mut picture = mem::zeroed();
            assert!(webp::WebPPictureInit(&mut picture) != 0);
            picture
        };

        picture.width = dimensions.0 as i32;
        picture.height = dimensions.1 as i32;
        picture.use_argb = 1;

        Ok(Self { picture })
    }

    pub fn as_webp_picture_ref(&mut self) -> &mut webp::WebPPicture {
        &mut self.picture
    }

    pub fn set_data(&mut self, data: &[u8], color_mode: ColorMode) -> Result<(), Error> {
        let received_len = data.len();
        let expected_len = self.data_size();
        if received_len != expected_len {
            return Err(Error::BufferSizeFailed(expected_len, received_len));
        }

        if unsafe {
            match color_mode {
                ColorMode::Rgba => webp::WebPPictureImportRGBA(
                    &mut self.picture,
                    data.as_ptr(),
                    self.picture.width * 4,
                ),
                ColorMode::Bgra => webp::WebPPictureImportBGRA(
                    &mut self.picture,
                    data.as_ptr(),
                    self.picture.width * 4,
                ),
                ColorMode::Rgb => webp::WebPPictureImportRGB(
                    &mut self.picture,
                    data.as_ptr(),
                    self.picture.width * 3,
                ),
                ColorMode::Bgr => webp::WebPPictureImportBGR(
                    &mut self.picture,
                    data.as_ptr(),
                    self.picture.width * 3,
                ),
            }
        } == 0
        {
            return Err(Error::PictureImportFailed);
        }

        Ok(())
    }

    fn data_size(&self) -> usize {
        self.picture.width as usize * self.picture.height as usize * PIXEL_BYTES
    }
}

impl Drop for PictureWrapper {
    fn drop(&mut self) {
        unsafe { webp::WebPPictureFree(&mut self.picture) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decoder, EncodingType, Frame, LossyEncodingConfig};
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn test_encoder() {
        // read frames to be encoded
        let mut frames = read_frames();

        // encode data
        let mut encoder = Encoder::new((400, 400)).unwrap();
        for frame in &mut frames {
            encoder.add_frame(frame.data(), frame.timestamp()).unwrap();
        }
        let webp_data = encoder.finalize(440).unwrap();
        assert!(webp_data.len() > 0);
        assert_eq!(&webp_data[..5], &[82, 73, 70, 70, 18]);

        // decode previously encoded data
        let decoder = Decoder::new(&webp_data).unwrap();
        let decoded_frames: Vec<_> = decoder.into_iter().collect();

        // assert that re-encoded matches the decoded frames
        assert_eq!(frames.len(), decoded_frames.len());
        for (f1, f2) in decoded_frames.iter().zip(frames) {
            assert_eq!(f1.dimensions(), f2.dimensions());
            assert_eq!(f1.color_mode(), f2.color_mode());
            assert_eq!(f1.timestamp(), f2.timestamp());
            assert_eq!(f1.data(), f2.data());
        }
    }

    fn read_frames() -> Vec<Frame> {
        let mut buf = Vec::new();
        File::open("./data/animated.webp")
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();

        let decoder = Decoder::new(&buf).unwrap();
        let frames: Vec<_> = decoder.into_iter().collect();
        frames
    }

    #[test]
    fn test_enc_options() {
        let mut encoder = Encoder::new((400, 400)).unwrap();
        encoder.add_frame(&[0u8; 400 * 400 * 4], 0).unwrap();
        encoder
            .add_frame_with_config(&[0u8; 400 * 400 * 4], 100, &EncodingConfig::default())
            .unwrap();

        let buf = encoder.finalize(200).unwrap();

        let decoder = Decoder::new(&buf).unwrap();
        let frames: Vec<_> = decoder.into_iter().collect();
        assert_eq!(frames[0].dimensions(), (400, 400));
        assert_eq!(frames[0].data(), &[0u8; 400 * 400 * 4]);
    }

    #[test]
    fn test_failures() {
        let mut encoder = Encoder::new((400, 400)).unwrap();
        assert_eq!(
            encoder.add_frame(&[0u8; 450 * 450 * 4], 0).unwrap_err(),
            Error::BufferSizeFailed(640_000, 810_000)
        );

        assert_eq!(
            encoder.add_frame(&[0u8; 50 * 50 * 4], 0).unwrap_err(),
            Error::BufferSizeFailed(640_000, 10_000)
        );

        encoder.add_frame(&[0u8; 400 * 400 * 4], 0).unwrap();

        assert_eq!(
            encoder.add_frame(&[0u8; 400 * 400 * 4], -1).unwrap_err(),
            Error::TimestampMustBeHigherThanPrevious(-1, 0)
        );

        assert_eq!(
            encoder.add_frame(&[0u8; 400 * 400 * 4], 0).unwrap_err(),
            Error::TimestampMustBeHigherThanPrevious(0, 0)
        );

        encoder.add_frame(&[0u8; 400 * 400 * 4], 10).unwrap();

        assert_eq!(
            encoder.finalize(0).unwrap_err(),
            Error::TimestampMustBeEqualOrHigherThanPrevious(0, 10)
        );
    }

    #[test]
    fn test_wrong_encoding_config() {
        let mut encoder = Encoder::new((4, 4)).unwrap();
        assert!(encoder
            .add_frame_with_config(
                &[0u8; 4 * 4 * 4],
                0,
                &EncodingConfig {
                    quality: 100.,
                    ..Default::default()
                },
            )
            .is_ok());

        assert_eq!(
            encoder
                .add_frame_with_config(
                    &[0u8; 4 * 4 * 4],
                    5,
                    &EncodingConfig {
                        quality: 101.,
                        ..Default::default()
                    },
                )
                .unwrap_err(),
            Error::InvalidEncodingConfig
        );
    }

    #[test]
    fn test_wrong_lossy_config() {
        assert_eq!(
            add_lossy_frame(LossyEncodingConfig {
                segments: 9999,
                ..Default::default()
            })
            .unwrap_err(),
            Error::InvalidEncodingConfig
        );

        assert_eq!(
            add_lossy_frame(LossyEncodingConfig {
                pass: 11,
                ..Default::default()
            })
            .unwrap_err(),
            Error::InvalidEncodingConfig
        );

        assert_eq!(
            add_lossy_frame(LossyEncodingConfig {
                filter_sharpness: 8,
                ..Default::default()
            })
            .unwrap_err(),
            Error::InvalidEncodingConfig
        );

        assert!(add_lossy_frame(LossyEncodingConfig {
            filter_sharpness: 7,
            ..Default::default()
        })
        .is_ok());
    }

    fn add_lossy_frame(lossy_config: LossyEncodingConfig) -> Result<(), Error> {
        let mut encoder = Encoder::new((4, 4)).unwrap();
        encoder.add_frame_with_config(
            &[0u8; 4 * 4 * 4],
            0,
            &EncodingConfig {
                encoding_type: EncodingType::Lossy(lossy_config),
                ..Default::default()
            },
        )
    }
}
