A graphical interface for viewing the current pipewire graph (name pending, suggestions welcome), inspired by the JACK tool [catia](https://kx.studio/Applications:Catia).

![Screenshot](screenshot.png)

# Features planned

- Allow creation of links from one port to another.
- Color ports and links based on whether they carry audio, video or midi data.

More suggestions are welcome!

# Building
For compilation, you will need:

- An up-to-date rust toolchain
- gtk-4.0 and pipewire-0.3 development headers

To compile, run

    $ cargo build --release

in the repository root.
The resulting binary will be at `target/release/graphui`.