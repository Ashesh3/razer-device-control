mod audio;
mod device;
mod protocol;
mod registry;

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[cfg(windows)]
extern "system" {
    fn FreeConsole() -> i32;
    fn CreateMutexW(attrs: *mut u8, initial_owner: i32, name: *const u16) -> *mut u8;
    fn GetLastError() -> u32;
}

const _ERROR_ALREADY_EXISTS: u32 = 183;

/// Ensure only one instance of rzr --watch is running.
/// Returns false if another instance already holds the mutex.
#[cfg(windows)]
fn acquire_single_instance() -> bool {
    let name: Vec<u16> = "Global\\rzr_blackshark_v2_pro\0"
        .encode_utf16()
        .collect();
    unsafe {
        let handle = CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
        if handle.is_null() || GetLastError() == _ERROR_ALREADY_EXISTS {
            return false;
        }
    }
    // Leak the handle — lives for process lifetime
    true
}

#[cfg(not(windows))]
fn acquire_single_instance() -> bool {
    true
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let silent = args.iter().any(|a| a == "--silent" || a == "-s");

    if silent {
        // Hide console window on Windows
        #[cfg(windows)]
        unsafe { FreeConsole(); }

        if args.iter().any(|a| a == "--watch" || a == "-w") {
            run_watch(true);
        } else {
            run_silent();
        }
        return;
    }

    let cmd = args.iter().find(|a| !a.starts_with('-') && *a != &args[0]);

    match cmd.map(|s| s.as_str()) {
        Some("config") => run_config(),
        Some("help") => print_help(),
        Some("watch") => run_watch(false),
        None => {
            if args.iter().any(|a| a == "--help" || a == "-h") {
                print_help();
            } else if args.iter().any(|a| a == "--watch" || a == "-w") {
                run_watch(false);
            } else {
                run_apply();
            }
        }
        Some(other) => {
            eprintln!("Unknown command: {other}");
            eprintln!("Use 'rzr config' to configure, or just 'rzr' to apply settings.");
            std::process::exit(1);
        }
    }
}


fn print_help() {
    println!("rzr - Razer BlackShark V2 Pro Audio Initializer");
    println!();
    println!("Sends audio configuration (EQ, volume, enhancement) to your headset");
    println!("via USB HID, replicating what Razer Synapse does on profile switch.");
    println!();
    println!("Usage:");
    println!("  rzr              Apply saved settings to headset");
    println!("  rzr config       Interactive configuration menu");
    println!("  rzr --watch      Watch for headset and apply on connect");
    println!("  rzr --silent     Apply silently (no window, for startup)");
    println!("  rzr --silent --watch  Watch silently (best for startup)");
    println!("  rzr --help       Show this help");
    println!();
    println!("Settings are stored in the Windows Registry at HKCU\\SOFTWARE\\rzr");
    println!();
    println!("Tip: Add rzr.exe to shell:startup to run automatically on login.");
}

/// Set default audio devices if configured.
fn apply_audio_defaults(settings: &registry::Settings, silent: bool) {
    if !settings.default_speaker.is_empty() {
        if !silent {
            print!("  Setting default speaker...");
            io::stdout().flush().ok();
        }
        if audio::set_default_device(&settings.default_speaker) {
            if !silent { println!(" done"); }
        } else if !silent {
            println!(" failed");
        }
    }
    if !settings.default_microphone.is_empty() {
        if !silent {
            print!("  Setting default microphone...");
            io::stdout().flush().ok();
        }
        if audio::set_default_device(&settings.default_microphone) {
            if !silent { println!(" done"); }
        } else if !silent {
            println!(" failed");
        }
    }
}

fn run_silent() {
    let settings = registry::Settings::load();
    let dev = match device::Device::open(settings.wait_timeout_ms) {
        Ok(d) => d,
        Err(_) => std::process::exit(1),
    };
    let _ = dev.apply_profile(
        settings.eq_enabled,
        &settings.eq_bands,
        settings.volume,
        settings.enhancement,
    );
    apply_audio_defaults(&settings, true);
}

