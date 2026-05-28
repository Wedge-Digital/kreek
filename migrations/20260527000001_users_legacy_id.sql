ALTER TABLE auth__users
    ADD COLUMN legacy_id INTEGER UNIQUE;