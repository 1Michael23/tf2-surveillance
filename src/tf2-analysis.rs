mod sql;
use a2s::players;
use argh::FromArgs;
use rusqlite::Connection;

#[derive(FromArgs)]
///Reads and analyses data from a database.
struct Arguments {
    ///sqlite db path
    #[argh(option, short = 'd')]
    db_file: String,
}

fn main(){

    let args: Arguments = argh::from_env();

    println!("Running analysis binary");

    let db_file = args.db_file;

    let mut ram_connection = Connection::open_in_memory().unwrap();
    let mut disk_connection = Connection::open(db_file).expect("Failed to establish database connection");

}