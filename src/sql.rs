extern crate rusqlite;
extern crate chrono;

use rusqlite::{params, Connection, Result, Row};
use chrono::NaiveDateTime;

#[derive(Debug)]
pub struct Server {
    pub server_id: i32,
    pub address: String,
}

#[derive(Debug)]
pub struct ServerSettings {
    pub setting_id: i32,
    pub server_id: i32,
    pub name: String,
    pub max_players: i32,
    pub current_map: String,
    pub vac_status: bool,
    pub has_password: bool,
    pub game_version: String,
    pub bots: u8,
}

#[derive(Debug)]
pub struct Player {
    pub player_id: i32,
    pub name: String,
}

#[derive(Debug)]
pub struct Session {
    pub session_id: i32,
    pub server_id: i32,
    pub player_id: i32,
    pub score: i32,
    pub duration: f64,
    pub joined_at: NaiveDateTime,
    pub left_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct ServerEvent {
    pub event_id: i32,
    pub server_id: i32,
    pub event_type: String,
    pub event_data: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct PlayerEvent {
    pub event_id: i32,
    pub server_id: i32,
    pub player_id: i32,
    pub event_type: String,
    pub event_data: String, 
    pub created_at: NaiveDateTime,
}

pub fn insert_server(conn: &Connection, server: &Server) -> Result<usize> {
    conn.execute(
        "INSERT or IGNORE INTO servers (address) VALUES (?1)",
        params![&server.address,],
    )
}

pub fn _get_server(conn: &Connection, server_id: i32) -> Result<Server> {
    conn.query_row(
        "SELECT * FROM servers WHERE server_id = ?1",
        params![server_id],
        |row| Ok(map_to_server(row))
    )
}

pub fn get_server_by_addr(conn: &Connection, address: String) -> Result<Server> {
    conn.query_row(
        "SELECT * FROM servers WHERE address = ?1",
        params![address],
        |row| Ok(map_to_server(row))
    )
}

pub fn insert_server_settings(conn: &Connection, settings: &ServerSettings) -> Result<usize> {
    conn.execute(
        "INSERT INTO server_settings (server_id, name, max_players, current_map, vac_status, has_password, game_version, bots) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            settings.server_id,
            &settings.name,
            settings.max_players,
            &settings.current_map,
            settings.vac_status,
            settings.has_password,
            &settings.game_version,
            settings.bots,
        ],
    )
}

pub fn _get_server_settings(conn: &Connection, server_id: i32) -> Result<ServerSettings> {
    conn.query_row(
        "SELECT * FROM server_settings WHERE server_id = ?1 ORDER BY updated_at DESC LIMIT 1",
        params![server_id],
        |row| {
            Ok(ServerSettings {
                setting_id: row.get(0)?,
                server_id: row.get(1)?,
                name: row.get(2)?,
                max_players: row.get(3)?,
                current_map: row.get(4)?,
                vac_status: row.get(5)?,
                has_password: row.get(6)?,
                game_version: row.get(7)?,
                bots: row.get(8)?,
            })
        },
    )
}

fn map_to_server(row: &Row) -> Server {
    Server {
        server_id: row.get(0).unwrap(),
        address: row.get(1).unwrap(),
    }
}

pub fn insert_player(conn: &Connection, player: &Player) -> Result<usize> {
    conn.execute(
        "INSERT OR IGNORE INTO players (name) VALUES (?1) ON CONFLICT (name) DO NOTHING",
        params![&player.name],
    )
}

pub fn _get_player(conn: &Connection, player_id: i32) -> Result<Player> {
    conn.query_row(
        "SELECT * FROM players WHERE player_id = ?1",
        params![player_id],
        |row| {
            Ok(Player {
                player_id: row.get(0)?,
                name: row.get(1)?,
            })
        },
    )
}
pub fn get_player_by_name(conn: &Connection, name: String) -> Result<Player> {
    conn.query_row(
        "SELECT * FROM players WHERE name = ?1",
        params![name],
        |row| {
            Ok(Player {
                player_id: row.get(0)?,
                name: row.get(1)?,
            })
        },
    )
}

pub fn insert_session(conn: &Connection, session: &Session) -> Result<usize> {
    conn.execute(
        "INSERT INTO sessions (server_id, player_id, score, duration, joined_at, left_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![session.server_id, session.player_id, session.score, session.duration, session.joined_at.format("%Y-%m-%d %H:%M:%S").to_string(), session.left_at.format("%Y-%m-%d %H:%M:%S").to_string()]
    )
}

pub fn _get_session(conn: &Connection, session_id: i32) -> Result<Session> {
    conn.query_row(
        "SELECT * FROM sessions WHERE session_id = ?1",
        params![session_id],
        |row| {
            Ok(Session {
                session_id: row.get(0).unwrap(),
                server_id: row.get(1).unwrap(),
                player_id: row.get(2).unwrap(),
                score: row.get(3).unwrap(),
                duration: row.get(4).unwrap(),
                joined_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5).unwrap(), "%Y-%m-%d %H:%M:%S").unwrap(),
                left_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(6).unwrap(), "%Y-%m-%d %H:%M:%S").unwrap(),
            })
        },
    )
}

pub fn insert_server_event(conn: &Connection, event: &ServerEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO server_events (server_id, event_type, event_data, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![event.server_id, &event.event_type, &event.event_data, event.created_at.format("%Y-%m-%d %H:%M:%S").to_string()],
    )?;
    Ok(())
}

pub fn _get_server_event(conn: &Connection, event_id: i32) -> Result<ServerEvent> {
    conn.query_row(
        "SELECT * FROM server_events WHERE event_id = ?1",
        params![event_id],
        |row| {
            Ok(ServerEvent {
                event_id: row.get(0)?,
                server_id: row.get(1)?,
                event_type: row.get(2)?,
                event_data: row.get(3)?,
                created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(4).unwrap(), "%Y-%m-%d %H:%M:%S").unwrap(),
            })
        },
    )
}

pub fn insert_player_event(conn: &Connection, event: &PlayerEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO player_events (server_id, player_id, event_type, event_data, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![event.server_id, event.player_id, &event.event_type, event.event_data, event.created_at.format("%Y-%m-%d %H:%M:%S").to_string()],
    )?;
    Ok(())
}

pub fn _get_player_event(conn: &Connection, event_id: i32) -> Result<PlayerEvent> {
    conn.query_row(
        "SELECT * FROM player_events WHERE event_id = ?1",
        params![event_id],
        |row| {
            Ok(PlayerEvent {
                event_id: row.get(0)?,
                server_id: row.get(1)?,
                player_id: row.get(2)?,
                event_type: row.get(3)?,
                event_data: row.get(4)?,
                created_at: NaiveDateTime::parse_from_str(&row.get::<_, String>(5).unwrap(), "%Y-%m-%d %H:%M:%S").unwrap(),
            })
        },
    )
}
