-- Create table for staff members
CREATE TABLE staff (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    pin CHAR(4) UNIQUE,
    cardid CHAR(6) UNIQUE,
    is_visible BOOLEAN NOT NULL DEFAULT TRUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Create table for events
CREATE TABLE events (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL,
    event_json TEXT NOT NULL
);