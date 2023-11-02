extern crate a2s;
extern crate chrono;

use chrono::Local;
use a2s::{info::Info, players::Player, A2SClient};
use std::{net::SocketAddr, thread::sleep, time::Duration, fs::read_to_string, str::FromStr};
use owo_colors::OwoColorize;
use argh::FromArgs;

#[macro_use]
extern crate json;

const WEBHOOK_URL: &str = "https://discord.com/api/webhooks/1167774134634303570/BsKlnw9B83Ety-aeNuA7bx3s1B78R-eKTQXLN2jplHaWGyR0cdUY98hXSJ5Hqq2p5SBL";
const AVATAR_URL: &str = "http://images.clipartpanda.com/alarm-clipart-1408568727.png";

#[derive(FromArgs)]
///Scan and report information from a dedicated tf2 server
struct Arguments {
    ///IP Address and port of the specefied server
    #[argh(positional)]
    address: String,
    ///displays extra information
    #[argh(switch, short = 'v')]
    verbose: bool
}

enum ServerEvent {
    MapChange(String),
    RoundChange(),
}

enum PlayerEvent {
    PlayerJoined(Player),
    PlayerLeft(Player),
    TargetJoined(Player),
}

fn main() {

    let args: Arguments = argh::from_env();

    let addr = SocketAddr::from_str(&args.address).expect("Invalid address");

    let mut client = A2SClient::new().expect("Failed to create A2S client");

    client.max_size(3000);

    let mut saved_info : Option<Info> = None;
    let mut saved_players : Vec<Player> = Vec::new();
    let mut saved_targets : Vec<String> = Vec::new();

    loop {
        //Load Targets and save to check for updated file.
        match try_read_lines("target_players.txt"){
            Some(e) => {
                if saved_targets != e {
                    saved_targets = e;
                    println!("Loaded Targeted Players:");
                    for name in &saved_targets{
                        println!("{}",name)
                    }
                }
            },
            None => continue,
        }
        
        //Query server info
        match client.info(&addr) {
            Ok(info) => {
                match args.verbose {
                    true => {
                            println!("{} : {} : {}",Local::now().format("%H:%M:%S"), "Server Query Sucessful".green(), info.name);
                            println!("Server Name: {}", info.name.green());
                            println!("Map: {}", info.map);
                            println!("Player Count: {}/{}", info.players, info.max_players);
                            println!("Server Version: {}", info.version);},
                    false => println!("{} : {} : {}",Local::now().format("%H:%M:%S"), "Server Query Sucessful".green(), info.name),
                }

                saved_info = Some(info);

            }
            Err(e) => {
                eprintln!("Failed to query server: {}", e);
            }
        }

        //Query player info
        match client.players(&addr) {
            Ok(players) => {
                match args.verbose {
                    true => {
                        println!("{} : {} : {} Players",Local::now().format("%H:%M:%S"), "Player Query Sucessful".green(), players.len());
                        for player in &players {
                            println!("Player: {: <15} | Score: {:<4} | Time: {}", player.name.green(), player.score.red(), player.duration.blue());
                        }},
                    false => println!("{} : {} : {} Players",Local::now().format("%H:%M:%S"), "Player Query Sucessful".green(), players.len())
                }

                let events = generate_player_events(&saved_players, &players, &saved_targets);
                
                for event in events{
                    match event {
                        PlayerEvent::PlayerJoined(player) => println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "Player Joined".yellow(), player.name),
                        PlayerEvent::PlayerLeft(player) => println!("{} : {} : {} , Points: {}, Duration: {:?}", Local::now().format("%H:%M:%S"), "Player Left".blue(), player.name, player.score, Duration::from_secs(player.duration as u64)),
                        PlayerEvent::TargetJoined(player) => {
                            println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "Target Joined".red(), player.name); 
                            send_alert(format!("__**{}**__ Detected in server ({} : {})", player.name, match &saved_info{
                                    Some(info) => format!("{} : {}", info.name, info.map),
                                    None => "Unknown name : Unknown map".to_string(),
                                }, addr.to_string()))},
                    }
                }

                saved_players = players;
                
            }
            Err(e) => {
                eprintln!("Failed to query player list: {}", e);
            }
        }
        sleep(Duration::from_secs(3));
    }
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

fn send_alert(input_string: String) {
    let json_request = object! {
        username: "TF2-Alert",
        avatar_url: AVATAR_URL,
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
    let url = WEBHOOK_URL;

    ureq::post(url)
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