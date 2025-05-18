pub mod schema;
pub mod star;
pub mod repository;

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;