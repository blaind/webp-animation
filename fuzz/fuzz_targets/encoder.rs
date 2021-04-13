#![no_main]
use libfuzzer_sys::fuzz_target;
use webp_animation::Encoder;

fuzz_target!(|data: &[u8]| {
    let mut encoder = Encoder::new((64, 64)).unwrap();
    if let Ok(_) = encoder.add_frame(data, 0) {
        let _ = encoder.finalize(100);
    }
});
