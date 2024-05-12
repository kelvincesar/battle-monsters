use diesel::r2d2::{self, ConnectionManager};
use dotenvy::dotenv;
use diesel::PgConnection;

type DBPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub struct Database {
    pool: DBPool,
}

impl Database {
    pub fn new() -> Self {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool: DBPool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");
        Database { pool }
    }

    pub fn get_connection(&self) -> r2d2::PooledConnection<ConnectionManager<PgConnection>> {
        self.pool.get().expect("Failed to get a database connection")
    }
}
