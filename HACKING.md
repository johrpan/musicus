# Blueprint files

When adding a new Blueprint file in `data/res/`, remember to add it to both, the
list of bluprint sources in `data/res/meson.build` and the list of resources in
`data/res/musicus.gresource.xml` (`.blp` replaced with `.ui`).

# Internationalization

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