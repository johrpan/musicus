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
    find src -name "*.rs" -a ! -name "config.rs" >> po/POTFILES
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