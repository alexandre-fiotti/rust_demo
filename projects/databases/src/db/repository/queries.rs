use diesel::prelude::*;
use crate::db::{repository::models::*, schema::repositories::dsl::*};

#[derive(Debug, thiserror::Error)]
pub enum InsertRepositoryError {
    #[error("InsertRepository: {source}")]
    InsertRepository{ 
        #[from]
        source: diesel::result::Error 
    },
}

pub fn insert_repository(
    conn: &mut PgConnection,
    new: &NewRepository
) -> Result<Repository, InsertRepositoryError> {
    diesel::insert_into(repositories)
        .values(new)
        .get_result(conn)
        .map_err(|source| InsertRepositoryError::InsertRepository{ source })
}

#[derive(Debug, thiserror::Error)]
pub enum GetRepositoryByNameError {
    #[error("GetRepositoryByName: {source}")]
    GetRepositoryByName{
        #[from] 
        source: diesel::result::Error
    },
}

pub async fn get_repository_by_name(
    conn: &mut PgConnection,
    owner_val: &str,
    name_val: &str
) -> Result<Option<Repository>, GetRepositoryByNameError> {
    repositories
        .filter(owner.eq(owner_val))
        .filter(name.eq(name_val))
        .first::<Repository>(conn)
        .optional()
        .map_err(|source| GetRepositoryByNameError::GetRepositoryByName{ source })
}
