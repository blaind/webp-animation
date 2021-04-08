use webp_animation::Decoder;

fn main() {
    env_logger::init();

    let buffer = std::fs::read("./data/animated.webp").unwrap();
    let decoder = Decoder::new(&buffer).unwrap();

    for frame in decoder.into_iter() {
        assert_eq!(frame.dimensions(), (400, 400));
        assert_eq!(frame.data().len(), 400 * 400 * 4); // w * h * rgba
    }
}
