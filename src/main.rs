use clap::Parser;
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use proc_mounts::MountIter;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::{fs, thread, time};

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
}

fn all_mounted(cmd: &String, dry_run: bool) {
    // TODO: actually do something useful with this, call configured safe to start cmd?
    println!("All NFS mounts are available");
    if !dry_run {
        println!("Running command: {}", cmd);
    }
}

fn any_unmounted(cmd: &String, dry_run: bool) {
    // TODO: actually do something useful with this, call configured must stop cmd?
    println!("One or more NFS mounts are disconnected");
    if !dry_run {
        println!("Running command: {}", cmd);
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
    let cli = Cli::parse();

    // Load configuration
    let config_content = fs::read_to_string("config.yml")?;
    let config: Config = match serde_yaml::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => panic!("Failed to parse configuration: {}", e),
    };

    // Initialize inotify
    let mut inotify = Inotify::init()?;
    let mut watches: HashMap<String, WatchDescriptor> = HashMap::new();

    // Check initial state and set up watches
    let mut current_state = true;
    for path in &config.mount_points {
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
        println!("== Dry run enabled, no commands will be executed. ==");
    }

    // Execute on initial state
    print!("Initial state: ");
    if current_state {
        all_mounted(&config.all_mounted_cmd, cli.dry_run);
    } else {
        any_unmounted(&config.any_unmounted_cmd, cli.dry_run);
    }

    // Loop for observation of watchers
    println!(
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
        println!("Processed events in {}ms", elapsed.as_millis());

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
