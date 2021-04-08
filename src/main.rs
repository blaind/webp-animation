use std::io::Read;

use webp_animation::{Decoder, Encoder};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args[2] == "encode" {
        // encode data
        for _ in 0..500 {
            let mut encoder = Encoder::new((400, 400)).unwrap();
            encoder.add_frame(&[255u8; 400 * 400 * 4], 50).unwrap();
            let _ = encoder.finalize(440).unwrap();
        }
    }

    let mut buffer = Vec::new();
    std::fs::File::open(&args[1])
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let decoder = Decoder::new(&buffer).unwrap();
    let (width, height) = decoder.dimensions();

    println!("Decoded webp file, canvas size {}x{}", width, height);

    for (i, frame) in decoder.into_iter().enumerate() {
        let output_filename = format!("image-{}.png", i);
        println!(
            "\tFrame {} at timestamp {}, store to {:?}",
            i,
            frame.timestamp(),
            output_filename
        );

        #[cfg(feature = "image")]
        image::save_buffer(
            output_filename,
            frame.data(),
            width,
            height,
            image::ColorType::Rgba8,
        )
        .unwrap()
    }
    println!("\tDecoding done");
}
