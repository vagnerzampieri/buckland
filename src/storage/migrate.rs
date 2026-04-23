use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../../migrations/0001_init.sql"))])
}

pub(crate) fn run(conn: &mut Connection) -> anyhow::Result<()> {
    migrations().to_latest(conn)?;
    Ok(())
}
