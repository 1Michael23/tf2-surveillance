mod sql;

use rusqlite::Connection;
use chrono::{Local, NaiveDateTime};
use a2s::{info::Info, A2SClient};
use std::{collections::HashMap, fs::{self, read_to_string}, net::SocketAddr, process::exit, thread::sleep, time::{Duration, Instant}};
use argh::FromArgs;
use std::sync::{Arc, RwLock, atomic::{Ordering, AtomicUsize}};
use rayon::{prelude::*, ThreadPoolBuilder};

//If using heartbeat with uptimekuma, this will append the tf2 scan total latency at the end of the request
//If using heartbeat with a service other than uptimekuma change to false.
const UPTIMEKUMA_PING: bool = true;

#[macro_use]
extern crate json;

#[derive(FromArgs)]
///Scan and report information from a dedicated tf2 server
struct Arguments {
    ///config file
    #[argh(option, short = 'c')]
    config_file: String,
    ///sqlite db path
    #[argh(option, short = 'd')]
    db_file: Option<String>,
    ///target server path
    #[argh(option, short = 's')]
    server_file: Option<String>,
    ///target players path
    #[argh(option, short = 'p')]
    target_file: Option<String>,
    ///print all leave/join events
    #[argh(switch, short = 'm')]
    monitor: bool,    
}

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Deserialize)]
struct Config {
    webhook_enabled: bool,
    webhook_url: String,
    webhook_image: String,
    refresh_delay: u64,
    heartbeat_enabled: bool,
    heartbeat_url: String,
    database_file: String,
    server_file: String,
    target_file: String,
}

#[derive(Clone)]
struct Player {
    name: String,
    score: i32,
    duration: f32,
}

#[derive(Debug)]
enum ServerEvent {
    ServerUp(String),
    ServerDown(String),
    Settings(String, Info)
}

enum PlayerEvent {
    PlayerJoined(Player),
    PlayerLeft(Player),
    TargetJoined(Player),
    TargetLeft(Player),
    PointUpdate(Player, usize)
}


