use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

#[derive(Insertable, Queryable, Selectable, Identifiable, AsChangeset, Clone, Debug)]
#[diesel(table_name = crate::schema::job_queue)]
#[diesel(primary_key(name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ScheduledJob {
    pub scheduled_execution_time: DateTime<Utc>,
    pub name: String,
    pub in_progress: bool,
}
