use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::dsl::{count_star, max};
use crate::db::{star::models::*, schema::stars::dsl::*};

#[derive(Debug, thiserror::Error)]
pub enum StarDbError {
    #[error("Diesel error: {0}")]
    DieselError(#[from] diesel::result::Error),
}

pub fn insert_star(
    conn: &mut PgConnection,
    new: &NewStar
) -> Result<Star, StarDbError> {
    diesel::insert_into(stars)
        .values(new)
        .get_result(conn)
        .map_err(StarDbError::from)
}

pub fn get_daily_star_count(
    conn: &mut PgConnection,
    repo_id_val: Uuid
) -> Result<Vec<(NaiveDate, i64)>, StarDbError> {
    use diesel::dsl::sql;
    use diesel::sql_types::Date;

    stars
        .filter(repository_id.eq(repo_id_val))
        .select((
            sql::<Date>("DATE(starred_at)"),
            count_star()
        ))
        .group_by(sql::<Date>("DATE(starred_at)"))
        .order_by(sql::<Date>("DATE(starred_at)"))
        .load::<(NaiveDate, i64)>(conn)
        .map_err(StarDbError::from)
}

pub fn get_latest_star_date(
    conn: &mut PgConnection,
    repo_id_val: Uuid
) -> Result<Option<NaiveDateTime>, StarDbError> {
    stars
        .filter(repository_id.eq(repo_id_val))
        .select(max(starred_at))
        .first::<Option<NaiveDateTime>>(conn)
        .map_err(StarDbError::from)
}
