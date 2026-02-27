# sc4n

Stealthy adaptive network port scanner built in Rust. Designed for penetration testers and bug bounty hunters.

> **NOTE: This tool is intended for use on networks and systems you have explicit authorization to test. Unauthorized scanning is illegal and unethical. Always obtain proper written permission before scanning any target.**

## Installation

```bash
# Clone the repository
git clone <repo-url>
cd sc4n

# Build optimized release binary
cargo build --release

# Binary will be at target/release/sc4n
```

## Usage

```bash
# Basic scan with default balanced profile (ports 1-1024)
./target/release/sc4n -H 192.168.1.1

# Scan specific ports
./target/release/sc4n -H 192.168.1.1 -p 22,80,443,8080

# Scan a port range
./target/release/sc4n -H 192.168.1.1 -p 1-65535

# Mixed port specification
./target/release/sc4n -H 192.168.1.1 -p 22,80,443,8000-9000
```

### Profile Examples

```bash
# Aggressive — maximum speed, use on your own infrastructure only
./target/release/sc4n -H 10.0.0.1 -p 1-65535 -P aggressive

# Balanced — default profile, good for authorized assessments
./target/release/sc4n -H target.example.com -p 1-1024 -P balanced

# Stealth — low and slow, designed to avoid IDS detection
./target/release/sc4n -H target.example.com -p 1-1024 -P stealth

# Paranoid — one probe at a time with random long delays, maximum stealth
./target/release/sc4n -H target.example.com -p 22,80,443 -P paranoid
```

### Override Profile Settings

```bash
# Use stealth profile but override concurrency and rate
./target/release/sc4n -H target.example.com -p 1-1024 -P stealth -c 50 -r 100

# Custom timeout per probe (milliseconds)
./target/release/sc4n -H target.example.com -p 1-1024 -t 3000

# Disable port randomization
./target/release/sc4n -H target.example.com -p 1-1024 --no-randomize

# Quiet mode — suppress banner, print results only
./target/release/sc4n -H target.example.com -p 1-1024 -q

# Debug mode — show all ports including closed
./target/release/sc4n -H target.example.com -p 22,80,443 -d

# Custom output file
./target/release/sc4n -H target.example.com -p 1-1024 -o results.jsonl
```

## Flag Reference

| Flag | Long | Description | Default |
|------|------|-------------|---------|
| `-H` | `--host` | Target host or IP address | *(required)* |
| `-p` | `--ports` | Port range (e.g. `1-1024`, `80`, `22,80,443`) | `1-1024` |
| `-P` | `--profile` | Scan profile: `aggressive`, `balanced`, `stealth`, `paranoid` | `balanced` |
| `-c` | `--concurrency` | Number of concurrent probes (overrides profile) | *(profile default)* |
| `-r` | `--rate` | Requests per second limit, 0 = unlimited (overrides profile) | *(profile default)* |
| `-o` | `--output` | Output file path | `sc4n_results.jsonl` |
| `-t` | `--timeout-ms` | Timeout per probe in milliseconds (overrides profile) | *(profile default)* |
| `-d` | `--debug` | Show all ports including closed | `false` |
| | `--randomize` | Randomize port scan order | `true` |
| `-q` | `--quiet` | Suppress banner, print results only | `false` |

## Scan Profiles

| Profile | Concurrency | Rate/sec | Timeout | Jitter | Description |
|---------|-------------|----------|---------|--------|-------------|
| `aggressive` | 1000 | unlimited | 500ms | 0-10ms | Maximum speed. Own infrastructure only. |
| `balanced` | 200 | 500 | 1000ms | 5-50ms | Default. Good for authorized assessments. |
| `stealth` | 10 | 20 | 2000ms | 100-500ms | Low and slow. Avoids IDS detection. |
| `paranoid` | 1 | 1 | 5000ms | 10-60s | One probe at a time. Maximum stealth. |

## Output Format (JSONL)

Results are written as newline-delimited JSON (one JSON object per line):

```json
{"host":"192.168.1.1","port":22,"status":"open","latency_ms":3}
{"host":"192.168.1.1","port":80,"status":"open","latency_ms":5}
{"host":"192.168.1.1","port":443,"status":"closed","latency_ms":0}
{"host":"192.168.1.1","port":8080,"status":"filtered","latency_ms":1000}
```

### Schema

| Field | Type | Description |
|-------|------|-------------|
| `host` | string | Target host or IP address |
| `port` | integer | Port number (1-65535) |
| `status` | string | One of: `open`, `closed`, `filtered` |
| `latency_ms` | integer | Connection latency in milliseconds |

### Port Status Meanings

- **open** — TCP connection succeeded
- **closed** — TCP connection refused (RST received)
- **filtered** — Connection timed out (no response, likely firewalled)

## License

MIT
