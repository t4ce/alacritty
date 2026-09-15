# TRUEOS GL profile 0 inputs

This directory is the checked-in input root for the **Alacritty GLES2Pure
AOT** profile.  It is deliberately not a general GLES or GLSL asset format.

`bakery-input.json` pins the complete shader strings Alacritty would otherwise
pass to `ShaderSource`: the GLES2 header, the optional rectangle define, and
the source file.  The Bakery must verify those hashes and publish the seven
stage artifacts and five linked programs atomically beneath this directory.

The runtime program names and uniform locations are defined by the generated
manifest and mirrored by `src/renderer/aot.rs` until the Bakery owns that Rust
output.  A change to one of the GLSL sources must refresh this input file and
the corresponding Bakery artifacts together.
