# DigitalOcean Droplet Bootstrap

## Requirements

- Ubuntu 22.04 LTS (x86_64)
- 2 GB RAM minimum (1 GB will work, 2 GB recommended)
- SSH access as root or a sudo user

## Initial Setup

### 1. Create a dedicated service user

```bash
sudo useradd -r -m -s /bin/false clawbot
sudo mkdir -p /var/lib/clawbot
sudo chown clawbot:clawbot /var/lib/clawbot
```

### 2. Configure firewall (ufw)

Only expose SSH and the dashboard port:

```bash
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 443/tcp   # HTTPS (dashboard; add TLS termination via nginx if needed)
sudo ufw allow 8080/tcp  # Dashboard (direct; remove after adding nginx/TLS)
sudo ufw enable
```

### 3. Create the environment config directory

```bash
sudo mkdir -p /etc/clawbot
# The CD workflow will write /etc/clawbot/env automatically.
# For manual initial setup:
sudo tee /etc/clawbot/env > /dev/null <<'EOF'
BINANCE_API_KEY=your_key_here
BINANCE_SECRET=your_secret_here
TELEGRAM_TOKEN=your_bot_token_here
TELEGRAM_ALLOWED_USER_IDS=123456789
DASHBOARD_TOKEN=choose_a_strong_random_token
TRADING_MODE=paper
DATABASE_URL=sqlite:///var/lib/clawbot/clawbot.db
EOF
sudo chmod 600 /etc/clawbot/env
sudo chown root:root /etc/clawbot/env
```

### 4. Install the systemd service unit

```bash
sudo cp deploy/clawbot.service /etc/systemd/system/clawbot.service
sudo systemctl daemon-reload
sudo systemctl enable clawbot
```

### 5. First-time binary deploy (before CI/CD is set up)

```bash
scp target/x86_64-unknown-linux-musl/release/clawbot user@your-droplet:/usr/local/bin/clawbot
ssh user@your-droplet "chmod +x /usr/local/bin/clawbot"
sudo systemctl start clawbot
sudo systemctl status clawbot
```

## GitHub Secrets Required

Set these in your GitHub repository → Settings → Secrets → Actions:

| Secret | Description |
|---|---|
| `DO_SSH_PRIVATE_KEY` | Private key matching the Droplet's authorized_keys |
| `DROPLET_IP` | Public IP of the Droplet |
| `DROPLET_USER` | SSH user with sudo access |
| `BINANCE_API_KEY` | Binance API key |
| `BINANCE_SECRET` | Binance API secret |
| `TELEGRAM_TOKEN` | Telegram bot token |
| `TELEGRAM_ALLOWED_USER_IDS` | Comma-separated Telegram user IDs |
| `DASHBOARD_TOKEN` | Bearer token for dashboard auth |
| `TRADING_MODE` | `paper` or `live` |

## Rollback

If a deploy fails:
```bash
ssh user@your-droplet "sudo cp /usr/local/bin/clawbot.prev /usr/local/bin/clawbot && sudo systemctl restart clawbot"
```

## Logs

```bash
sudo journalctl -u clawbot -f
```
