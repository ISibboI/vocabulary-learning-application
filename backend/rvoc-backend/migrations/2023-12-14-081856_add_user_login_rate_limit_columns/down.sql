ALTER TABLE users
    DROP COLUMN login_attempt_count,
    DROP COLUMN failed_login_attempt_count,
    DROP COLUMN next_login_attempt_count_reset;