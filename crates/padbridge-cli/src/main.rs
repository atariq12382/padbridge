use std::fs;
use std::fs::OpenOptions;
use std::path::Path;
use std::process::Command as ProcessCommand;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about = "Control the padbridge daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    Shutdown,
    Doctor,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Status => println!("padbridge daemon status is not implemented yet"),
        Command::Shutdown => println!("padbridge daemon shutdown is not implemented yet"),
        Command::Doctor => {
            if !run_doctor() {
                std::process::exit(1);
            }
        }
    }
}

fn run_doctor() -> bool {
    let kernel_release = fs::read_to_string("/proc/sys/kernel/osrelease").ok();
    let kernel_version = kernel_release.as_deref().and_then(parse_kernel_version);
    let kernel_ok = kernel_version.is_some_and(|version| version >= (5, 12));
    let kernel_detail = match (kernel_release.as_deref(), kernel_version) {
        (Some(release), Some(_)) if kernel_ok => {
            format!("running kernel {release} meets the 5.12 minimum")
        }
        (Some(release), Some(_)) => format!("running kernel {release} is older than 5.12"),
        _ => "could not determine the running kernel version".to_string(),
    };
    let mut passed = report_check(
        "kernel version",
        kernel_ok,
        &kernel_detail,
        "Boot a Linux kernel version 5.12 or newer.",
    );

    let hid_available = Path::new("/sys/module/hid_playstation").exists()
        || ProcessCommand::new("modprobe")
            .args(["--dry-run", "--quiet", "hid_playstation"])
            .status()
            .is_ok_and(|status| status.success());
    passed &= report_check(
        "hid_playstation module",
        hid_available,
        if hid_available {
            "module is loaded or available"
        } else {
            "module is neither loaded nor available"
        },
        "Use a kernel build that provides hid_playstation, then load it with `sudo modprobe hid_playstation`.",
    );

    let uinput_path = Path::new("/dev/uinput");
    let uinput_exists = uinput_path.exists();
    let uinput_writable = uinput_exists && OpenOptions::new().write(true).open(uinput_path).is_ok();
    let uinput_detail = match (uinput_exists, uinput_writable) {
        (false, _) => "/dev/uinput does not exist",
        (true, false) => "/dev/uinput exists but is not writable by this user",
        (true, true) => "/dev/uinput exists and is writable",
    };
    let uinput_fix = if uinput_exists {
        "Grant this user write access to /dev/uinput through the input group or a udev uaccess rule, then log in again."
    } else {
        "Load it now with `sudo modprobe uinput`; to persist this, put `uinput` in `/etc/modules-load.d/uinput.conf`."
    };
    passed &= report_check("/dev/uinput", uinput_writable, uinput_detail, uinput_fix);

    let input_group = is_in_input_group();
    let uaccess = has_input_device_access();
    let user_access = input_group || uaccess;
    let access_detail = match (input_group, uaccess) {
        (true, _) => "current user belongs to the input group",
        (false, true) => {
            "current user can access an input device through its permissions or uaccess ACL"
        }
        (false, false) => {
            "current user is not in the input group and has no accessible input device"
        }
    };
    passed &= report_check(
        "input user access",
        user_access,
        access_detail,
        "Add the user to the `input` group or configure a udev `TAG+=\"uaccess\"` rule; then start a new login session.",
    );

    let udev_rules = has_padbridge_udev_rule();
    passed &= report_check(
        "udev rules",
        udev_rules,
        if udev_rules {
            "a padbridge udev rule is installed"
        } else {
            "no padbridge udev rule was found in the standard rules directories"
        },
        "Install a padbridge `.rules` file in a standard udev rules directory, reload rules with `sudo udevadm control --reload-rules`, and reconnect devices.",
    );

    let init_system = detect_init_system();
    passed &= report_check("init system", true, &format!("detected {init_system}"), "");

    passed
}

fn parse_kernel_version(release: &str) -> Option<(u32, u32)> {
    let mut components = release.split('.');
    let major = components.next()?.parse().ok()?;
    let minor = components
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()?;
    Some((major, minor))
}

fn is_in_input_group() -> bool {
    let group_id = fs::read_to_string("/etc/group").ok().and_then(|groups| {
        groups.lines().find_map(|line| {
            let mut fields = line.split(':');
            (fields.next()? == "input")
                .then(|| fields.nth(1)?.parse::<u32>().ok())
                .flatten()
        })
    });
    let Some(group_id) = group_id else {
        return false;
    };

    fs::read_to_string("/proc/self/status")
        .ok()
        .is_some_and(|status| {
            status.lines().any(|line| {
                let Some((name, values)) = line.split_once(':') else {
                    return false;
                };
                matches!(name, "Gid" | "Groups")
                    && values
                        .split_whitespace()
                        .filter_map(|value| value.parse::<u32>().ok())
                        .any(|value| value == group_id)
            })
        })
}

fn has_input_device_access() -> bool {
    let input_dir = Path::new("/dev/input");
    fs::read_dir(input_dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_name().to_string_lossy().starts_with("event")
                && OpenOptions::new().read(true).open(entry.path()).is_ok()
        })
    })
}

fn has_padbridge_udev_rule() -> bool {
    [
        "/etc/udev/rules.d",
        "/run/udev/rules.d",
        "/usr/lib/udev/rules.d",
        "/usr/local/lib/udev/rules.d",
        "/lib/udev/rules.d",
    ]
    .iter()
    .filter_map(|directory| fs::read_dir(directory).ok())
    .flat_map(fs::ReadDir::flatten)
    .any(|entry| {
        let path = entry.path();
        path.extension()
            .is_some_and(|extension| extension == "rules")
            && (path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().to_lowercase().contains("padbridge"))
                || fs::read_to_string(path)
                    .is_ok_and(|contents| contents.to_lowercase().contains("padbridge")))
    })
}

fn detect_init_system() -> &'static str {
    let process_name = fs::read_to_string("/proc/1/comm").unwrap_or_default();
    if Path::new("/run/systemd/system").exists() || process_name.trim() == "systemd" {
        "systemd"
    } else if Path::new("/run/openrc").exists() || process_name.contains("openrc") {
        "OpenRC"
    } else if Path::new("/etc/runit").exists()
        || Path::new("/run/runit").exists()
        || process_name.trim() == "runit"
    {
        "runit"
    } else {
        "other"
    }
}

fn report_check(name: &str, passed: bool, detail: &str, fix: &str) -> bool {
    println!("{} {name}: {detail}", if passed { "PASS" } else { "FAIL" });
    if !passed {
        println!("  Fix: {fix}");
    }
    passed
}
