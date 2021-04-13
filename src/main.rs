use std::io::Read;

use webp_animation::Decoder;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    println!("File: {}", args[1]);

    let mut buffer = Vec::new();
    std::fs::File::open(&args[1])
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let decoder = Decoder::new(&buffer).unwrap();
    let (width, height) = decoder.dimensions();

    println!("Decoded webp file, canvas size {}x{}", width, height);

    for (i, frame) in decoder.into_iter().enumerate() {
        #[cfg(not(feature = "image"))]
        {
            println!("\tFrame {} at timestamp {}", i, frame.timestamp(),);
        }

        #[cfg(feature = "image")]
        {
            let output_filename = format!("image-{}.png", i);
            println!(
                "\tFrame {} at timestamp {}, store to {:?}",
                i,
                frame.timestamp(),
                output_filename
            );

            image::save_buffer(
                output_filename,
                frame.data(),
                width,
                height,
                image::ColorType::Rgba8,
            )
            .unwrap()
        }
    }
    println!("\tDecoding done");
}
