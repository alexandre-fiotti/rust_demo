use chrono::NaiveDateTime;
use uuid::Uuid;
use diesel::prelude::*;
use crate::db::schema::repositories;

#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = repositories)]
pub struct Repository {
    pub id: Uuid,
    pub owner: String,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = repositories)]
pub struct NewRepository<'a> {
    pub id: Uuid,
    pub owner: &'a str,
    pub name: &'a str,
}
