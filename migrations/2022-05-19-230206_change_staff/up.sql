-- Your SQL goes here
ALTER TABLE staff 
ADD COLUMN 
    is_visible BOOLEAN NOT NULL DEFAULT TRUE;