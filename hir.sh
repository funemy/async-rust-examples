# /bin/bash
# This generate HIR for the specified bin that is somewhat readable (similar enough to Rust)
# `unpretty` is one of:
# `normal`, `identified`, `expanded`, `expanded,identified`, `expanded,hygiene`, `ast-tree`, `ast-tree,expanded`, `hir`, `hir,identified`, `hir,typed`, `hir-tree`, `thir-tree`, `thir-flat`, `mir` or `mir-cfg`
# cargo rustc --lib $1 -- -Z unpretty=hir,typed
cargo rustc --bin $1 -- -Z unpretty=hir,typed
# if you'd like a tree structure
# cargo +nightly rustc --bin hello_world -- -Z unpretty=hir-tree
