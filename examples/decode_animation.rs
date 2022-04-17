use log::info;
use webp_animation::Decoder;

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let buffer = std::fs::read("./data/animated.webp").unwrap();
    let decoder = Decoder::new(&buffer).unwrap();

    for frame in decoder.into_iter() {
        assert_eq!(frame.dimensions(), (400, 400));
        assert_eq!(frame.data().len(), 400 * 400 * 4); // w * h * rgba

        #[cfg(features = "image")]
        assert_eq!(frame.into_image().unwrap().dimensions(), (400, 400));

        info!(
            "Frame, dimensions={:?}, data_len={}",
            frame.dimensions(),
            frame.data().len()
        );
    }
}
