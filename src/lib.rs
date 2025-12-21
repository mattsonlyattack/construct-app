pub mod db;

pub use db::Database;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_accessible_from_crate_root() {
        let db = Database::in_memory();
        assert!(db.is_ok());
    }
}
