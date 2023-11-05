# TF2 Surveillance

An advanced data logging software to track players across Source game servers and collect statistics.

## Installation

1. Install `dbrowser` for SQLite and create a database.
2. Initialize a new database with the SQL found in `db-tools/up.sql`.

## Usage

```plaintext
Usage: tf2-surveillance <target_server_file> [-v] [-m] [-r]

Scan and report information from a dedicated TF2 server.

Positional Arguments:
  target_server_file     Target server file

Options:
  -v, --verbose          Displays extra information
  -m, --monitor          Print all leave/join events
  -r, --report           Trigger Discord webhook on target join
  --help                 Display usage information
```

## Configuration

Settings found in config.toml

```toml
webhook_url = "https://discord.com/api/webhooks/..."
webhook_image = "http://images.clipartpanda.com/alarm-clipart-1408568727.png"
refresh_delay = 5
database_file = "players.db"
```

## Dependencies

Built with Rust and Python3.X. It's recommended to have it open in VSCode (VSCodium) for assistance with Jupyter notebook and live analysis.

## Contributing

Open to contributions.
