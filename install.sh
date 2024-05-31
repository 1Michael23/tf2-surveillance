#!/bin/bash

# Set variables
EXEC_PATH=/usr/local/bin/tf2-surveillance
CONFIG_DIR=/etc/tf2-surveillance
DB_DIR=/var/lib/tf2-surveillance
DB_FILE=players.db
SERVICE_FILE=/etc/systemd/system/tf2-surveillance.service

# Prompt for the username
read -p "Enter the username for running the service (default: $(whoami)): " USER_NAME
USER_NAME=${USER_NAME:-$(whoami)}

# Build Project
cargo build --release

# Copy Binaries
sudo cp target/release/tf2-surveillance $EXEC_PATH
sudo chmod +x $EXEC_PATH

# Create directories
sudo mkdir -p $CONFIG_DIR
sudo mkdir -p $DB_DIR

# Check if the database file exists
if [ -f "$CONFIG_DIR/config.toml" ]; then
    read -p "Config file already exists. Do you want to overwrite it? (y/n): " OVERWRITE_CONFIG
    if [[ "$OVERWRITE_CONFIG" == "y" || "$OVERWRITE_CONFIG" == "Y" ]]; then
        # Copy config
        sudo cp config/config.toml $CONFIG_DIR/config.toml
    else
        echo "Skipping config overwrite."
    fi
else
    # Copy config
    sudo cp config/config.toml $CONFIG_DIR/config.toml
fi

# Check if the database file exists
if [ -f "$DB_DIR/$DB_FILE" ]; then
    read -p "Database file already exists. Do you want to overwrite it? (y/n): " OVERWRITE_DB
    if [[ "$OVERWRITE_DB" == "y" || "$OVERWRITE_DB" == "Y" ]]; then
        # Copy database
        sudo cp db-tools/players-empty.db $DB_DIR/$DB_FILE
    else
        echo "Skipping database file overwrite."
    fi
else
    # Copy database
    sudo cp db-tools/players-empty.db $DB_DIR/$DB_FILE
fi

# Set permissions
sudo chown -R $USER_NAME:$USER_NAME $CONFIG_DIR
sudo chown -R $USER_NAME:$USER_NAME $DB_DIR

# Remove existing service file

sudo rm $SERVICE_FILE

# Create the systemd service file
echo "[Unit]
Description=TF2-Surveillance
After=network.target

[Service]
ExecStart=$EXEC_PATH -c $CONFIG_DIR/config.toml
Restart=always
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=tf2-surveillance
User=$USER_NAME
Group=$USER_NAME

[Install]
WantedBy=multi-user.target" | sudo tee $SERVICE_FILE

sudo systemctl daemon-reload
sudo systemctl enable --now tf2-surveillance.service
systemctl status tf2-surveillance.service
