sea-orm-cli migrate generate NAME_OF_MIGRATION [--local-time]

# E.g. to generate `migration/src/m20220101_000001_create_table.rs` shown below
sea-orm-cli migrate generate create_table

# This create the same migration file as above command
sea-orm-cli migrate generate "create table"sea-orm-cli migrate generate NAME_OF_MIGRATION [--local-time]

# E.g. to generate `migration/src/m20220101_000001_create_table.rs` shown below
sea-orm-cli migrate generate create_table

# This create the same migration file as above command
sea-orm-cli migrate generate "create table"

#Generate entities
sea-orm-cli generate entity \
-u "postgres://myuser:mysecretpassword@localhost:5432/mydatabase" \
-o "src/entities" \
--with-serde both \
--serde-skip-deserializing-primary-key \
--expanded-format