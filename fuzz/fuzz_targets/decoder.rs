#![no_main]
use libfuzzer_sys::fuzz_target;
use webp_animation::Decoder;

fuzz_target!(|data: &[u8]| {
    let decoder = match Decoder::new(&data, Default::default()) {
        Ok(dec) => dec,
        Err(_) => {
            return;
        }
    };

    let _frames: Vec<_> = decoder.into_iter().collect();
});
