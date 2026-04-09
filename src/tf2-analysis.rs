mod sql;
use argh::FromArgs;
use rusqlite::Connection;
use std::{process::exit, time::Instant};

#[derive(FromArgs)]
///Reads and analyses data from a database.
struct Arguments {
    ///sqlite db path
    #[argh(option, short = 'd')]
    db_file: String,
}

fn main() {
    let args: Arguments = argh::from_env();

    let db_file = args.db_file;

    let mut connection = match Connection::open(db_file.clone()) {
        Ok(disk_connection) => {
            println!("DB opened ({})", db_file);
            disk_connection
        }
        Err(e) => {
            eprintln!("Failed to establish database connection ({})", e);
            exit(1)
        }
    };

    let start = Instant::now();

    let mut sessions = sql::get_all_sessions(&connection).unwrap();
    println!("({:?}) Sessions: {}", start.elapsed(), sessions.len());
    let mut server_events = sql::get_all_server_events(&connection).unwrap();
    println!("({:?}) Server Events: {}", start.elapsed(), server_events.len());
    let mut player_events = sql::get_all_player_events(&connection).unwrap();
    println!("({:?}) Player Events: {}", start.elapsed(), player_events.len());

    println!("{:?}", sessions.get(sessions.len()-1).unwrap());

    loop {
        
    }

}