fn run_watch(silent: bool) {
    if !acquire_single_instance() {
        if !silent {
            eprintln!("rzr: another instance is already running.");
        }
        std::process::exit(0);
    }

    let settings = registry::Settings::load();
    let poll_interval = Duration::from_secs(5);
    let mut was_connected = false;
    let mut applied = false;

    if !silent {
        println!("rzr: watching for headset (poll every 5s, Ctrl+C to stop)");
    }

    loop {
        let connected = match device::Device::open(1000) {
            Ok(dev) => {
                let c = dev.is_headset_connected();
                if c && !applied {
                    // Headset just connected (or first detection)
                    if !silent {
                        println!("  Headset connected, applying profile...");
                    }
                    let _ = dev.apply_profile(
                        settings.eq_enabled,
                        &settings.eq_bands,
                        settings.volume,
                        settings.enhancement,
                    );
                    if !silent {
                        if let Some(batt) = dev.get_battery() {
                            println!("  Battery: {}%", batt);
                        }
                        println!("  Done!");
                    }
                    apply_audio_defaults(&settings, silent);
                    applied = true;
                }
                c
            }
            Err(_) => false,
        };

        if was_connected && !connected {
            // Headset disconnected — reset so we re-apply on next connect
            if !silent {
                println!("  Headset disconnected, waiting for reconnect...");
            }
            applied = false;
        }

        was_connected = connected;
        thread::sleep(poll_interval);
    }
}

fn run_apply() {
    let settings = registry::Settings::load();

    print!("rzr: waiting for device...");
    io::stdout().flush().ok();

    let dev = match device::Device::open(settings.wait_timeout_ms) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("\nError: {e}");
            std::process::exit(1);
        }
    };

    println!(" found {}", dev.product);

    if let Some(batt) = dev.get_battery() {
        println!("  Battery: {}%", batt);
    }

    print!("  Applying profile...");
    io::stdout().flush().ok();

    match dev.apply_profile(
        settings.eq_enabled,
        &settings.eq_bands,
        settings.volume,
        settings.enhancement,
    ) {
        Ok(()) => println!(" done!"),
        Err(e) => {
            eprintln!(" failed: {e}");
            std::process::exit(1);
        }
    }

    apply_audio_defaults(&settings, false);
}

