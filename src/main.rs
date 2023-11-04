extern crate a2s;
extern crate chrono;
extern crate serde;
extern crate toml;

use chrono::Local;
use a2s::{info::Info, A2SClient};
use std::{net::SocketAddr, thread::sleep, time::Duration, fs::{self, read_to_string}, collections::HashMap};
use owo_colors::OwoColorize;
use argh::FromArgs;

use std::sync::{Arc, RwLock, atomic::{Ordering, AtomicUsize}};
use rayon::{prelude::*, ThreadPoolBuilder};

#[macro_use]
extern crate json;

#[derive(FromArgs)]
///Scan and report information from a dedicated tf2 server
struct Arguments {
    ///target server file
    #[argh(positional)]
    target_server_file: String,
    ///displays extra information
    #[argh(switch, short = 'v')]
    verbose: bool,
    ///print all leave/join events
    #[argh(switch, short = 'm')]
    monitor: bool,
    ///trigger discord webhook on target join.
    #[argh(switch, short = 'r')]
    report: bool,
    
}

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Deserialize)]
struct Config {
    webhook_url: String,
    webhook_image: String,
    refresh_delay: u64,
}

#[derive(Clone)]
struct Player {
    name: String,
    score: i32,
    duration: f32,
}

enum PlayerEvent {
    PlayerJoined(Player),
    PlayerLeft(Player),
    TargetJoined(Player),
}

fn main() {

    let args: Arguments = argh::from_env();
    let config = load_config();

    let pool = ThreadPoolBuilder::new().num_threads(200).build().unwrap();

    let saved_info = Arc::new(RwLock::new(HashMap::new()));
    let saved_players = Arc::new(RwLock::new(HashMap::new()));
    let mut saved_targets : Vec<String> = Vec::new();
    let target_server_addresses : Vec<SocketAddr> = try_read_lines(&args.target_server_file)
        .expect("Failed to read target server file").iter()
        .filter_map(|address| address.parse().ok())
        .collect();
    

    loop {
        //Load Targets and save to check for updated file.
        match try_read_lines("target_players.txt"){
            Some(e) => {
                if saved_targets != e {
                    saved_targets = e;
                    println!("{}", "Loaded Targeted Players:".green());
                    for name in &saved_targets{
                        println!("{}",name)
                    }
                }
            },
            None => continue,
        }

        let sucessful = AtomicUsize::new(0);
        let failed = AtomicUsize::new(0);
        let event_count = AtomicUsize::new(0);
        let players_under_my_domain = AtomicUsize::new(0);
        
        pool.install(|| {
            target_server_addresses.par_iter().for_each(|server| {

                let mut client = A2SClient::new().expect("Failed to create A2S client");
                client.max_size(3000);

                let mut current_info: Option<Info> = None;
                match client.info(server) {
                    Ok(info) => {
                        let mut saved_info_write = saved_info.write().unwrap();
                        current_info = Some(info.clone());
                        saved_info_write.insert(*server, info);
                    },
                    Err(error) => if args.verbose {eprintln!("{} : {} : {} : {}",Local::now().format("%H:%M:%S"), "Server Query Failed".red(),server.to_string(), error)},
                }

                let previous_players: Vec<Player>;
                {
                    let saved_players_read = saved_players.read().unwrap();
                    previous_players = saved_players_read.get(server).unwrap_or(&Vec::new()).clone();
                }
                
                match client.players(server){
                    Ok(players) => {
                        let players = a2s_player_parse(&players);
                        let events = generate_player_events(&previous_players, &players, &saved_targets);
                        event_count.fetch_add(events.len(), Ordering::Relaxed);
                        for event in &events{
                            match event {
                                PlayerEvent::PlayerJoined(player) =>  if args.monitor {println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "Player Joined".yellow(), player.name)},
                                PlayerEvent::PlayerLeft(player) => if args.monitor {println!("{} : {} : {} , Points: {}, Duration: {:?}", Local::now().format("%H:%M:%S"), "Player Left".blue(), player.name, player.score, Duration::from_secs(player.duration as u64))},
                                PlayerEvent::TargetJoined(player) => {
                                                        println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "Target Joined".red(), player.name); 
                                                        if args.report {send_alert(config.webhook_url.clone(), config.webhook_image.clone(), format!("__**{}**__ Detected in server ({} : {})", player.name, match current_info.clone() {
                                                                Some(info) => format!("{} : {}", info.name, info.map),
                                                                None => "Unknown name : Unknown map".to_string(),
                                                            }, server.to_string()))}},
                            }
                        }
                        sucessful.fetch_add(1, Ordering::Relaxed);
                        players_under_my_domain.fetch_add(players.len(), Ordering::Relaxed);
                        {
                            let mut saved_players_write = saved_players.write().unwrap();
                            saved_players_write.insert(*server, players);
                        }
                    },
                    Err(error) => {
                        if args.verbose {eprintln!("{} : {} : {} : {}",Local::now().format("%H:%M:%S"), "Player Query Failed".red(),server.to_string(), error)};
                        failed.fetch_add(1, Ordering::Relaxed);
                    },
                }
            });
        });
        println!("{} : {} : Events ({}), Players({})",Local::now().format("%H:%M:%S"), format!("Scanned ({}:{}:{})", target_server_addresses.len(), sucessful.fetch_or(0, Ordering::Relaxed).green(), failed.fetch_or(0, Ordering::Relaxed).red()), event_count.fetch_or(0, Ordering::Relaxed).blue(), players_under_my_domain.fetch_or(0, Ordering::Relaxed));
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
        }
    }

    for player in previous_players {
        if !current_names.contains(&&player.name) & !player.name.is_empty() {
            events.push(PlayerEvent::PlayerLeft(player.clone()));
        }
    }

    return events;
}

fn send_alert(url: String, image: String, input_string: String) {
    let json_request = object! {
        username: "TF2-Alert",
        avatar_url: image,
        contents: "ALERT",
        embeds: [
            {
                title: "🚨🚨🚨",
                description: input_string.as_str(),
                color: 16711680,
            }
        ]
    };

    let json = json_request.dump();

    ureq::post(&url)
        .set("Content-Type", "application/json")
        .send(json.as_bytes())
        .expect("Failed to post to webhook");
}

fn try_read_lines(filename: &str) -> Option<Vec<String>> {
    match read_to_string(filename) {
        Ok(data) => Some(data.lines().map(String::from).collect()),
        Err(_) => None,
    } 
}

fn a2s_player_parse(input: &[a2s::players::Player]) -> Vec<Player> {
    input.iter().map(|player| Player {
        name: player.name.clone(),
        score: player.score,
        duration: player.duration,
    }).collect()
}

fn load_config() -> Config {
    let contents = fs::read_to_string("config.toml").expect("Failed to read configuration file");
    toml::from_str(&contents).expect("Failed to parse configuration file")
}