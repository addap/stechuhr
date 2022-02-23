-- Create table for staff members
CREATE TABLE staff (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    pin CHAR(4) NOT NULL,
    cardid CHAR(6) NOT NULL,
    status BOOLEAN NOT NULL DEFAULT FALSE
);

-- Create table for events
CREATE TABLE events (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event BLOB NOT NULL
);