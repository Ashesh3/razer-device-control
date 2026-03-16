# rzr

Tiny portable tool that fixes audio quality on the **Razer BlackShark V2 Pro** wireless headset without needing Razer Synapse.

<img width="479" height="497" alt="image" src="https://github.com/user-attachments/assets/d77e0d6b-d910-46f5-be87-72de9e18a1c0" />


## The Problem

When you connect your BlackShark V2 Pro and switch to a custom profile, the audio sounds quiet and flat. Starting Razer Synapse and switching profiles magically fixes it — audio becomes louder, clearer, and the EQ actually works.

This happens because the headset's onboard presets are basic factory defaults. Synapse pushes the real audio configuration (EQ bands, volume gain, DSP enhancement) to the headset over USB HID every time you switch profiles. Without Synapse, those commands never get sent.

**rzr** sends the exact same USB commands, in a 201KB exe with zero dependencies.

## Usage

```
rzr              # Apply saved audio profile to headset
rzr config       # Interactive configuration menu
rzr --help       # Show help
```

### First Run

Just run `rzr.exe`. It uses sensible defaults (custom EQ, max volume, enhancement on). Settings are saved to the Windows Registry at `HKCU\SOFTWARE\rzr`.

### Configuration

Run `rzr config` to customize:

```
rzr - Configuration
===================

  1. EQ Enabled:     yes
  2. EQ Bands:       [1,-2,1,-3,1,-3,-5,2,2,3]
  3. Volume:         255
  4. Enhancement:    yes
  5. Wait Timeout:   5000ms

  Presets:
  6. Load preset: flat
  7. Load preset: game
  8. Load preset: music
  9. Load preset: movie

  a. Apply now (send to headset)
  0. Save & exit
  q. Quit without saving
```

### Auto-Start on Login

Place `rzr.exe` (or a shortcut to it) in `shell:startup` (press Win+R, type `shell:startup`). It will apply your profile every time you log in.

## How It Works

Through reverse engineering Razer Synapse 4, we discovered the exact USB HID commands sent to the headset's wireless dongle when switching audio profiles.

### Protocol

The BlackShark V2 Pro dongle exposes a vendor-defined HID endpoint on USB Interface 3 (Usage Page `0xFF00`). Synapse communicates via 64-byte interrupt OUT transfers using what Razer internally calls the "Audio MXIC" protocol:

```
Byte 0:    0x02        Report type
Byte 1:    0x80        Direction (output)
Byte 2:    total_len   Total payload length
Byte 5-6:  0x50 0x41   "PA" (Protocol Audio)
Byte 7:    inner_len   Inner data length
Byte 9:    cmd_type    Command type (2=set, 3=get, 4=set+ack, 6=config, 13=bulk)
Byte 10:   cmd_id      Command identifier
Byte 11+:  params      Command-specific parameters
```

### Commands Sent

When you switch profiles in Synapse, it sends this exact sequence:

| Step | Command | ID | Description |
|------|---------|----|-------------|
| 1 | SET_CONFIG | `0x06/0x01` | Configures the audio DSP pipeline |
| 2 | setSpeakerPresetEQ | `0x04/0x9E` | Enables the EQ processing engine |
| 3 | setVolume | `0x04/0x93` | Sets internal DSP gain (0-255) |
| 4 | setEnhancement | `0x04/0x9D` | Enables audio enhancement |
| 5 | SET_EQ_BANDS | `0x0D/0x95` | Pushes 10-band EQ (signed bytes) |

Each command is preceded by a `setRemoteMode` (`0x02/0xE1`) call that toggles between software and device control. The sequence is sent twice for reliability (matching Synapse's behavior).

### EQ Presets

| Preset | Bands |
|--------|-------|
| flat | `0,0,0,0,0,0,0,0,0,0` |
| game | `-3,-3,-4,0,5,5,4,1,0,-1` |
| music | `2,2,1,1,2,3,3,3,1,0` |
| movie | `4,4,3,0,-3,-1,3,5,2,1` |

## Building

```
cargo build --release
```

The binary is at `target/release/rzr.exe` (~200KB, statically linked).

### Dependencies

- [hidapi](https://crates.io/crates/hidapi) — USB HID communication
- [winreg](https://crates.io/crates/winreg) — Windows Registry access

## Reverse Engineering Process

This tool was built by reverse engineering Razer Synapse 4:

1. **Extracted the Electron app** (`app.asar`) to find the JavaScript source code for device communication
2. **Analyzed Synapse logs** at `%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Logs\` which contain every HID command with full byte dumps
3. **Captured USB traffic** with USBPcap while Synapse initialized the headset
4. **Decoded the MXIC protocol** by correlating log entries (`AudioMxicDevice.sendCommandOut()`) with USB packets
5. **Identified the key insight**: audio quality isn't controlled by "device mode" — it's the EQ, volume gain, and enhancement commands pushed during profile switches

### Device Info

| Property | Value |
|----------|-------|
| Vendor ID | `0x1532` (Razer) |
| Product ID | `0x0555` |
| USB Interface | 3 |
| HID Usage Page | `0xFF00` (vendor-defined) |
| HID Usage | `0x01` |
| Protocol | Audio MXIC (64-byte interrupt transfers) |

## License

MIT
