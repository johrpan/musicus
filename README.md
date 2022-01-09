# Musicus

This is a desktop app for Musicus.

## Hacking

### Building

Musicus uses the [Meson build system](https://mesonbuild.com/). You can build
it using the following commands:

```
$ meson build
$ ninja -C build
```

Afterwards the resulting binary executable is under
`build/target/debug/musicus`.

### Flatpak

There is a Flatpak manifest file called `de.johrpan.musicus.json`. To build a
Flatpak you need the the latest Gnome SDK and the Freedesktop SDK with the Rust
extension. You can install those using the following commands:

```
$ flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
$ flatpak remote-add --user --if-not-exists gnome-nightly https://nightly.gnome.org/gnome-nightly.flatpakrepo
$ flatpak install --user gnome-nightly org.gnome.Sdk org.gnome.Platform
$ flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//19.08
```

Afterwards, the following commands will build, install and run the application:

```
$ rm -rf flatpak
$ flatpak-builder --user --install flatpak de.johrpan.musicus.json
$ flatpak run de.johrpan.musicus
```

### Special requirements

This program uses [Diesel](https://diesel.rs) as its ORM. After installing
the Diesel command line utility, you will be able to create a new schema
migration using the following command:

```
$ diesel migration generate [change_description]
```

To update the `src/database/schema.rs` file, you should use the following
command:

```
$ diesel migration run --database-url test.sqlite
```

This file should never be edited manually.

## License

Musicus is free and open source software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by the
Free Software Foundation, either version 3 of the License, or (at your option)
any later version.

Musicus is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
details.

You should have received a copy of the GNU Affero General Public License along
with this program. If not, see https://www.gnu.org/licenses/.
