extern crate a2s;
extern crate chrono;

use chrono::Local;
use a2s::{info::Info, players::Player, A2SClient};
use std::{net::SocketAddr, thread::sleep, time::Duration, fs::read_to_string, str::FromStr};
use owo_colors::OwoColorize;
use argh::FromArgs;

#[macro_use]
extern crate json;

const WEBHOOK_URL: &str = "WEBHOOK GOES HERE";
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

fn main() {

    let args: Arguments = argh::from_env();

    let addr = SocketAddr::from_str(&args.address).expect("Invalid address");

    let client = A2SClient::new().expect("Failed to create A2S client");

    let target_players = try_read_lines("target_players.txt");
    if target_players.is_some(){
        println!("Loaded Targeted Players: ");
            for line in target_players.clone().unwrap(){
                println!("{}", line);
            }
    }

    let mut saved_info : Option<Info> = None;
    let mut saved_players : Vec<Player> = Vec::new();

    loop {
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

                //Alert when a target player joints the server.
                if let Some(target_players) = &target_players {
                    for target_name in target_players{
                        if !(saved_players.iter().any(|saved_player| saved_player.name == *target_name))
                            && (players.iter().any(|player| player.name == *target_name)){
                                send_alert(format!("{} detected on ({} : {})",target_name,
                                            match saved_info {
                                                Some(ref e) => e.name.clone(),
                                                None => "Unknown Map".to_string(),
                                            },
                                            addr.to_string()
                                        ));
                        }
                    }
                }

                saved_players = players.clone();
                
            }
            Err(e) => {
                eprintln!("Failed to query player list: {}", e);
            }
        }
        sleep(Duration::from_secs(15));
    }
}

fn send_alert(input_string: String) {
    println!("{} : {} : {}", Local::now().format("%H:%M:%S"), "ALERT CALLED!!!".red(), input_string);
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