# Musicus

The classical music player and organizer.

https://musicus.org

## Hacking

For now you have to generate the static resources manually before the first
compilation and after changes to them. Use the following command line:

```
$ cd res && glib-compile-resources resources.xml && cd ..
```

Afterwards you can compile and run the program using:

```
$ cargo run
```

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