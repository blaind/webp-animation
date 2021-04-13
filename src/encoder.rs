use std::{mem, pin::Pin, ptr};

use libwebp_sys as webp;

use crate::{ColorMode, Error, WebPData, PIXEL_BYTES};

/// An options struct for [`Encoder`] instance
///
/// See also [`EncodingConfig`] for frame encoding configuration. Can be set globally
/// or per-frame.
#[derive(Clone)]
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
    pub encoding: Option<EncodingConfig>,
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
            encoding: None,
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
    frame: PictureWrapper,
    options: EncoderOptions,
    previous_timestamp: i32,
    encoding_config: Option<ConfigContainer>,
}

impl Encoder {
    pub fn new(dimensions: (u32, u32)) -> Result<Self, Error> {
        Encoder::new_with_options(dimensions, Default::default())
    }

    pub fn new_with_options(
        dimensions: (u32, u32),
        options: EncoderOptions,
    ) -> Result<Self, Error> {
        if dimensions.0 <= 0 || dimensions.1 <= 0 {
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

        if let Some(config) = options.encoding {
            encoder.set_default_encoding_config(config)?;
        }

        Ok(encoder)
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

        log::trace!("Add frame at timestamp {}, {} bytes", timestamp, data.len());

        Ok(())
    }

    pub fn set_default_encoding_config(&mut self, config: EncodingConfig) -> Result<(), Error> {
        self.encoding_config = Some(config.to_config_container()?);
        self.options.encoding = Some(config);
        Ok(())
    }

    pub fn finalize(self, timestamp: i32) -> Result<WebPData, Error> {
        if self.previous_timestamp == -1 {
            // -1 = no frames added
            return Err(Error::NoFramesAdded);
        }

        if timestamp < self.previous_timestamp {
            return Err(Error::TimestampMustBeEqualOrHigherThanPrevious(
                timestamp,
                self.previous_timestamp,
            ));
        }

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

        if unsafe { webp::WebPAnimEncoderAssemble(self.encoder_wr.encoder, data.inner_ref()) } == 0
        {
            return Err(Error::EncoderAssmebleFailed);
        }

        log::trace!(
            "Finalize encoding at timestamp {}, output binary size {} bytes",
            timestamp,
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

    enc_options.minimize_size = if options.minimize_size { 1 } else { 0 };
    enc_options.kmin = options.kmin as i32;
    enc_options.kmax = options.kmax as i32;
    enc_options.allow_mixed = if options.allow_mixed { 1 } else { 0 };
    enc_options.verbose = if options.verbose { 1 } else { 0 };

    Ok(enc_options)
}

#[derive(Debug, Clone)]
pub enum EncodingType {
    Lossy(LossyEncodingConfig),
    Lossless,
}

#[derive(Debug, Clone)]
pub struct EncodingConfig {
    /// Encoding Type (lossless or lossy). Default lossless
    pub encoding_type: EncodingType,

    /// Between 0 and 100. For lossy, 0 gives the smallest
    /// size and 100 the largest. For lossless, this
    /// parameter is the amount of effort put into the
    /// compression: 0 is the fastest but gives larger
    /// files compared to the slowest, but best, 100.
    pub quality: f32,

    /// Quality/speed trade-off (0=fast, 6=slower-better)
    pub method: usize,
    // image_hint todo?
}

impl EncodingConfig {
    fn to_config_container(&self) -> Result<ConfigContainer, Error> {
        ConfigContainer::new(self)
    }

    fn apply_to(&self, webp_config: &mut webp::WebPConfig) {
        webp_config.lossless = match &self.encoding_type {
            EncodingType::Lossy(lossless_config) => {
                lossless_config.apply_to(webp_config);
                0
            }
            EncodingType::Lossless => 1,
        };
        webp_config.quality = self.quality;
    }
}

impl Default for EncodingConfig {
    fn default() -> Self {
        // src/enc/config_enc.c has defaults
        Self {
            encoding_type: EncodingType::Lossless,
            quality: 1.,
            method: 4,
        }
    }
}

/// Parameters related to lossy compression only:
#[derive(Debug, Clone)]
pub struct LossyEncodingConfig {
    /// if non-zero, set the desired target size in bytes.
    /// Takes precedence over the 'compression' parameter.
    pub target_size: usize,

    /// if non-zero, specifies the minimal distortion to
    /// try to achieve. Takes precedence over target_size.
    pub target_psnr: f32,

    /// maximum number of segments to use, in [1..4]
    pub segments: usize,

    /// Spatial Noise Shaping. 0=off, 100=maximum.
    pub sns_strength: usize,

    /// range: [0 = off .. 100 = strongest]
    pub filter_strength: usize,

    /// range: [0 = off .. 7 = least sharp]
    pub filter_sharpness: usize,

    /// filtering type: 0 = simple, 1 = strong (only used
    /// if filter_strength > 0 or autofilter > 0)
    pub filter_type: bool,

    /// Auto adjust filter's strength [0 = off, 1 = on]
    pub autofilter: bool,

    /// Algorithm for encoding the alpha plane (0 = none,
    /// 1 = compressed with WebP lossless). Default is 1.
    pub alpha_compression: bool,

    /// Predictive filtering method for alpha plane.
    /// 0: none, 1: fast, 2: best. Default if 1.
    pub alpha_filtering: isize, // TODO enum

    /// Between 0 (smallest size) and 100 (lossless).
    /// Default is 100.
    pub alpha_quality: usize,

    /// number of entropy-analysis passes (in [1..10]).
    pub pass: usize,

    /// if true, export the compressed picture back.
    /// In-loop filtering is not applied.
    pub show_compressed: bool,

    /// preprocessing filter (0=none, 1=segment-smooth)
    pub preprocessing: bool,

    /// log2(number of token partitions) in [0..3]
    /// Default is set to 0 for easier progressive decoding.
    pub partitions: usize,

    /// quality degradation allowed to fit the 512k limit on
    /// prediction modes coding (0: no degradation,
    /// 100: maximum possible degradation).
    pub partition_limit: isize,

    /// if needed, use sharp (and slow) RGB->YUV conversion
    pub use_sharp_yuv: bool,
}

impl Default for LossyEncodingConfig {
    fn default() -> Self {
        Self {
            // src/enc/config_enc.c contains defaults
            target_size: 0,
            target_psnr: 0.,
            segments: 1,
            sns_strength: 50,
            filter_strength: 60,
            filter_sharpness: 0,
            filter_type: true,
            partitions: 0,
            pass: 1,
            show_compressed: false,
            autofilter: false,
            alpha_compression: true,
            alpha_filtering: 1,
            alpha_quality: 100,
            preprocessing: false,
            partition_limit: 0,
            use_sharp_yuv: false,
        }
    }
}

impl LossyEncodingConfig {
    pub fn new_preset_default() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_preset_picture() -> Self {
        Self {
            sns_strength: 80,
            filter_sharpness: 4,
            filter_strength: 35,
            // preprocessing: 2, FIXME
            ..Default::default()
        }
    }

    pub fn new_preset_photo() -> Self {
        Self {
            sns_strength: 80,
            filter_sharpness: 3,
            filter_strength: 30,
            // preprocessing: 2, FIXME
            ..Default::default()
        }
    }

    pub fn new_preset_drawing() -> Self {
        Self {
            sns_strength: 25,
            filter_sharpness: 6,
            filter_strength: 10,
            ..Default::default()
        }
    }

    pub fn new_preset_icon() -> Self {
        Self {
            sns_strength: 0,
            filter_strength: 0,
            // preprocessing: 2, FIXME
            ..Default::default()
        }
    }

    pub fn new_preset_text() -> Self {
        Self {
            sns_strength: 0,
            filter_strength: 0,
            // preprocessing: 2, FIXME
            segments: 2,
            ..Default::default()
        }
    }

    fn apply_to(&self, webp_config: &mut webp::WebPConfig) {
        webp_config.target_size = self.target_size as i32;
        webp_config.target_PSNR = self.target_psnr;
        webp_config.segments = self.segments as i32;
        webp_config.sns_strength = self.sns_strength as i32;
        webp_config.filter_strength = self.filter_strength as i32;
        webp_config.filter_sharpness = self.filter_sharpness as i32;
        webp_config.filter_type = self.filter_type as i32;
        webp_config.autofilter = self.autofilter as i32;
        webp_config.alpha_compression = self.alpha_compression as i32;
        webp_config.alpha_filtering = self.alpha_filtering as i32;
        webp_config.alpha_quality = self.alpha_quality as i32;
        webp_config.pass = self.pass as i32;
        webp_config.show_compressed = self.show_compressed as i32;
        webp_config.preprocessing = self.preprocessing as i32;
        webp_config.partitions = self.partitions as i32;
        webp_config.partition_limit = self.partition_limit as i32;
        webp_config.use_sharp_yuv = self.use_sharp_yuv as i32;
    }

    // FIXME could implement presets
    // (WEBP_PRESET_PICTURE, WEBP_PRESET_PHOTO, WEBP_PRESET_DRAWING, WEBP_PRESET_ICON, WEBP_PRESET_TEXT)
}

struct ConfigContainer {
    config: webp::WebPConfig,
}

impl ConfigContainer {
    pub fn new(config: &EncodingConfig) -> Result<Self, Error> {
        let mut webp_config = unsafe {
            let mut config = mem::zeroed();
            webp::WebPConfigInit(&mut config);
            config
        };

        config.apply_to(&mut webp_config);

        if unsafe { webp::WebPValidateConfig(&webp_config) } == 0 {
            return Err(Error::InvalidEncodingConfig);
        }

        Ok(Self {
            config: webp_config,
        })
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

    #[test]
    fn test_config_defaults() {
        let default_webp_config = unsafe {
            let mut config = mem::zeroed();
            webp::WebPConfigInit(&mut config);
            config
        };

        let config = ConfigContainer::new(&EncodingConfig::default()).unwrap();

        let left = config.as_ptr();
        let def = &default_webp_config;

        // custom-set
        assert_eq!(left.lossless, 1);
        assert_eq!(left.quality, 1.0);

        // matches libwebp
        assert_eq!(left.method, def.method, "c.method");
        assert_eq!(left.image_hint, def.image_hint, "c.image_hint");
        assert_eq!(left.target_size, def.target_size, "c.target_size");
        assert_eq!(left.target_PSNR, def.target_PSNR, "c.target_PSNR");
        assert_eq!(left.segments, def.segments, "c.segments");
        assert_eq!(left.sns_strength, def.sns_strength, "c.sns_strength");
        assert_eq!(
            left.filter_strength, def.filter_strength,
            "c.filter_strength"
        );
        assert_eq!(
            left.filter_sharpness, def.filter_sharpness,
            "c.filter_sharpness"
        );
        assert_eq!(left.filter_type, def.filter_type, "c.filter_type");
        assert_eq!(left.autofilter, def.autofilter, "c.autofilter");
        assert_eq!(
            left.alpha_compression, def.alpha_compression,
            "c.alpha_compression"
        );
        assert_eq!(
            left.alpha_filtering, def.alpha_filtering,
            "c.alpha_filtering"
        );
        assert_eq!(left.alpha_quality, def.alpha_quality, "c.alpha_quality");
        assert_eq!(left.pass, def.pass, "c.pass");
        assert_eq!(
            left.show_compressed, def.show_compressed,
            "c.show_compressed"
        );
        assert_eq!(left.preprocessing, def.preprocessing, "c.preprocessing");
        assert_eq!(left.partitions, def.partitions, "c.partitions");
        assert_eq!(
            left.partition_limit, def.partition_limit,
            "c.partition_limit"
        );
        assert_eq!(
            left.emulate_jpeg_size, def.emulate_jpeg_size,
            "c.emulate_jpeg_size"
        );
        assert_eq!(left.thread_level, def.thread_level, "c.thread_level");
        assert_eq!(left.low_memory, def.low_memory, "c.low_memory");

        assert_eq!(left.near_lossless, def.near_lossless, "c.near_lossless");
        assert_eq!(left.exact, def.exact, "c.exact");
        assert_eq!(
            left.use_delta_palette, def.use_delta_palette,
            "c.use_delta_palette"
        );
        assert_eq!(left.use_sharp_yuv, def.use_sharp_yuv, "c.use_sharp_yuv");
        assert_eq!(left.qmin, def.qmin, "c.qmin");
        assert_eq!(left.qmax, def.qmax, "c.qmax");
    }
}
