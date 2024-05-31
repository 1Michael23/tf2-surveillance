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

# Copy Files
sudo cp config/* $CONFIG_DIR/

# Check if the database file exists
if [ -f "$DB_DIR/$DB_FILE" ]; then
    read -p "Database file already exists. Do you want to overwrite it? (y/n): " OVERWRITE
    if [[ "$OVERWRITE" == "y" || "$OVERWRITE" == "Y" ]]; then
        # Copy database
        sudo cp /path/to/your/database/file $DB_FILE
    else
        echo "Skipping database file overwrite."
    fi
else
    # Copy database
    sudo cp /path/to/your/database/file $DB_FILE
fi

# Set permissions
sudo chown -R $USER_NAME:$USER_NAME $CONFIG_DIR
sudo chown -R $USER_NAME:$USER_NAME $DB_DIR

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