fn main() {

    let args: Arguments = argh::from_env();
    let config = load_config(args.config_file);

    let db_file = args.db_file.unwrap_or(config.database_file);

    //connect to database specified in config
    let mut connection = match Connection::open(db_file.clone()){
        Ok(connection) => {println!("Opened database at ({})", db_file); connection},
        Err(e) => {eprintln!("Failed to establish database connection ({:?})",e);exit(1)},
    };

    //Allocate space for running memory
    let saved_info: Arc<RwLock<HashMap<SocketAddr, Info>>> = Arc::new(RwLock::new(HashMap::new()));
    let saved_players = Arc::new(RwLock::new(HashMap::new()));
    let saved_player_events_by_server = Arc::new(RwLock::new(HashMap::new()));
    let saved_server_events: Arc<RwLock<HashMap<SocketAddr, Vec<ServerEvent>>>> = Arc::new(RwLock::new(HashMap::new()));
    let mut saved_target_players : Vec<String> = Vec::new();
    let target_server_addresses : Vec<SocketAddr> = try_read_lines(&args.server_file.unwrap_or(config.server_file.clone()))
        .expect("Failed to read target server file").iter()
        .filter_map(|address| address.parse().ok())
        .collect();

    //Allocate thread pool
    let pool = ThreadPoolBuilder::new().num_threads(200).build().unwrap();

    loop {
        //Load Targets and save to check for updated file.
        match try_read_lines(&args.target_file.clone().unwrap_or(config.target_file.clone())){
            Some(e) => {
                if saved_target_players != e {
                    saved_target_players = e;
                    println!("Loaded ({}) target players", saved_target_players.len());
                }
            },
            None => {},
        }

        let time_scan = Instant::now();

        let sucessful = AtomicUsize::new(0);
        let failed = AtomicUsize::new(0);
        let num_players = AtomicUsize::new(0);
        
        pool.install(|| {
            target_server_addresses.par_iter().for_each(|server| {

                let mut a2s_client = A2SClient::new().expect("Failed to create A2S client");
                a2s_client.max_size(3000);

                let mut server_events: Vec<ServerEvent> = Vec::new();
                let mut current_info : Option<Info> = None;
                
                match a2s_client.info(server) {
                    Ok(info) => {
                        //Check if any server settings have changed
                        let saved_info_read = saved_info.read().unwrap().clone();
                        match saved_info_read.get(server){
                            Some(previous) => {
                                if info.name == previous.name && info.vac == previous.vac && info.visibility == previous.visibility && info.bots == previous.bots && info.map == previous.map && info.max_players == previous.max_players{
                                }else {
                                    server_events.push(ServerEvent::Settings(server.to_string(), info.clone()));
                                }
                            },
                            None => {
                                server_events.push(ServerEvent::Settings(server.to_string(), info.clone()));
                            },
                        };

                        current_info = Some(info.clone());

                        server_events.push(ServerEvent::ServerUp(server.to_string()));
                        
                        let mut saved_info_write = saved_info.write().unwrap();
                        saved_info_write.insert(*server, info.clone());
                    },
                    Err(error) => {
                        server_events.push(ServerEvent::ServerDown(server.to_string()));
                        eprintln!("{} : {} : {} : {}",Local::now().format("%H:%M:%S"), "Server Query Failed",server.to_string(), error);    
                    }
                }

                {
                    let mut server_events_write = saved_server_events.write().unwrap();
                    server_events_write.insert(*server, server_events);
                }

                let previous_players: Vec<Player>;
                {
                    let saved_players_read = saved_players.read().unwrap();
                    previous_players = saved_players_read.get(server).unwrap_or(&Vec::new()).clone();
                }
                
                match a2s_client.players(server){
                    Ok(players) => {
                        let players = a2s_player_parse(&players);
                        let events = generate_player_events(&previous_players, &players, &saved_target_players);

                        for event in &events{
                            match event {
                                PlayerEvent::PlayerJoined(player) =>  if args.monitor {println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "Player Joined", player.name)},
                                PlayerEvent::PlayerLeft(player) => if args.monitor {println!("{} : {} : {} , Points: {}, Duration: {}", Local::now().format("%H:%M:%S"), "Player Left", player.name, player.score, format_duration(player.duration as usize))},
                                PlayerEvent::TargetJoined(player) => {
                                    println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "Target Joined", player.name);
                                    if config.webhook_enabled {send_alert(config.webhook_url.clone(), config.webhook_image.clone(), format!("__**{}**__ Detected in server \n({} : {})", player.name, match current_info.clone() {
                                        Some(info) => format!("{} : {}", info.name, info.map),
                                        None => "Unknown name : Unknown map".to_string(),
                                    }, server.to_string()),"🚨🚨🚨 Alert.".to_string(), 16711680)};
                                },
                                PlayerEvent::TargetLeft(player) => {
                                    println!("{} : {} : {} : time: {}", Local::now().format("%H:%M:%S"), "Target Left", player.name, format_duration(player.duration as usize));
                                    if config.webhook_enabled {send_alert(config.webhook_url.clone(), config.webhook_image.clone(), format!("__**{}**__ Left the server \n({} : {})\nPoints: {}, Duration: {}", player.name, match current_info.clone() {
                                        Some(info) => format!("{} : {}", info.name, info.map),
                                        None => "Unknown name : Unknown map".to_string(),
                                    }, server.to_string(), player.score, format_duration(player.duration as usize)), "🦀🦀🦀 Runner.".to_string(),22230)}
                                }
                                PlayerEvent::PointUpdate(_player, _total) => {
                                    //do nothing
                                }
                            };
                        }
                        
                        sucessful.fetch_add(1, Ordering::Relaxed);
                        num_players.fetch_add(players.len(), Ordering::Relaxed);
                        {
                            let mut saved_players_write = saved_players.write().unwrap();
                            saved_players_write.insert(*server, players);
                        }
                        {
                            let mut saved_events_write = saved_player_events_by_server.write().unwrap();
                            saved_events_write.insert(*server, events);
                        }
                    },
                    Err(error) => {
                        eprintln!("{} : {} : {} : {}",Local::now().format("%H:%M:%S"), "Player Query Failed",server.to_string(), error);
                        failed.fetch_add(1, Ordering::Relaxed);
                    },
                }
            });
        });

        let scan_time = time_scan.elapsed().as_millis();
        let db_time = Instant::now();
        let mut event_count = 0;

        for address in target_server_addresses.clone(){
            sql::insert_server(&connection, &sql::Server { server_id: 0, address: address.to_string() }).unwrap();
            event_count += 1;
        }

        for server_events in saved_server_events.read().unwrap().iter(){
            let server_id = sql::get_server_by_addr(&connection, server_events.0.to_string()).unwrap();
            for event in server_events.1{
                match event {
                    ServerEvent::ServerUp(_address) => {
                        sql::insert_server_event(&connection, &sql::ServerEvent { event_id: 0, server_id: server_id.server_id, event_type: "up".to_string(), event_data: "".to_string(), created_at: Local::now().naive_local() }).unwrap();
                    },
                    ServerEvent::ServerDown(_address) => {
                        sql::insert_server_event(&connection, &sql::ServerEvent { event_id: 0, server_id: server_id.server_id, event_type: "down".to_string(), event_data: "".to_string(), created_at: Local::now().naive_local() }).unwrap();
                    },
                    ServerEvent::Settings(_address, info) => {

                        let mut new_settings = sql::ServerSettings { 
                            setting_id: 0, 
                            server_id: server_id.server_id, 
                            name: info.name.to_string(), 
                            max_players: info.max_players as i32, 
                            current_map: info.map.to_string(), 
                            vac_status: info.vac, 
                            has_password: info.visibility, 
                            game_version: info.version.to_string(), 
                            bots: info.bots,
                            created_at: NaiveDateTime::from_timestamp_opt(1, 0).unwrap() //temporary unix time 1;
                        };

                        //Read from the database to check if settings have changed or just been dropped from memory (program restart)
                        match sql::get_server_settings(&connection, server_id.server_id){
                            Ok(mut previous_settings) => {
                                previous_settings.created_at = NaiveDateTime::from_timestamp_opt(1, 0).unwrap();
                                previous_settings.setting_id = 0;
                                if previous_settings != new_settings {
                                    new_settings.created_at = Local::now().naive_local();
                                    sql::insert_server_event(&connection, &sql::ServerEvent { event_id: 0, server_id: server_id.server_id, event_type: "setting change".to_string(), event_data: info.map.to_string(), created_at: Local::now().naive_local() }).unwrap();
                                    sql::insert_server_settings(&connection, &new_settings).unwrap();
                                    event_count += 1;
                                }
                            },
                            Err(_) => {
                                new_settings.created_at = Local::now().naive_local();
                                sql::insert_server_event(&connection, &sql::ServerEvent { event_id: 0, server_id: server_id.server_id, event_type: "setting change".to_string(), event_data: info.map.to_string(), created_at: Local::now().naive_local() }).unwrap();
                                sql::insert_server_settings(&connection, &new_settings).unwrap();
                                event_count += 1;
                            },
                        }
                    },
                }
                event_count += 1;
            }
        }

        for server_players in saved_players.read().unwrap().iter(){
            let players: Vec<_> = server_players.1.iter().map(|player| sql::Player { player_id: 0, name: player.name.clone() }).collect();
            let _ = sql::insert_players_batch(&mut connection, &players).unwrap();
        }

        for player_events in saved_player_events_by_server.read().unwrap().iter(){
            match sql::get_server_by_addr(&connection, player_events.0.to_string()){
                Ok(server) => {
                    for event in player_events.1 {   
                        match event {
                            PlayerEvent::PlayerJoined(player) => {
                                sql::insert_player_event(&connection, &player.name, &sql::PlayerEvent { 
                                    event_id: 0,  
                                    server_id: server.server_id, 
                                    player_id: 0,
                                    event_type: "join".to_string(), 
                                    created_at: Local::now().naive_local(),
                                    event_data: "".to_string(), 
                                }).unwrap();
                            },
                            PlayerEvent::PlayerLeft(player) => {
                                sql::insert_session(&connection, &player.name,&sql::Session { 
                                    session_id: 0, 
                                    server_id: server.server_id, 
                                    player_id: 0, 
                                    score: player.score, 
                                    duration: player.duration as f64, 
                                    joined_at: (Local::now() - Duration::from_secs(player.duration as u64)).naive_local(), 
                                    left_at: Local::now().naive_local() }).unwrap();
                                
                                sql::insert_player_event(&connection, &player.name, &sql::PlayerEvent { 
                                    event_id: 0,  
                                    server_id: server.server_id, 
                                    player_id: 0, 
                                    event_type: "leave".to_string(), 
                                    created_at: Local::now().naive_local(),
                                    event_data: "".to_string(), }).unwrap();

                            },
                            PlayerEvent::TargetJoined(player) => {
                                sql::insert_player_event(&connection,&player.name, &sql::PlayerEvent { 
                                    event_id: 0,  
                                    server_id: server.server_id, 
                                    player_id: 0, 
                                    event_type: "target join".to_string(), 
                                    created_at: Local::now().naive_local(),
                                    event_data: "".to_string(), }).unwrap();

                            },
                            PlayerEvent::TargetLeft(player) => {
                                sql::insert_session(&connection, &player.name,&sql::Session { 
                                    session_id: 0, 
                                    server_id: server.server_id, 
                                    player_id: 0, 
                                    score: player.score, 
                                    duration: player.duration as f64, 
                                    joined_at: (Local::now() - Duration::from_secs(player.duration as u64)).naive_local(), 
                                    left_at: Local::now().naive_local() }).unwrap();

                                sql::insert_player_event(&connection, &player.name, &sql::PlayerEvent { 
                                    event_id: 0,  
                                    server_id: server.server_id, 
                                    player_id: 0, 
                                    event_type: "target leave".to_string(), 
                                    created_at: Local::now().naive_local(),
                                    event_data: "".to_string(), }).unwrap();

                            },
                            PlayerEvent::PointUpdate(player, total) => {
                                sql::insert_player_event(&connection, &player.name, &sql::PlayerEvent { 
                                    event_id: 0, 
                                    server_id: server.server_id, 
                                    player_id: 0, 
                                    event_type: "point change".to_string(), 
                                    event_data: total.to_string(), 
                                    created_at: Local::now().naive_local() }).unwrap();

                            }
                        }
                        event_count += 1;
                    }
                },
                Err(_) => continue,
            }                
        }

        println!("{} : {} : Events({}) Players({}) scan({}ms) db({}ms)",Local::now().format("%H:%M:%S"), format!("Scanned ({}:{}:{})", target_server_addresses.len(), sucessful.fetch_or(0, Ordering::Relaxed), failed.fetch_or(0, Ordering::Relaxed)), event_count, num_players.fetch_or(0, Ordering::Relaxed), scan_time, db_time.elapsed().as_millis());

        let ping_param: String = match UPTIMEKUMA_PING {
            true => scan_time.to_string(),
            false => "".to_string(),
        };
        if config.heartbeat_enabled {send_heartbeat(config.heartbeat_url.clone() + &ping_param)}

        sleep(Duration::from_secs(config.refresh_delay));
    };
}

