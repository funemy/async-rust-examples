# /bin/bash
# cargo +nightly rustc --bin hello_world -- --emit mir -Z unpretty=mir-cfg
# cargo +nightly rustc --lib -- --emit mir -Z unpretty=mir
cargo +nightly rustc --lib -- --emit mir -Z unpretty=mir-cfg
