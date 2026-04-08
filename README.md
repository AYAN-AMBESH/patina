# Patina

> `nm-connection-editor` power, over SSH, with the UX of a modern TUI.

A Rust rewrite of [nmtui](https://developer-old.gnome.org/NetworkManager/stable/nmtui.html) targeting full NetworkManager feature parity, live monitoring, and modern TUI ergonomics — all usable over SSH without a graphical environment.

## Why

No existing tool combines SSH-friendly operation, full NM configuration, modern UX, and live monitoring.

| | SSH | Full config | VPN | WPA-E | Revert timer | Live stats | Fuzzy search |
|---|---|---|---|---|---|---|---|
| nm-connection-editor | — | ✓ | ✓ | ✓ | — | — | — |
| GNOME Settings | — | partial | ✓ | — | ✓ | — | — |
| plasma-nm | — | ✓ | ✓ | ✓ | — | — | — |
| Cockpit | web | partial | — | — | ✓ | ✓ | — |
| nmtui | ✓ | partial | — | — | — | — | — |
| wifitui | ✓ | — | — | — | — | — | ✓ |
| nmcli | ✓ | ✓ | ✓ | ✓ | — | — | — |
| **patina++** | **✓** | **✓** | **✓** | **✓** | **✓** | **✓** | **✓** |

**Cockpit** is the closest competitor — remote-capable, live stats, revert safety — but requires a web server daemon, open port 9090, and a browser. Can't do VPN or WiFi config. For locked-down servers, it's overkill or unavailable.

**wifitui** proved demand for fuzzy search and modern TUI UX in network tooling, but dead-ends at WiFi connect — no configuration editing at all.

## Feature Roadmap

### Pillar 1 — Discovery & Search

- [ ] Fuzzy search across saved connections and visible SSIDs
- [ ] Manual WiFi rescan trigger (nmtui waits ~60s passively)
- [ ] Sort by signal strength, name, security type
- [ ] Filter by band (2.4 / 5 / 6 GHz)
- [ ] Hidden SSID connect
- [ ] Channel overlap visualization

### Pillar 2 — Monitoring & Stats

- [ ] Live signal strength sparkline (dBm over time)
- [ ] TX/RX throughput graph (from `/proc/net/dev` or `nl80211`)
- [ ] Latency / packet loss monitor (ICMP ping)
- [ ] Per-device status dashboard (`nmcli device show` data)
- [ ] Connection uptime and history log
- [ ] DNS resolution timing

### Pillar 3 — Configuration

Everything nmtui currently supports:

- [ ] Connection types: Ethernet, Wi-Fi, Bond, Bridge, Team, VLAN, InfiniBand
- [ ] Per-connection: profile name, device, cloned MAC, MTU
- [ ] WiFi: SSID, mode, BSSID, WPA/WPA2-PSK, password
- [ ] IPv4/IPv6: Automatic / Manual / Link-Local / Disabled
- [ ] IP detail: addresses, gateway, DNS servers, search domains
- [ ] Routing: static routes (dest/prefix, next-hop, metric), never-default, ignore-auto-dns
- [ ] Bond: mode, slaves, JSON config
- [ ] Bridge: STP, priority, forward delay, hello time, max age
- [ ] VLAN: parent device, VLAN ID
- [ ] Flags: auto-connect, available to all users

What nmtui is **missing** (nmcli / nm-connection-editor can do these):

- [ ] VPN profiles — WireGuard, OpenVPN, OpenConnect
- [ ] WPA Enterprise (EAP-TLS, PEAP, TTLS, FAST)
- [ ] 802.1X wired authentication
- [ ] IP Tunnel, MACsec connection types
- [ ] Firewall zone assignment
- [ ] Connection priority and metered hint
- [ ] Proxy settings (PAC / manual)
- [ ] DNS-over-TLS / DNSSEC options
- [ ] Wake-on-LAN settings
- [ ] WiFi power saving / scan interval
- [ ] MAC randomization (per-scan, per-connect)
- [ ] Autoconnect retries and priority ordering
- [ ] Connection secondaries / dependencies
- [ ] Import / export connection profiles

### Pillar 4 — UX & Safety

**Simplification — Presets:**

- [ ] "Home WiFi" — WPA2-PSK, DHCP, auto-DNS
- [ ] "Server" — static IP, no WiFi, jumbo MTU
- [ ] "Corporate" — WPA-Enterprise + proxy + VPN auto-trigger
- [ ] "Privacy" — MAC randomization + DNS-over-TLS
- [ ] "Hotspot" — AP mode, bridge, DHCP server

**Smart completions:**

- [ ] Auto-detect subnet from gateway (CIDR calculator)
- [ ] DNS autocomplete — dropdown of popular resolvers (1.1.1.1, 8.8.8.8, 9.9.9.9)
- [ ] Certificate file picker (TUI file browser) for EAP / VPN
- [ ] SSID autocomplete from live scan results
- [ ] IP conflict detection before apply

**Visual aids:**

- [ ] Live validation (red/green) on IP and CIDR input fields
- [ ] Diff view — pending changes vs active running config
- [ ] Topology preview for complex setups (bond → bridge → VLAN tree)
- [ ] Revert timer — auto-rollback in N seconds if user doesn't confirm (critical for remote SSH config; inspired by GNOME Settings' 15s countdown, but SSH-accessible)

