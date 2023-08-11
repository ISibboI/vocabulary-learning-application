CREATE TABLE job_queue (
	name TEXT NOT NULL PRIMARY KEY,
	scheduled_execution_time TIMESTAMP WITH TIME ZONE NOT NULL,
	in_progress BOOLEAN NOT NULL DEFAULT false
);
