ALTER TABLE spaces__user_space
    DROP CONSTRAINT user_space_unique_member,
    ADD CONSTRAINT spaces__user_space_pk PRIMARY KEY (space_id, coach_id);