**Navigation:**

- [ ] Vim-style keybinds (`j`/`k`/`g`/`G`, `/` for search)
- [ ] Radio toggle panel (WiFi / WWAN / Bluetooth on/off)
- [ ] Inline password reveal + WiFi QR code generation

## Architecture Notes

```
┌─────────────┐      ┌──────────────┐      ┌────────────┐
│  TUI layer  │◄────►│  App state   │◄────►│ NM backend │
│  (ratatui)  │      │ (os thread)  │      │            │
└─────────────┘      └──────────────┘      └────────────┘
                                            │          │
                                     ┌──────┘          └──────┐
                                     ▼                        ▼
                              ┌────────────┐          ┌─────────────┐
                              │ D-Bus      │          │ nmcli       │
                              │ (zbus)     │          │ (fallback)  │
                              │ preferred  │          │ subprocess  │
                              └────────────┘          └─────────────┘
```

- **TUI**: `ratatui` — actively maintained, good widget ecosystem
- **Async runtime**: `Os Thread` — drives polling loop, D-Bus subscriptions, ping probes
- **NM backend (preferred)**: `zbus` for typed async D-Bus calls to `org.freedesktop.NetworkManager`
- **NM backend (fallback)**: `nmcli` subprocess parsing for environments where D-Bus is restricted
- **Fuzzy matching**: `nucleo` (same engine as Helix editor)

## Prior Art

| Tool | Language | What it does well | What it lacks |
|---|---|---|---|
| [nmtui](https://developer-old.gnome.org/NetworkManager/stable/nmtui.html) | C (newt) | Decent config UI for basic types | No VPN, no WPA-E, no search, no stats, no validation |
| [wifitui](https://github.com/shazow/wifitui) | Go | Fuzzy search, QR, rescan, multi-backend | Zero configuration editing |
| [impala](https://github.com/pythops/impala) | Rust | Clean Rust TUI, signal bars | iwd-only, no NM, no config |
| [Cockpit](https://cockpit-project.org/) | JS | Web UI, live graphs, revert safety | Requires daemon + browser, no VPN, no WiFi |
| [nm-connection-editor](https://wiki.gnome.org/Projects/NetworkManager) | C (GTK) | Full NM feature parity | Needs X11/Wayland |
| [wavemon](https://github.com/uoaerg/wavemon) | C (ncurses) | Real-time WiFi signal monitoring | No config, WiFi only |

## License

TBD
