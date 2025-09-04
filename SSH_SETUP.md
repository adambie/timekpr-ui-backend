# SSH Key Setup for Timekpr UI

This document explains how to properly configure SSH keys for the Timekpr UI to remotely manage client machines.

## Overview

The Timekpr UI uses SSH keys (not passwords) to securely connect to remote machines. This prevents password prompts and ensures automated operation.

## SSH Keys Location

- **Private Key**: `ssh/timekpr_ui_key` (automatically generated, never commit to git)
- **Public Key**: `ssh/timekpr_ui_key.pub` (safe to share)

## Setup Instructions

### Step 1: SSH Key Generation (Already Done)

The SSH keys have been automatically generated in the `ssh/` directory:
- Private key: `ssh/timekpr_ui_key`
- Public key: `ssh/timekpr_ui_key.pub`

### Step 2: Deploy Public Key to Target Machines

For each computer you want to manage with Timekpr UI:

1. **Copy the public key to the target machine**:
   ```bash
   # Copy the public key content
   cat ssh/timekpr_ui_key.pub
   
   # On target machine, add it to authorized_keys
   ssh your-user@target-machine-ip
   ```

2. **On the target machine, set up the timekpr-remote user**:
   ```bash
   # Create the timekpr-remote user
   sudo useradd -m -s /bin/bash timekpr-remote
   
   # Add user to timekpr group
   sudo usermod -a -G timekpr timekpr-remote
   
   # Create SSH directory
   sudo mkdir -p /home/timekpr-remote/.ssh
   sudo chmod 700 /home/timekpr-remote/.ssh
   
   # Add the public key (paste the content from ssh/timekpr_ui_key.pub)
   sudo nano /home/timekpr-remote/.ssh/authorized_keys
   # Paste the public key content here
   
   # Set proper permissions
   sudo chmod 600 /home/timekpr-remote/.ssh/authorized_keys
   sudo chown -R timekpr-remote:timekpr-remote /home/timekpr-remote/.ssh
   ```

3. **Configure sudo for timekpr commands** (if needed):
   ```bash
   # Allow timekpr-remote to run timekpr commands without password
   sudo visudo
   # Add this line:
   # timekpr-remote ALL=(ALL) NOPASSWD: /usr/bin/timekpra
   ```

### Step 3: Test SSH Connection

Test the connection from your Timekpr UI machine:

```bash
# Test basic SSH connection
ssh -i ssh/timekpr_ui_key -o StrictHostKeyChecking=no -o BatchMode=yes timekpr-remote@TARGET_MACHINE_IP echo "Connection test"

# Test timekpr command
ssh -i ssh/timekpr_ui_key -o StrictHostKeyChecking=no -o BatchMode=yes timekpr-remote@TARGET_MACHINE_IP timekpra --help
```

If successful, you should see output without any password prompts.

## Docker Deployment

When using Docker, the SSH keys are automatically mounted:

```bash
# Run with Docker
docker-compose up -d

# The ssh/ directory is mounted read-only into the container at /app/ssh
```

## Security Notes

- **Private keys are never committed to git** (protected by .gitignore)
- **SSH keys use Ed25519 encryption** (modern, secure)
- **Connections use BatchMode** to prevent password prompts
- **Keys are mounted read-only** in Docker containers

## Troubleshooting

### "SSH key not found" errors:
- Ensure `ssh/timekpr_ui_key` exists in the project root
- Check file permissions: private key should be 600, public key should be 644

### "Permission denied" errors:
- Verify the public key is properly added to target machine's authorized_keys
- Check that timekpr-remote user exists on target machine
- Ensure timekpr-remote user is in the timekpr group

### "Connection timeout" errors:
- Verify target machine is reachable on port 22
- Check firewall settings
- Ensure SSH service is running on target machine

## Manual SSH Key Generation (if needed)

If you need to regenerate the keys:

```bash
# Remove old keys
rm -f ssh/timekpr_ui_key ssh/timekpr_ui_key.pub

# Generate new keys
ssh-keygen -t ed25519 -f ssh/timekpr_ui_key -N "" -C "timekpr-ui-remote-access"

# Set permissions
chmod 600 ssh/timekpr_ui_key
chmod 644 ssh/timekpr_ui_key.pub
```

## Quick Setup Script for Target Machines

Save this as `setup_target.sh` and run on each target machine:

```bash
#!/bin/bash
# Quick setup script for Timekpr UI target machines

PUBLIC_KEY="$1"
if [ -z "$PUBLIC_KEY" ]; then
    echo "Usage: $0 'ssh-ed25519 AAAAC3N... timekpr-ui-remote-access'"
    exit 1
fi

# Create user and setup SSH
sudo useradd -m -s /bin/bash timekpr-remote
sudo usermod -a -G timekpr timekpr-remote
sudo mkdir -p /home/timekpr-remote/.ssh
echo "$PUBLIC_KEY" | sudo tee /home/timekpr-remote/.ssh/authorized_keys
sudo chmod 700 /home/timekpr-remote/.ssh
sudo chmod 600 /home/timekpr-remote/.ssh/authorized_keys
sudo chown -R timekpr-remote:timekpr-remote /home/timekpr-remote/.ssh

echo "Setup complete for timekpr-remote user"
echo "Test with: ssh -i ssh/timekpr_ui_key timekpr-remote@$(hostname -I | awk '{print $1}')"
```

Usage:
```bash
# Copy your public key
cat ssh/timekpr_ui_key.pub

# Run setup script on target machine
chmod +x setup_target.sh
./setup_target.sh "ssh-ed25519 AAAAC3N... timekpr-ui-remote-access"
```