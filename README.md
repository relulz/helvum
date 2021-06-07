Helvum is a GTK-based patchbay for pipewire, inspired by the JACK tool [catia](https://kx.studio/Applications:Catia).

![Screenshot](screenshot.png)

[![Packaging status](https://repology.org/badge/vertical-allrepos/helvum.svg)](https://repology.org/project/helvum/versions)


# Features planned

- Volume control
- "Debug mode" that lets you view advanced information for nodes and ports

More suggestions are welcome!

# Building
For compilation, you will need:

- An up-to-date rust toolchain
- `libclang-3.7` or higher
- `gtk-4.0` and `pipewire-0.3` development headers

To compile, run

    $ cargo build --release

in the repository root.
The resulting binary will be at `target/release/helvum`.

# License
Helvum is distributed under the terms of the GPL3 license.
See LICENSE for more information.