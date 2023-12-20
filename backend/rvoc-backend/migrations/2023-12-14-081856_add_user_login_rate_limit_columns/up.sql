ALTER TABLE users
    ADD COLUMN login_attempt_count INT NOT NULL DEFAULT(0),
    ADD COLUMN failed_login_attempt_count INT NOT NULL DEFAULT(0),
    ADD COLUMN next_login_attempt_count_reset TIMESTAMPTZ NOT NULL DEFAULT(NOW());