fn generate_player_events(previous_players : &Vec<Player>, current_players : &Vec<Player>, target_players: &Vec<String>) -> Vec<PlayerEvent>{
    let mut events : Vec<PlayerEvent> = Vec::new();

    let previous_names : Vec<String> = previous_players.iter().map(|player| player.name.clone()).collect();
    let current_names : Vec<String> = current_players.iter().map(|player| player.name.clone()).collect();

    for player in current_players {
        if !previous_names.contains(&&player.name) && !player.name.is_empty() {
            if target_players.contains(&player.name) {
                events.push(PlayerEvent::TargetJoined(player.clone()));
            } else {
                events.push(PlayerEvent::PlayerJoined(player.clone()));
            }
        }else if !player.name.is_empty() {
            for prev_player in previous_players{
                if player.name == prev_player.name{
                    if player.score != prev_player.score{
                        events.push(PlayerEvent::PointUpdate(player.clone(), player.score as usize))
                    }
                }
            }
        }
    }

    for player in previous_players {
        if !current_names.contains(&&player.name) & !player.name.is_empty() {
            if target_players.contains(&player.name){
                events.push(PlayerEvent::TargetLeft(player.clone()));
            }else {
                events.push(PlayerEvent::PlayerLeft(player.clone()));
            }      
        }
    }
    return events;
}

