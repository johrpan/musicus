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

1. Update `template.pot`

    ```bash
    xgettext \
        --from-code=UTF-8 \
        --add-comments \
        --keyword=_ \
        --keyword=C_:1c,2 \
        --files-from=po/POTFILES \
        --output=po/template.pot
    ```

2. Update translation files

    ```bash
    msgmerge \
        --update \
        --backup=off \
        --no-fuzzy-matching \
        po/de.po \
        po/template.pot
    ```