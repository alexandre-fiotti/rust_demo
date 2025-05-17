use diesel::prelude::*;
use crate::db::{repository::models::*, schema::repositories::dsl::*};

#[derive(Debug, thiserror::Error)]
pub enum RepositoryDbError {
    #[error("Diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
}

pub fn insert_repository(
    conn: &mut PgConnection,
    new: &NewRepository
) -> Result<Repository, RepositoryDbError> {
    diesel::insert_into(repositories)
        .values(new)
        .get_result(conn)
        .map_err(RepositoryDbError::from)
}

pub fn get_repository_by_name(
    conn: &mut PgConnection,
    owner_val: &str,
    name_val: &str
) -> Result<Option<Repository>, RepositoryDbError> {
    repositories
        .filter(owner.eq(owner_val))
        .filter(name.eq(name_val))
        .first::<Repository>(conn)
        .optional()
        .map_err(RepositoryDbError::from)
}
