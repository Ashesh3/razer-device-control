mod device;
mod protocol;
mod registry;

use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "config" => run_config(),
            "--help" | "-h" | "help" => print_help(),
            other => {
                eprintln!("Unknown command: {other}");
                eprintln!("Use 'rzr config' to configure, or just 'rzr' to apply settings.");
                std::process::exit(1);
            }
        }
    } else {
        run_apply();
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
    println!("  rzr --help       Show this help");
    println!();
    println!("Settings are stored in the Windows Registry at HKCU\\SOFTWARE\\rzr");
    println!();
    println!("Tip: Add rzr.exe to shell:startup to run automatically on login.");
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
}

fn run_config() {
    let mut settings = registry::Settings::load();

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
