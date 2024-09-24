use std::process::Command;

use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;

#[init]
fn init(_config_dir: RString) {}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Power Options".into(),
        icon: "system-shutdown".into(),
    }
}

#[get_matches]
fn get_matches(input: RString) -> RVec<Match> {
    match input.to_lowercase().as_str() {
        "reboot" => RVec::from(vec![Match {
            title: "Reboot".into(),
            icon: ROption::RSome("system-reboot".into()),
            use_pango: false,
            description: ROption::RSome("Restart the system".into()),
            id: ROption::RNone,
        }]),
        "shutdown" => RVec::from(vec![Match {
            title: "Shutdown".into(),
            icon: ROption::RSome("system-shutdown".into()),
            use_pango: false,
            description: ROption::RNone,
            id: ROption::RNone,
        }]),
        "power off" => RVec::from(vec![Match {
            title: "Power off".into(),
            icon: ROption::RSome("system-shutdown".into()),
            use_pango: false,
            description: ROption::RNone,
            id: ROption::RNone,
        }]),
        _ => RVec::new(),
    }
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    match selection.title.as_str() {
        "Reboot" => {
            match Command::new("systemctl").arg("reboot").output() {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to execute reboot command: {e}"),
            };
        }
        "Shutdown" => {
            match Command::new("shutdown").arg("now").output() {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to execute shutdown command: {e}"),
            };
        }
        "Power off" => {
            match Command::new("systemctl").arg("poweroff").output() {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to execute shutdown command: {e}"),
            };
        }
        _ => (),
    }
    
    HandleResult::Close
}
