CREATE TABLE servers (
    server_id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    max_players INTEGER NOT NULL
);

CREATE TABLE players (
    player_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE sessions (
    session_id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL REFERENCES servers(server_id),
    player_id INTEGER NOT NULL REFERENCES players(player_id),
    score INTEGER NOT NULL,
    duration REAL NOT NULL,
    joined_at DATETIME NOT NULL,
    left_at DATETIME NOT NULL
);

CREATE TABLE server_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL REFERENCES servers(server_id),
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    created_at DATETIME NOT NULL
);

CREATE TABLE player_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL REFERENCES servers(server_id),
    player_id INTEGER NOT NULL REFERENCES players(player_id),
    event_type TEXT NOT NULL,
    created_at DATETIME NOT NULL
);