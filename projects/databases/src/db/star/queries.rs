use thiserror::Error;
use uuid::Uuid;
use chrono::NaiveDate;
use diesel::{dsl::{count_star, sql}, prelude::*, sql_types::Date};
use crate::db::{star::models::*, schema::stars::dsl::*};

#[derive(Debug, Error)]
pub enum InsertStarError {
    #[error("InsertStar: {source}")]
    InsertStar{ 
        #[from]
        source: diesel::result::Error 
    },
}

pub fn insert_star(
    conn: &mut PgConnection,
    new: &NewStar
) -> Result<Star, InsertStarError> {
    diesel::insert_into(stars)
        .values(new)
        .get_result(conn)
        .map_err(|source| InsertStarError::InsertStar{ source })
}

#[derive(Debug, Error)]
pub enum GetDailyStarCountError {
    #[error("GetDailyStarCount: {source}")]
    GetDailyStarCount{ 
        #[from] 
        source: diesel::result::Error 
    },
}

pub fn get_daily_star_count(
    conn: &mut PgConnection,
    repo_id_val: Uuid
) -> Result<Vec<(NaiveDate, i64)>, GetDailyStarCountError> {
    stars
        .filter(repository_id.eq(repo_id_val))
        .select((
            sql::<Date>("DATE(starred_at)"),
            count_star()
        ))
        .group_by(sql::<Date>("DATE(starred_at)"))
        .order_by(sql::<Date>("DATE(starred_at)"))
        .load::<(NaiveDate, i64)>(conn)
        .map_err(|source| GetDailyStarCountError::GetDailyStarCount{ source })
}

