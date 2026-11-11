# Local dependency patches

## proc-macro-error2 2.0.1

Source: the crates.io `proc-macro-error2` 2.0.1 package, as pinned previously in
`Cargo.lock` (upstream: https://github.com/GnomedDev/proc-macro-error-2).
Original source, tests, manifests, documentation, and MIT/Apache-2.0 licenses
are retained in `proc-macro-error2/`.

Local change: make `extern crate proc_macro` public in `src/lib.rs` so that
its existing public re-export in `__export` is valid. This fixes the Rust
`pub_use_of_private_extern_crate` future-incompatibility warning (E0365),
without suppressing warnings or changing macro behavior.

The root `[patch.crates-io]` applies this fix to GPUI's transitive
`stacksafe-macro` dependency in local builds and GitHub Actions alike.
Remove this patch when the dependency chain no longer needs the affected crate.
