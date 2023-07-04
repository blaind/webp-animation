use image::RgbaImage;
use log::info;
use std::fs;

use imageproc::{drawing, rect::Rect};
use webp_animation::{AnimParams, Encoder, EncoderOptions};

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let (width, height) = (480, 480);
    let (frames, total_time_ms) = (30, 1000);

    let mut encoder = Encoder::new_with_options(
        (width, height),
        EncoderOptions {
            anim_params: AnimParams {
                loop_count: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();

    info!("Encoder initialized");

    let mut frame = RgbaImage::new(width, height);
    let dark = image::Rgba([0, 0, 0, 255]);
    let white = image::Rgba([255, 255, 255, 255]);

    let frame_ms = (total_time_ms as f32 / frames as f32) as i32;
    for i in 0..frames {
        info!("\tencode frame {}", i);

        let pos = ((i as f32 * width as f32) / frames as f32) as i32;

        drawing::draw_filled_rect_mut(&mut frame, Rect::at(0, 0).of_size(width, height), dark);
        drawing::draw_filled_rect_mut(&mut frame, Rect::at(pos, pos).of_size(20, 20), white);

        encoder.add_frame(frame.as_raw(), i * frame_ms).unwrap();
    }

    let final_timestamp = frames * frame_ms;

    let webp_data = encoder.finalize(final_timestamp).unwrap();
    let output = "data/loop_twice.webp";
    fs::write(output, webp_data).unwrap();

    info!("Saved to {}", output);
}
