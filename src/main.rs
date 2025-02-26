use clap::Parser;
use env_logger::Env;
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use log::{debug, error, info, warn};
use proc_mounts::MountIter;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::{fs, process, thread, time};

#[derive(Debug, Deserialize)]
struct Config {
    mount_points: Vec<String>,
    delay_seconds: u64,
    all_mounted_cmd: String,
    any_unmounted_cmd: String,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long, short, action)]
    dry_run: bool,
    #[clap(long, short, action)]
    verbose: bool,
}

fn all_mounted(cmd: &String, dry_run: bool) {
    info!("All NFS mounts are available");
    if !dry_run {
        debug!("Running command: {}", cmd);
        // TODO: actually do something useful with this, call configured safe to start cmd?
    }
}

fn any_unmounted(cmd: &String, dry_run: bool) {
    error!("One or more NFS mounts are disconnected!!");
    if !dry_run {
        debug!("Running command: {}", cmd);
        // TODO: actually do something useful with this, call configured must stop cmd?
    }
}

fn is_mount_point(path: &str) -> bool {
    // Get the systems mount points from /proc/mounts
    let Ok(canonical_path) = PathBuf::from(path).canonicalize() else {
        return false;
    };
    let mounts = match MountIter::new() {
        Ok(m) => m,
        Err(_) => return false,
    };

    // Filter for the matching path.
    mounts
        .filter_map(Result::ok)
        .filter_map(|m| m.dest.canonicalize().ok())
        .any(|p| p == canonical_path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get CLI config
    let cli = Cli::parse();

    // Configure the logger
    let mut builder = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    if cli.verbose {
        builder.filter_level(log::LevelFilter::Trace);
    }
    builder.init();

    // Load configuration
    // Path to the configuration file should default to $HOME/.config/nofus/config.yml
    let config_path =
        PathBuf::from(std::env::var("HOME").unwrap()).join(".config/nofus/config.yml");
    // If the directory doesn't exist, create it
    if !config_path.parent().unwrap().exists() {
        debug!("Creating config directory");
        fs::create_dir_all(config_path.parent().unwrap())?;
    }
    // If the config file doesn't exist, create it
    if !config_path.exists() {
        warn!(
            "Creating a default config file at {}, you'll want to edit it.",
            config_path.display()
        );
        let default_config = r#"mount_points:
  - /mnt/hostname/share/
delay_seconds: 5
all_mounted_cmd: echo "All clear!"
any_unmounted_cmd: echo "Very bad!"#;
        fs::write(config_path, default_config)?;
        process::exit(1) // Just exit because they really should update that...
    }
    let config_content = fs::read_to_string("config.yml")?;
    let config: Config = match serde_yml::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => panic!("Failed to parse configuration: {}", e),
    };

    // Initialize inotify
    let mut inotify = Inotify::init()?;
    let mut watches: HashMap<String, WatchDescriptor> = HashMap::new();

    // Check initial state and set up watches
    let mut current_state = true;
    for path in &config.mount_points {
        info!("Monitoring mount point: {}", path);
        //  Check state and setup watch
        if is_mount_point(path) {
            if let Ok(watch) = inotify.watches().add(path, WatchMask::ALL_EVENTS) {
                watches.insert(path.clone(), watch);
            }
        } else {
            current_state = false;
        }
    }

    // Notify if dry run
    if cli.dry_run {
        warn!("== Dry run enabled, no commands will be executed. ==");
    }

    // Execute on initial state
    info!("Initial state: ");
    if current_state {
        all_mounted(&config.all_mounted_cmd, cli.dry_run);
    } else {
        any_unmounted(&config.any_unmounted_cmd, cli.dry_run);
    }

    // Loop for observation of watchers
    debug!(
        "Starting observation loop ({} second delay)...",
        config.delay_seconds
    );

    let mut buffer = [0; 4096];
    loop {
        // Benchmark the timing
        let start_time = time::Instant::now();

        // Process inotify events
        let mut events = Vec::new();
        match inotify.read_events(&mut buffer) {
            Ok(read_events) => read_events.for_each(|event| events.push(event)),
            Err(error) if error.kind() == ErrorKind::WouldBlock => continue,
            _ => panic!("Error while reading events"),
        }

        for event in events {
            if event.mask.contains(EventMask::IGNORED) {
                // Remove invalidated watches
                let path = watches
                    .iter()
                    .find(|(_, wd)| **wd == event.wd)
                    .map(|(p, _)| p.clone());

                if let Some(path) = path {
                    watches.remove(&path);
                }
            }
        }

        let mut new_state = true;
        let mut state_changed = false;

        // Update watches and check mount status
        for path in &config.mount_points {
            let is_mounted = is_mount_point(path);

            // Update watches
            if is_mounted && !watches.contains_key(path) {
                if let Ok(watch) = inotify.watches().add(path, WatchMask::ALL_EVENTS) {
                    watches.insert(path.clone(), watch);
                }
            }

            // Update state
            if !is_mounted {
                new_state = false;
            }
        }

        // Check if state changed
        if new_state != current_state {
            state_changed = true;
            current_state = new_state;
        }

        // Job done, how long did it take?
        let elapsed = start_time.elapsed();
        debug!("Processed events in {}ms", elapsed.as_millis());

        // Trigger appropriate function if state changed
        if state_changed {
            if current_state {
                all_mounted(&config.all_mounted_cmd, cli.dry_run);
            } else {
                any_unmounted(&config.any_unmounted_cmd, cli.dry_run);
            }
        }

        // Periodic check every 5 seconds
        thread::sleep(time::Duration::from_secs(config.delay_seconds));
    }
}
