# /bin/bash
# cargo +nightly rustc --bin hello_world -- --emit mir -Z unpretty=mir-cfg
cargo +nightly rustc --bin hello_world -- --emit mir -Z unpretty=mir
