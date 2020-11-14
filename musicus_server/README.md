# Musicus Server

This is a server for hosting metadata on classical music.

## Running

The Musicus server should reside behind a reverse proxy (e.g. Nginx) that is
set up to only use TLS encrypted connections. You will need a running
[PostgreSQL](https://www.postgresql.org/) service. To set up the database (and
migrate to future versions) use the Diesel command line utility from within
the source code repository. This utility and the Musicus server itself use the
environment variable `MUSICUS_DATABASE_URL` to find the database. A nice way to
set it up is to use a file called `.env` within the toplevel directory of the
repository.

```bash
# Install the Diesel command line utility:
cargo install diesel_cli --no-default-features --features postgres

# Configure the database URL (replace username and table):
echo "MUSICUS_DATABASE_URL=\"postgres://username@localhost/table\"" >> .env

# Run migrations:
~/.cargo/bin/diesel migration run

# Set a secret that will be used to sign access tokens:
echo "MUSICUS_SECRET=\"$(openssl rand -base64 64)\"" >> .env
```

## Hacking

The Musicus server is written in [Rust](https://www.rust-lang.org) using the
[Actix Web](https://actix.rs/) framework for serving requests and
[Diesel](https://diesel.rs/) for database access. The linked websites should
provide you with the necessary information to get started.

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