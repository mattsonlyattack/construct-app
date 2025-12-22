pub mod db;
pub mod models;

pub use db::Database;
pub use models::{Note, NoteBuilder, Tag, TagAssignment, TagSource};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_accessible_from_crate_root() {
        let db = Database::in_memory();
        assert!(db.is_ok());
    }

    #[test]
    fn types_accessible_from_crate_root() {
        use time::OffsetDateTime;

        let tag = Tag::new(1, "test");
        assert_eq!(tag.name, "test");

        let source = TagSource::User;
        assert_eq!(format!("{}", source), "user");

        let now = OffsetDateTime::now_utc();
        let tag_assignment = TagAssignment::user_created(1, now);
        assert_eq!(tag_assignment.confidence, 100);

        let note = NoteBuilder::new().id(1).content("test").build();
        assert_eq!(note.content, "test");
    }
}
