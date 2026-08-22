# musicus-testgen

A generator for exemplary Musicus libraries. It writes a complete library
folder — `musicus.musdb` plus track files — filled with procedurally generated
metadata, so that search, browsing, program generation, export and
reorganization can be tried out at a realistic scale without building a library
by hand.

```
cargo run -p musicus-testgen -- --output ~/musicus-testlib
cargo run -p musicus-testgen -- --output ~/musicus-testlib --recordings 2000 --seed 7
musicus-testgen --help
```

Point the app at the resulting folder to browse it (Musicus reads the folder
from its `library-path` setting).

Everything is written through `musicus-library`'s public API, the same
mutators the app itself uses, so the result is indistinguishable from a library
the user built by hand.

## What it generates

Persons, instruments, roles, ensembles, works and recordings, wired together:
ensembles have members, works have a composer and instrumentation, recordings
have performers and ensembles. `--recordings` is the primary size knob; the
other counts default to a share of it and can each be overridden.

It deliberately does *not* generate play history, albums, tags or nested
multi-part works.

## Two things to know

**The track files are not audio.** They are small placeholder files with an
`.mp3` extension. The library copies, names and catalogues them like any other
track — the file names come out of the usual filename pattern, so they look
right — but writing audio tags into them fails, which the import only logs. The
generated library is complete and browsable, but **not playable**.

**The names are invented.** They are assembled from syllable and word tables in
`src/names.rs`; no real person, ensemble or composition is involved. The point
is that the data looks plausible at a glance.

## Reproducibility

The same `--seed` always produces the same names and the same structure. The
database is not byte-identical between two runs, because entity IDs (UUIDs) and
timestamps are minted inside `musicus-library`.

## Safety

The generator only ever adds. It creates `--output` if it does not exist and
refuses to generate into a folder that is not empty unless `--force` is given;
it never deletes an existing file.
