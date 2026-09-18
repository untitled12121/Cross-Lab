# M10 UniFFI dependency note

M10 Task 7 uses UniFFI as the narrow Rust-to-mobile binding generator for the Android façade.

## Verified dependency

- Production dependency: `uniffi 0.32.1`.
- License: MPL-2.0.
- Verified release date: 2026-09-09.
- The uploaded research repository is an earlier `0.32.0` source snapshot and was inspected for proc-macro object/record/enum/error patterns.
- Cross-Lab depends on the published crate and does not copy or modify UniFFI source.

The runtime dependency disables UniFFI default features. Proc-macro scaffolding is available directly in the published `0.32.1` crate; there is no published `macro-scaffolding` feature. The local `bindgen` feature enables UniFFI's CLI and Cargo-metadata support only while generating foreign bindings.

## Integration choice

Cross-Lab uses proc-macro declarations plus `uniffi::setup_scaffolding!()` so the Rust API remains the single interface definition. The exported surface is intentionally limited to owned mobile DTOs, typed lifecycle errors, one lifecycle object, and bounded snapshot delivery.

UniFFI-generated C-ABI scaffolding necessarily contains unsafe code. `crates/mobile-ffi` is therefore the narrow FFI exception to the workspace's handwritten-unsafe policy; Cross-Lab source in the crate must not add handwritten unsafe blocks or unsafe functions.

Kotlin bindings are generated with:

`bash scripts/generate-mobile-bindings.sh [output-directory]`

The script first builds the host `cdylib`, then asks the workspace-locked UniFFI CLI to generate Kotlin from the library's embedded metadata. This follows the maintained UniFFI library-mode path rather than the experimental source-parser path. Generated Kotlin is build output and is not hand-maintained in multiple repository locations.
