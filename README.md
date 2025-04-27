# Musicus

The classical music player and organizer.

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