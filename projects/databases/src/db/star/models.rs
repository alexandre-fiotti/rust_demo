use chrono::NaiveDateTime;
use uuid::Uuid;
use diesel::prelude::*;
use crate::db::schema::stars;
use crate::db::repository::models::Repository;

#[derive(Debug, Clone, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(Repository))] // ✅ Now it resolves
#[diesel(table_name = stars)]
pub struct Star {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub stargazer: String,
    pub email: Option<String>,
    pub starred_at: NaiveDateTime,
    pub fetched_at: NaiveDateTime,
}


#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = stars)]
pub struct NewStar<'a> {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub stargazer: &'a str,
    pub email: Option<&'a str>,
    pub starred_at: NaiveDateTime,
    pub fetched_at: NaiveDateTime,
}
