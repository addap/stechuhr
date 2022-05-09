-- Create table for staff members
CREATE TABLE staff (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    pin CHAR(4) NOT NULL UNIQUE,
    cardid CHAR(6) NOT NULL UNIQUE,
    status BOOLEAN NOT NULL DEFAULT FALSE
);

-- Create table for events
CREATE TABLE events (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP NOT NULL,
    event_json TEXT NOT NULL
);