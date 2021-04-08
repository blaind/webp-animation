use std::{mem, pin::Pin, ptr};

use libwebp_sys as webp;

use crate::{ColorMode, Error, WebPData, PIXEL_BYTES};

/// An options struct for [`Encoder`] instance
///
/// See also [`EncodingConfig`] for frame encoding configuration. Can be set globally
/// or per-frame.
pub struct EncoderOptions {
    /// If true, minimize the output size (slow). Implicitly
    /// disables key-frame insertion. Default `false`
    pub minimize_size: bool,

    /// Minimum and maximum distance between consecutive key
    /// frames in the output. The library may insert some key
    /// frames as needed to satisfy this criteria.
    /// Note that these conditions should hold: `kmax > kmin`
    /// and `kmin >= kmax / 2 + 1`. Also, if `kmax <= 0`, then
    /// key-frame insertion is disabled; and if `kmax == 1`,
    /// then all frames will be key-frames (kmin value does
    /// not matter for these special cases). Defaults to zero
    pub kmin: isize,
    pub kmax: isize,

    /// If true, use mixed compression mode; may choose
    /// either lossy and lossless for each frame. Default `false`
    pub allow_mixed: bool,

    /// If true, print info and warning messages to stderr. Default `false`
    pub verbose: bool,

    /// Input colorspace. [`ColorMode::Rgba`] by default
    pub color_mode: ColorMode,

    /// Default per-frame encoding config, optional. Can also be added per-frame
    /// by [`Encoder::add_frame_with_config`]
    pub default_config: Option<EncodingConfig>,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            minimize_size: false,
            kmin: 0,
            kmax: 0,
            allow_mixed: false,
            verbose: false,
            color_mode: ColorMode::Rgba,
            default_config: None,
        }
    }
}

/// An encoder for creating webp animation
///
/// Will take `n` frames as an input. WebP binary data is output at the end
/// (wrapped into [`WebPData`] which acts as a `&[u8]`)
///
/// ```rust
/// use webp_animation::{Encoder, Frame};
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
///   let frame_timestamp = i * 170;
///
///   encoder.add_frame(rgba_data, frame_timestamp).unwrap();
/// }
///
/// // get encoded webp data
/// let final_timestamp = 1_000;
/// let webp_data = encoder.finalize(final_timestamp).unwrap();
/// // std::fs::write("my_animation.webp", webp_data);
/// ```

pub struct Encoder {
    encoder_wr: EncoderWrapper,
    current_frame: usize,
    frame: PictureWrapper,
    options: EncoderOptions,
}

impl Encoder {
    pub fn new(dimensions: (u32, u32)) -> Result<Self, Error> {
        Encoder::new_with_options(dimensions, Default::default())
    }

    pub fn new_with_options(
        dimensions: (u32, u32),
        options: EncoderOptions,
    ) -> Result<Self, Error> {
        let enc_options = convert_options(&options)?;
        let encoder_wr = EncoderWrapper::new(dimensions, enc_options)?;

        Ok(Self {
            encoder_wr,
            current_frame: 0,
            options,
            frame: PictureWrapper::new(dimensions)?,
        })
    }

    pub fn add_frame_with_config(
        &mut self,
        data: &[u8],
        timestamp: i32,
        config: &EncodingConfig,
    ) -> Result<(), Error> {
        self.add_frame_internal(data, timestamp, Some(config))
    }

    pub fn add_frame(&mut self, data: &[u8], timestamp: i32) -> Result<(), Error> {
        self.add_frame_internal(data, timestamp, None)
    }

    fn add_frame_internal(
        &mut self,
        data: &[u8],
        timestamp: i32,
        config: Option<&EncodingConfig>,
    ) -> Result<(), Error> {
        self.frame.set_data(data, self.options.color_mode)?;

        if unsafe {
            webp::WebPAnimEncoderAdd(
                self.encoder_wr.encoder,
                self.frame.as_webp_picture_ref(),
                timestamp,
                match config {
                    Some(config) => config.as_ptr(),
                    None => match &self.options.default_config {
                        Some(config) => config.as_ptr(),
                        None => std::ptr::null(),
                    },
                },
            )
        } == 0
        {
            return Err(Error::EncoderAddFailed);
        }

        self.current_frame += 1;

        Ok(())
    }

    pub fn set_default_encoding_config(&mut self, config: EncodingConfig) {
        self.options.default_config = Some(config);
    }

    pub fn finalize(self, timestamp: i32) -> Result<WebPData, Error> {
        // FIXME check that timestamp > prev frame timestamp
        if unsafe {
            webp::WebPAnimEncoderAdd(
                self.encoder_wr.encoder,
                ptr::null_mut(),
                timestamp,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(Error::EncoderAddFailed);
        }

        let mut data = WebPData::new();

        unsafe {
            assert!(webp::WebPAnimEncoderAssemble(self.encoder_wr.encoder, data.inner_ref()) != 0);
            // FIXME add error handling instead of assert
        }

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

    enc_options.minimize_size = if options.minimize_size { 1 } else { 0 };
    enc_options.kmin = options.kmin as i32;
    enc_options.kmax = options.kmax as i32;
    enc_options.allow_mixed = if options.allow_mixed { 1 } else { 0 };
    enc_options.verbose = if options.verbose { 1 } else { 0 };

    Ok(enc_options)
}

pub struct EncodingConfig {
    config: webp::WebPConfig,
}

impl EncodingConfig {
    pub fn new() -> Self {
        let config = unsafe {
            let mut config = mem::zeroed();
            webp::WebPConfigInit(&mut config);
            config.lossless = 1;
            //config.quality = 85.;

            assert!(webp::WebPValidateConfig(&config) != 0);
            config
        };

        Self { config }
    }

    pub fn as_ptr(&self) -> &webp::WebPConfig {
        &self.config
    }
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
    use crate::{Decoder, Frame};
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
            .add_frame_with_config(&[0u8; 400 * 400 * 4], 100, &EncodingConfig::new())
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
    }

    // TEST failure on decreasing timestamp
}
