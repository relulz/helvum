# Making a release

The following describes the process of making a new release:

1. In `data/org.freedesktop.ryuukyu.Helvum.metainfo.xml.in`,
   add a new `<release>` tag to the releases section with the appropriate version and date.

2. In `meson.build` and `Cargo.toml`, bumb the projects version to the new version.

3. Ensure cargo dependencies are up-to-date by running `cargo outdated` (may require running `cargo install cargo-outdated`) and updating outdated dependencies (including the versions specified in `Cargo.lock`).

4. Commit the changes with the a message of the format "Release x.y.z"

5. Add a tag to the release with the new version and a description from describing the changes as a message (run `git tag -a x.y.z`, then write the message)

6. Make a **new** meson build directory and run `meson dist`.
   Two files should be created in a `meson-dist` subdirectory:

   `helvum-x.y.z.tar.xz` and 
   `helvum-x.y.z.tar.xz.sha256sum`

7. Push the new commit and tag to upstream, then create a new release on gitlab from the new tag, the description from the tags message formatted as markdown, and also add the two files from step 6 to the description.
