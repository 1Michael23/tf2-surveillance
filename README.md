# TF2 Surveillance

An advanced data logging software to track players across Source game servers and collect statistics.

# Features

- Multithreaded scanning.
- Trigger webhook on target join.
- Extensive logging of all player sessions, join/leave times, and server activity.
- Jupyter Notebook for database reads and data search/analysis.

# Flaws

The steam server API that this program uses only provides two datapoints for each player, their current **name**, **score**, and **duration** on the server.
For this reason it can not:
- Distinguish multiple players with the same name
- Track players across name changes
- Associate players with Steam ID

The "score" attribute in tf2 is a direct code port from counterstrike, and as such it only displays kills, not the players score.

# Installation

```install.sh``` is only compatable with linux distributions running systemd.

however it can be compiled manually and run on any platform.

# Usage

```plaintext
Usage: tf2-surveillance [-d <db-file>] [-s <server-file>] [-p <target-file>] [-v] [-m]

Scan and report information from a dedicated tf2 server

Options:
  -d, --db-file     sqlite db path
  -s, --server-file target server path
  -p, --target-file target players path
  -v, --verbose     displays extra information
  -m, --monitor     print all leave/join events
  --help            display usage information
```

#### blacklist_extract.py

To get the ip addresses of many servers at once with the community browser tab, blacklist any server you want to target, then find the text file server_blacklist.txt, located in tf/cfg in the game files.
```plaintext
Usage: python blacklist_extract.py <input_file> <output_file>
```

#### analysis.ipynb

This is the jupyter notebook where the database is read and analysied, this is only tested in and intended to be used with visual studio code (vscodium perfered.)

Some modules are included, these need slight modification such as inputting a target file

Some also depend on data loaded by a previous module

## Configuration

Settings found in config.toml

Install location: ```/etc/tf2-surveillance/config.toml```

```toml
webhook_enabled = 0
webhook_url = "https://discord.com/api/webhooks/..." #place your discord webhook here
webhook_image = "http://images.clipartpanda.com/alarm-clipart-1408568727.png"
refresh_delay = 5
database_file = "/var/lib/tf2-surveillance/players.db"
server_file = "/etc/tf2-surveillance/target_servers.txt" #requires program restart to reload
target_file = "/etc/tf2-surveillance/target_players.txt" #does not require restart to reload
```

## Contributing

Open to contributions.
