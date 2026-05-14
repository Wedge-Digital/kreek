ALTER TABLE spaces__user_space
    ADD CONSTRAINT user_space_unique_member UNIQUE (space_id, coach_id);
