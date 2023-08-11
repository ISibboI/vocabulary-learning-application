CREATE TABLE job_queue (
	scheduled_execution_time TIMESTAMP NOT NULL,
	name TEXT NOT NULL,
	in_progress BOOLEAN NOT NULL DEFAULT false,
	PRIMARY KEY (scheduled_execution_time, name)
);
