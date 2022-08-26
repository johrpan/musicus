All commands should be executed from this directory!

Regenerate `POTFILES.in` using:

```bash
find ../crates \( -name \*.rs -o -name \*.ui \) -print > POTFILES.in
```

Update `musicus.pot` using:

```bash
xgettext -f POTFILES.in -o musicus.pot
```

Update the translation files using e.g.:

```bash
msgmerge de.po musicus.pot > tmp.po
# Inspect tmp.po for errors.
mv tmp.po de.po
```
