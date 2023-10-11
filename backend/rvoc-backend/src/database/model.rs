use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

#[derive(Insertable, Queryable, Selectable, Identifiable, AsChangeset, Clone, Debug)]
#[diesel(table_name = crate::database::schema::job_queue)]
#[diesel(primary_key(name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ScheduledJob {
    pub scheduled_execution_time: DateTime<Utc>,
    pub name: String,
    pub in_progress: bool,
}

impl ScheduledJob {
    /// Sets `in_progress` to `true`, but panics if it was set to true already.
    pub fn set_in_progress(mut self) -> Self {
        assert!(!self.in_progress);
        self.in_progress = true;
        self
    }
}
