use migration::Migrator;
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    let databse_url = "postgres://myuser:mysecretpassword@localhost:5432/mydatabase";

    let connection = sea_orm::Database::connect(databse_url)
        .await
        .expect("Failed to connect to the database");

    Migrator::up(&connection, None)
        .await
        .expect("Migration failed");
}