fn run_config() {
    let mut settings = registry::Settings::load();

    // Show battery on entry
    if let Ok(dev) = device::Device::open(2000) {
        if let Some(batt) = dev.get_battery() {
            println!("\n  Device: {}", dev.product);
            println!("  Battery: {}%", batt);
        }
    }

    loop {
        println!();
        println!("rzr - Configuration");
        println!("===================");
        println!();
        println!(
            "  1. EQ Enabled:     {}",
            if settings.eq_enabled { "yes" } else { "no" }
        );
        println!(
            "  2. EQ Bands:       [{}]",
            registry::format_eq_bands(&settings.eq_bands)
        );
        println!("  3. Volume:         {}", settings.volume);
        println!(
            "  4. Enhancement:    {}",
            if settings.enhancement { "yes" } else { "no" }
        );
        println!("  5. Wait Timeout:   {}ms", settings.wait_timeout_ms);
        println!();
        let spk_display = if settings.default_speaker.is_empty() {
            "disabled".to_string()
        } else {
            // Try to find name for stored ID
            let devs = audio::list_devices("render");
            devs.iter()
                .find(|d| d.id == settings.default_speaker)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| settings.default_speaker.clone())
        };
        let mic_display = if settings.default_microphone.is_empty() {
            "disabled".to_string()
        } else {
            let devs = audio::list_devices("capture");
            devs.iter()
                .find(|d| d.id == settings.default_microphone)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| settings.default_microphone.clone())
        };
        println!("  s. Default Speaker:  {}", spk_display);
        println!("  m. Default Mic:      {}", mic_display);
        println!();
        println!("  Presets:");
        println!("  6. Load preset: flat");
        println!("  7. Load preset: game");
        println!("  8. Load preset: music");
        println!("  9. Load preset: movie");
        println!();
        println!("  a. Apply now (send to headset)");
        println!("  0. Save & exit");
        println!("  q. Quit without saving");
        println!();
        print!("  Select: ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let choice = input.trim();

        match choice {
            "1" => {
                settings.eq_enabled = !settings.eq_enabled;
                println!(
                    "  EQ {}",
                    if settings.eq_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            "2" => {
                print!("  Enter 10 comma-separated values (-5 to 5): ");
                io::stdout().flush().ok();
                let mut buf = String::new();
                io::stdin().read_line(&mut buf).ok();
                match registry::parse_eq_bands(buf.trim()) {
                    Some(bands) => {
                        settings.eq_bands = bands;
                        println!("  EQ set to [{}]", registry::format_eq_bands(&bands));
                    }
                    None => println!("  Invalid input. Need exactly 10 values."),
                }
            }
            "3" => {
                print!("  Enter volume (0-255): ");
                io::stdout().flush().ok();
                let mut buf = String::new();
                io::stdin().read_line(&mut buf).ok();
                match buf.trim().parse::<u32>() {
                    Ok(v) if v <= 255 => {
                        settings.volume = v as u8;
                        println!("  Volume set to {}", v);
                    }
                    _ => println!("  Invalid. Enter 0-255."),
                }
            }
            "4" => {
                settings.enhancement = !settings.enhancement;
                println!(
                    "  Enhancement {}",
                    if settings.enhancement {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            "5" => {
                print!("  Enter timeout in ms: ");
                io::stdout().flush().ok();
                let mut buf = String::new();
                io::stdin().read_line(&mut buf).ok();
                match buf.trim().parse::<u32>() {
                    Ok(v) => {
                        settings.wait_timeout_ms = v;
                        println!("  Timeout set to {}ms", v);
                    }
                    _ => println!("  Invalid number."),
                }
            }
            "6" => {
                settings.eq_bands = registry::PRESET_FLAT;
                println!("  Loaded flat preset");
            }
            "7" => {
                settings.eq_bands = registry::PRESET_GAME;
                println!("  Loaded game preset");
            }
            "8" => {
                settings.eq_bands = registry::PRESET_MUSIC;
                println!("  Loaded music preset");
            }
            "9" => {
                settings.eq_bands = registry::PRESET_MOVIE;
                println!("  Loaded movie preset");
            }
            "a" | "A" => {
                println!("  Connecting to headset...");
                match device::Device::open(settings.wait_timeout_ms) {
                    Ok(dev) => {
                        match dev.apply_profile(
                            settings.eq_enabled,
                            &settings.eq_bands,
                            settings.volume,
                            settings.enhancement,
                        ) {
                            Ok(()) => println!("  Applied successfully!"),
                            Err(e) => println!("  Failed: {e}"),
                        }
                    }
                    Err(e) => println!("  {e}"),
                }
                apply_audio_defaults(&settings, false);
            }
            "s" | "S" => {
                let devices = audio::list_devices("render");
                if devices.is_empty() {
                    println!("  No speakers found.");
                } else {
                    println!("  Available speakers:");
                    println!("    0. Disable (don't change default)");
                    for (i, d) in devices.iter().enumerate() {
                        let marker = if d.is_default { " [current default]" } else { "" };
                        println!("    {}. {}{}", i + 1, d.name, marker);
                    }
                    print!("  Select: ");
                    io::stdout().flush().ok();
                    let mut buf = String::new();
                    io::stdin().read_line(&mut buf).ok();
                    match buf.trim().parse::<usize>() {
                        Ok(0) => {
                            settings.default_speaker = String::new();
                            println!("  Default speaker override disabled.");
                        }
                        Ok(n) if n <= devices.len() => {
                            settings.default_speaker = devices[n - 1].id.clone();
                            println!("  Will set default to: {}", devices[n - 1].name);
                        }
                        _ => println!("  Invalid selection."),
                    }
                }
            }
            "m" | "M" => {
                let devices = audio::list_devices("capture");
                if devices.is_empty() {
                    println!("  No microphones found.");
                } else {
                    println!("  Available microphones:");
                    println!("    0. Disable (don't change default)");
                    for (i, d) in devices.iter().enumerate() {
                        let marker = if d.is_default { " [current default]" } else { "" };
                        println!("    {}. {}{}", i + 1, d.name, marker);
                    }
                    print!("  Select: ");
                    io::stdout().flush().ok();
                    let mut buf = String::new();
                    io::stdin().read_line(&mut buf).ok();
                    match buf.trim().parse::<usize>() {
                        Ok(0) => {
                            settings.default_microphone = String::new();
                            println!("  Default mic override disabled.");
                        }
                        Ok(n) if n <= devices.len() => {
                            settings.default_microphone = devices[n - 1].id.clone();
                            println!("  Will set default to: {}", devices[n - 1].name);
                        }
                        _ => println!("  Invalid selection."),
                    }
                }
            }
            "0" => {
                match settings.save() {
                    Ok(()) => println!("  Settings saved to registry."),
                    Err(e) => eprintln!("  Error saving: {e}"),
                }
                break;
            }
            "q" | "Q" => {
                println!("  Exiting without saving.");
                break;
            }
            _ => println!("  Invalid option."),
        }
    }
}