fn send_alert(url: String, image: String, input_string: String, title: String, color: u64) {
        let json_request = object! {
        username: "TF2-Alert",
        avatar_url: image,
        contents: "ALERT",
        embeds: [
            {
                title: title.to_string(),
                description: input_string.as_str(),
                color: color,
            }
        ]
    };

    let json = json_request.dump();

    ureq::post(&url)
        .set("Content-Type", "application/json")
        .send(json.as_bytes())
        .expect("Failed to post to webhook");
}

fn send_heartbeat(url: String) {
    let call = ureq::get(&url).call();
    match call {
        Ok(_) => (),
        Err(e) => eprintln!("Failed to send heartbeat ({})", e),
    }
}

fn try_read_lines(filename: &str) -> Option<Vec<String>> {
    match read_to_string(filename) {
        Ok(data) => Some(data.lines().map(String::from).collect()),
        Err(_) => None,
    } 
}

fn format_duration(input: usize) -> String{
    let hours = input / 3600;
    let minutes = (input % 3600) / 60;
    let seconds = input % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn a2s_player_parse(input: &[a2s::players::Player]) -> Vec<Player> {
    input.iter().map(|player| Player {
        name: player.name.clone(),
        score: player.score,
        duration: player.duration,
    }).collect()
}

fn load_config(path: String) -> Config {
    let contents = fs::read_to_string(path).expect("Failed to read configuration file");
    toml::from_str(&contents).expect("Failed to parse configuration file")
}
