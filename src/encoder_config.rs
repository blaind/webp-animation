use std::mem;

use crate::{ColorMode, Error};

#[allow(unused_imports)]
use crate::Encoder; // needed by docs

use libwebp_sys as webp;

/// An options struct for [`Encoder`] instance
///
/// See also [`EncodingConfig`] for frame encoding configuration. Can be set globally
/// or per-frame.
#[derive(Clone)]
pub struct EncoderOptions {
    /// Animation parameters
    pub anim_params: AnimParams,

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
    pub encoding_config: Option<EncodingConfig>,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            anim_params: AnimParams::default(),
            minimize_size: false,
            kmin: 0,
            kmax: 0,
            allow_mixed: false,
            verbose: false,
            color_mode: ColorMode::Rgba,
            encoding_config: None,
        }
    }
}

/// Animation parameters
#[derive(Clone, Default)]
pub struct AnimParams {
    /// Number of times to repeat the animation [0 = infinite, default].
    pub loop_count: i32,
}

/// Encoding type
#[derive(Debug, Clone)]
pub enum EncodingType {
    /// Lossy encoding
    Lossy(LossyEncodingConfig),

    /// Losless encoding. Default.
    Lossless,
}

impl EncodingType {
    pub fn new_lossy() -> Self {
        EncodingType::Lossy(LossyEncodingConfig::default())
    }
}

/// Encoding configuration. Can be set for [`Encoder`] globally or per frame
///
/// Set globally as part of [`EncoderOptions`] when using [`Encoder::new_with_options`],
/// or per frame through [`Encoder::add_frame_with_config`]
#[derive(Debug, Clone)]
pub struct EncodingConfig {
    /// Encoding Type (lossless or lossy). Defaults to lossless
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
    pub fn new_lossy(quality: f32) -> Self {
        Self {
            encoding_type: EncodingType::new_lossy(),
            quality,
            ..Default::default()
        }
    }

    pub(crate) fn to_config_container(&self) -> Result<ConfigContainer, Error> {
        ConfigContainer::new(self)
    }

    pub(crate) fn apply_to(&self, webp_config: &mut webp::WebPConfig) {
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

/// Parameters related to lossy compression only
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
    pub filter_type: usize,

    /// Auto adjust filter's strength [false = off, true = on]
    pub autofilter: bool,

    /// Algorithm for encoding the alpha plane (false = none,
    /// true = compressed with WebP lossless). Default is true.
    pub alpha_compression: bool,

    /// Predictive filtering method for alpha plane.
    /// 0: none, 1: fast, 2: best. Default if 1.
    pub alpha_filtering: usize,

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
            filter_type: 1,
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
    pub fn new_from_default_preset() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_from_picture_preset() -> Self {
        Self {
            sns_strength: 80,
            filter_sharpness: 4,
            filter_strength: 35,
            preprocessing: false,
            ..Default::default()
        }
    }

    pub fn new_from_photo_preset() -> Self {
        Self {
            sns_strength: 80,
            filter_sharpness: 3,
            filter_strength: 30,
            preprocessing: false,
            ..Default::default()
        }
    }

    pub fn new_from_drawing_preset() -> Self {
        Self {
            sns_strength: 25,
            filter_sharpness: 6,
            filter_strength: 10,
            ..Default::default()
        }
    }

    pub fn new_from_icon_preset() -> Self {
        Self {
            sns_strength: 0,
            filter_strength: 0,
            preprocessing: false,
            ..Default::default()
        }
    }

    pub fn new_from_text_preset() -> Self {
        Self {
            sns_strength: 0,
            filter_strength: 0,
            preprocessing: false,
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
}

pub(crate) struct ConfigContainer {
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

#[cfg(test)]
mod tests {
    use super::*;

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
