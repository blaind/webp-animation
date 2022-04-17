use log::{info, warn};
use webp_animation::{Decoder, Encoder};

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    print_err(0, Encoder::new((0, 0)));
    let mut encoder = Encoder::new((1, 1)).unwrap();
    print_err(1, encoder.add_frame(&[], 0));
    print_err(2, encoder.add_frame(&[0u8; 4], -5));
    print_err(3, encoder.finalize(10));

    print_err(4, Decoder::new(&[]));
    print_err(5, Decoder::new(&[0x00, 0x01]));
    print_err(
        6,
        Decoder::new(&[
            0x2f, 0xff, 0xff, 0xff, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]),
    );

    #[cfg(feature = "image")]
    {
        let buffer = std::fs::read("./data/animated.webp").unwrap();
        let decoder = Decoder::new(&buffer).unwrap();
        let frame = decoder.into_iter().next().unwrap();
        print_err(6, frame.into_bgra_image());
    }
}

fn print_err<A, B>(num: usize, result: Result<A, B>)
where
    B: std::fmt::Debug,
{
    match result {
        Ok(_) => info!("Result {}: returned OK", num),
        Err(e) => {
            warn!("Result {}: {:?}", num, e);
        }
    }
}
