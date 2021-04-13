# Fuzzing

## Decoder

```rust
$ cargo rustc --bin decoder -- -C passes='sancov' -C llvm-args='-sanitizer-coverage-level=3' -C llvm-args='-sanitizer-coverage-inline-8bit-counters' -Z sanitizer=address
$ ./target/debug/decoder
```

## Encoder

```rust
$ cargo rustc --bin encoder -- -C passes='sancov' -C llvm-args='-sanitizer-coverage-level=3' -C llvm-args='-sanitizer-coverage-inline-8bit-counters' -Z sanitizer=address
$ ./target/debug/encoder
```

