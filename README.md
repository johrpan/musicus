![Musicus Logo](data/misc/logo.png)

# Musicus

## Introduction

Musicus is a classical music player and organizer designed for the
[GNOME](https://www.gnome.org)
platform. It helps you manage your personal collection of classical music
recordings. Musicus also comes with a pre-made sample library of public domain
recordings, which you can use as a starting point or to test the application.

The following features make Musicus special:

 - Combination of library management, search and playback
 - Metadata handling optimized for classical music
 - Intelligent random playback with customizable programs
 - Built-in sample library for just listening to music right-away
 - Local-first, no cloud or account required

Please note that Musicus will not be ready for everyone until version 1.0 is
released. Before then, the format of the music library may change, which could
result in permanent data loss. Do not use the Musicus library as your primary
music collection!

![Screenshot](data/misc/screenshot.png)

## Hacking

### ORM

This program uses [Diesel](https://diesel.rs) as its ORM. After installing
the Diesel command line utility, you will be able to create a new schema
migration using the following command:

```
$ diesel migration generate [change_description]
```

To update the `src/db/schema.rs` file, you should use the following command:

```
$ diesel migration run --database-url test.sqlite
```

This file should never be edited manually.

### Schema overview

This is an overview of the database schema used by Musicus. Every entity
also carries `created_at`, `edited_at`, `last_used_at`, `source` and
`enable_updates` bookkeeping columns, which are omitted here for clarity.

#### Works, composers, instruments

Works form a tree of work parts (`parent_work_id`) and can point at a related
work, e.g. an arrangement (`relates_to`). Composers, arrangers and the like
are credited on the work itself, each qualified by a role; instrumentation is
recorded separately.

```mermaid
erDiagram
    works {
        text work_id PK
        text parent_work_id FK
        text relates_to FK
        text name
    }

    persons {
        text person_id PK
        text name
    }

    roles {
        text role_id PK
        text name
    }

    instruments {
        text instrument_id PK
        text name
    }

    work_persons {
        text work_id FK
        text person_id FK
        text role_id FK
        int sequence_number
    }

    work_instruments {
        text work_id FK
        text instrument_id FK
        int sequence_number
    }

    works ||--o{ work_persons : composed
    persons ||--o{ work_persons : "credited on"
    roles ||--o{ work_persons : qualifies

    works ||--o{ work_instruments : "scored for"
    instruments ||--o{ work_instruments : "used in"
```

#### Recordings, performers, ensembles, instruments

Performers are credited on a recording independently of the work's own
composer credits, each qualified by a role and optionally an instrument.
Ensembles are credited the same way, and their own members are credited on
the ensemble itself.

```mermaid
erDiagram
    recordings {
        text recording_id PK
        text work_id FK
        text comment
    }

    persons {
        text person_id PK
        text name
    }

    roles {
        text role_id PK
        text name
    }

    instruments {
        text instrument_id PK
        text name
    }

    ensembles {
        text ensemble_id PK
        text name
    }

    recording_persons {
        text recording_id FK
        text person_id FK
        text role_id FK
        text instrument_id FK
        int sequence_number
    }

    recording_ensembles {
        text recording_id FK
        text ensemble_id FK
        text role_id FK
        int sequence_number
    }

    ensemble_persons {
        text ensemble_id FK
        text person_id FK
        text instrument_id FK
        text role_id FK
        int sequence_number
    }

    recordings ||--o{ recording_persons : credits
    persons ||--o{ recording_persons : "credited on"
    roles ||--o{ recording_persons : qualifies
    instruments ||--o{ recording_persons : "played on"

    recordings ||--o{ recording_ensembles : credits
    ensembles ||--o{ recording_ensembles : "credited on"
    roles ||--o{ recording_ensembles : qualifies

    ensembles ||--o{ ensemble_persons : "has member"
    persons ||--o{ ensemble_persons : "member of"
    instruments ||--o{ ensemble_persons : plays
    roles ||--o{ ensemble_persons : qualifies
```

#### Works, recordings, tracks, albums

A work is recorded as one or more recordings, each split into tracks. A track
can belong to more than one work, which happens when a recording runs
movements together without a break. Albums group recordings independently of
how their tracks are organized on disk.

```mermaid
erDiagram
    works {
        text work_id PK
        text name
    }

    recordings {
        text recording_id PK
        text work_id FK
        text comment
    }

    tracks {
        text track_id PK
        text recording_id FK
        text path
    }

    albums {
        text album_id PK
        text name
    }

    album_recordings {
        text album_id FK
        text recording_id FK
        int sequence_number
    }

    track_works {
        text track_id FK
        text work_id FK
        int sequence_number
    }

    works ||--o{ recordings : "recorded as"
    recordings ||--o{ tracks : "split into"
    albums ||--o{ album_recordings : contains
    recordings ||--o{ album_recordings : "appears on"
    tracks ||--o{ track_works : renders
    works ||--o{ track_works : "rendered by"
```

#### Tags and listening history

Tags can be attached to both works and recordings, optionally carrying a
value (e.g. a "Year" tag with `takes_value` set). Plays are logged per track
when known, and always per recording; the `*_last_played` views aggregate
`plays` for every other entity.

```mermaid
erDiagram
    tags {
        text tag_id PK
        text name
        bool takes_value
        bool private
    }

    works {
        text work_id PK
        text name
    }

    recordings {
        text recording_id PK
        text work_id FK
        text comment
    }

    tracks {
        text track_id PK
        text recording_id FK
    }

    work_tags {
        text work_id FK
        text tag_id FK
        text value
        int sequence_number
    }

    recording_tags {
        text recording_id FK
        text tag_id FK
        text value
        int sequence_number
    }

    plays {
        text play_id PK
        text track_id FK
        text recording_id FK
        timestamp played_at
    }

    works ||--o{ work_tags : "tagged with"
    tags ||--o{ work_tags : "assigned to"

    recordings ||--o{ recording_tags : "tagged with"
    tags ||--o{ recording_tags : "assigned to"

    recordings ||--o{ tracks : "split into"
    recordings ||--o{ plays : "logs a"
    tracks ||--o{ plays : "logs a"
```

### Internationalization

Execute the following commands from the project root directory to update
translation files whenever translatable strings have been changed.

1. Update `po/POTFILES`

    ```bash
    cat <<EOF > po/POTFILES
    data/de.johrpan.Musicus.desktop.in.in
    data/de.johrpan.Musicus.gschema.xml.in
    EOF

    find data/ui -name "*.blp" >> po/POTFILES
    find src musicus-library/src -name "*.rs" -a ! -name "config.rs" >> po/POTFILES
    ```

2. Update `po/template.pot`

    ```bash
    xgettext \
        --from-code=UTF-8 \
        --add-comments \
        --keyword=_ \
        --keyword=C_:1c,2 \
        --files-from=po/POTFILES \
        --output=po/template.pot
    ```

3. Update translation files

    ```bash
    msgmerge \
        --update \
        --backup=off \
        --no-fuzzy-matching \
        po/de.po \
        po/template.pot
    ```

### Building

#### Flatpak

```
flatpak-builder --repo=repo --force-clean build-dir flatpak/de.johrpan.Musicus.Devel.json
flatpak build-bundle repo de.johrpan.Musicus.Devel.flatpak de.johrpan.Musicus.Devel
flatpak install --user de.johrpan.Musicus.Devel.flatpak
```

#### Meson

```
meson setup _build --prefix="$HOME/.local"
ninja -C _build install
```

Ensure that `$HOME/.local/bin` is in `$PATH` and run the `musicus`
executable.

#### Windows

You will need MSYS2 installed.

Install Rust using `rustup` on your Windows system and switch to the
`x86_64-pc-windows-gnu` toolchain. Make sure `$USERPROFILE/.cargo/bin` is in
`$PATH` within your MSYS2 shell.

Install the dependencies:

```
pacman -S mingw-w64-ucrt-x86_64-gtk4 \
          mingw-w64-ucrt-x86_64-libadwaita \
          mingw-w64-ucrt-x86_64-gstreamer \
          mingw-w64-ucrt-x86_64-gst-plugins-base \
          mingw-w64-ucrt-x86_64-gst-plugins-good \
          mingw-w64-ucrt-x86_64-gst-libav \
          mingw-w64-ucrt-x86_64-gettext \
          mingw-w64-ucrt-x86_64-pkgconf \
          mingw-w64-ucrt-x86_64-gcc \
          mingw-w64-ucrt-x86_64-meson \
          mingw-w64-ucrt-x86_64-ninja \
          mingw-w64-ucrt-x86_64-python \
          mingw-w64-ucrt-x86_64-sqlite3 \
          zip
```

Install blueprint-compiler using PIP:

```
sudo pacman -S mingw-w64-ucrt-x86_64-python-pipx
pipx install blueprint-compiler
```

Make sure `$USERPROFILE/.local/bin` is in `$PATH`.

Build using Meson:

```
meson configure _build --prefix=$PWD/_build/install
ninja -C _build install
```

Run the application for testing:

```
export GSETTINGS_SCHEMA_DIR="$PWD/_build/install/share/glib-2.0/schemas"
export XDG_DATA_DIRS="$PWD/_build/install/share:$XDG_DATA_DIRS"
_build/install/bin/musicus.exe
```

Package the application:

```
bash build-aux/windows-package.sh
```

This will output a relocatable output directory `musicus_windows_portable` and
create `musicus_windows_portable.zip`.