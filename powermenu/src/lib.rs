use abi_stable::std_types::{RNone, RSome, RString, RVec};
use anyrun_plugin::*;
use common::Bib;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
    bib: Option<Bib>,
}

// QoL methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("!")
    }
    fn bib(&self) -> &Bib {
        self.bib.as_ref().unwrap_or(&Bib::All)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some("!".to_string()),
            bib: Some(Bib::All),
        }
    }
}

struct MenuOption<'a> {
    title: &'a str,
    icon: &'a str,
    command: &'a str,
    arg: &'a str,
}

const MENU_OPTIONS: &[MenuOption] = &[
    MenuOption {
        title: "Power off",
        icon: "system-shutdown",
        command: "systemctl",
        arg: "poweroff",
    },
    MenuOption {
        title: "Reboot",
        icon: "system-reboot",
        command: "systemctl",
        arg: "reboot",
    },
    MenuOption {
        title: "Suspend",
        icon: "face-tired",
        command: "systemctl",
        arg: "suspend",
    },
    MenuOption {
        title: "Lock",
        icon: "system-lock-screen",
        command: "loginctl",
        arg: "lock-session",
    }
];

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{config_dir}/powermenu.ron")) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|e| {
            eprintln!("Failed while parsing powermenu config: {e:?}. Falling back to default...");
            Config::default()
        }),
        Err(e) => {
            eprintln!("Failed while reading powermenu config: {e:?}. Falling back to default.");
            Config::default()
        }
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Power Options".into(),
        icon: "system-shutdown".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    // PLUGIN VALIDATION
    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    // MAIN
    // It is safe to unwrap here because of the early return above.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap();

    if stripped_input.is_empty() {
        match config.bib() {
            Bib::All => {
                return RVec::from_iter(MENU_OPTIONS.into_iter().map(|menu_option| Match {
                    title: RString::from(menu_option.title),
                    description: RNone,
                    use_pango: false,
                    icon: RSome(RString::from(menu_option.icon)),
                    id: RNone,
                }))
            }
            Bib::None => return RVec::new(),
            Bib::Currated(v) => {
                return RVec::from_iter(
                    MENU_OPTIONS
                        .into_iter()
                        .filter(|menu_option| v.contains(&menu_option.title.to_string()))
                        .map(|menu_option| Match {
                            title: RString::from(menu_option.title),
                            description: RNone,
                            use_pango: false,
                            icon: RSome(RString::from(menu_option.icon)),
                            id: RNone,
                        }),
                )
            }
        }
    }

    let matcher = SkimMatcherV2::default();
    // Performing fuzzy matching
    let mut options: Vec<(i64, &MenuOption)> = MENU_OPTIONS
        .into_iter()
        .filter_map(|menu_option| {
            let score = matcher.fuzzy_match(menu_option.title, stripped_input)?;
            Some((score, menu_option))
        })
        .collect();
    // Sorting options by score in descending order.
    options.sort_by(|a, b| b.0.cmp(&a.0));
    // We want to take only one option, the one with the highest score, because it makes no sense to display multiple powermenu options.
    let option = options[0].1;

    // SUCCESS
    return RVec::from(vec![Match {
        title: RString::from(option.title),
        description: RNone,
        use_pango: false,
        icon: RSome(RString::from(option.icon)),
        id: RNone,
    }]);
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    // It is safe to unwrap here because there is no way that the selection title doesn't match at least one title from MENU_OPTIONS.
    let selected_option = MENU_OPTIONS
        .into_iter()
        .find(|option| option.title == selection.title)
        .unwrap();

    if let Err(e) = Command::new(selected_option.command)
        .arg(selected_option.arg)
        .output()
    {
        eprintln!("Failed while executing powermenu command: {e:?}");
    }

    HandleResult::Close
}
