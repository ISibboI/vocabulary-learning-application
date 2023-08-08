CREATE TABLE job_queue (
	scheduled_execution_time TIMESTAMP PRIMARY KEY,
	name TEXT NOT NULL,
	in_progress BOOLEAN NOT NULL DEFAULT false
);
