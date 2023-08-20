ALTER TABLE sessions ADD COLUMN username VARCHAR(50) REFERENCES users (name);
