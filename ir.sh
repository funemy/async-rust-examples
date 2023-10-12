# /bin/bash
cargo +nightly rustc --bin timer -- --emit mir -Z unpretty=mir-cfg > mir.out
