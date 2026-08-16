# Third-Party Notices

This file lists third-party software included with OpenFX Network Video Plugin.
Crate versions are pinned by `Cargo.lock`.

The NDI® runtime DLL is not a crate. See `NDI_TERMS.txt` and the unmodified `Processing.NDI.Lib.Licenses.txt` bundled with the release package. NDI® is a registered trademark of Vizrt NDI AB. https://ndi.video/

## OpenFX headers

Vendored from [AcademySoftwareFoundation/openfx](https://github.com/AcademySoftwareFoundation/openfx)
commit `3de640d6f645fe6e346acd57e568d8b0a5ae4574`.

BSD 3-Clause License. The full text is in `crates/openfx/vendor/LICENSE.md`.

```text
Copyright (c) 2025, OpenFX and contributors to the OpenFX project
SPDX-License-Identifier: BSD-3-Clause
```

## grafton-ndi

`grafton-ndi` 1.0.0 (`default-features = false`). Apache License 2.0.
Copyright Grant Sparks / grafton-ndi contributors.
https://github.com/GrantSparks/grafton-ndi

## Design references (no source copied)

Resolve host compatibility notes (Filter + General, Create/Destroy, tiles off, avoiding `IsIdentity`) follow the public behavior of [ntsc-rs](https://github.com/valadaptive/ntsc-rs). ntsc-rs is GPL-3.0-or-later; this plugin does not include its source.

RAII patterns for OpenFX suites, images, and instance data were informed by [kreantio/openfx-rs](https://github.com/kreantio/openfx-rs) examples.

NDI runtime discovery and delay-load packaging follow `aviutl2-ndi-output`.

## Remaining crates

See `Cargo.lock` for the complete, version-pinned dependency graph. Runtime and build crates include bindgen, clang-sys, grafton-ndi, windows-sys, and the OpenFX helper crate in this workspace.

License texts for crates.io packages can be regenerated with `cargo about generate` when `cargo-about` is available.
