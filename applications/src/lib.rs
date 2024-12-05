use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use common::types::Bib;
use freedesktop_desktop_entry::DesktopEntry;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::Deserialize;
use std::{
    fs,
    process::{self},
};

mod util;

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
    // TODO 5.
    // desktop_actions: Option<bool>,
    max_entries: Option<usize>,
    // TODO 4.
    // terminal: Option<String>,
    // shell: Option<String>,
    command_prefix: Option<String>,
    bib: Option<Bib>,
}

// QoL methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("")
    }
    // fn terminal(&self) -> &str {
    //     self.terminal.as_deref().unwrap_or("kitty")
    // }
    fn bib(&self) -> &Bib {
        self.bib.as_ref().unwrap_or(&Bib::None)
    }
    fn max_entries(&self) -> usize {
        self.max_entries.unwrap_or(5)
    }
    fn command_prefix(&self) -> &str {
        &self.command_prefix.as_deref().unwrap_or_default()
    }
    // fn shell(&self) -> String {
    //     self.shell.clone().unwrap_or_else(|| {
    //         env::var("SHELL").unwrap_or_else(|e| {
    //             eprintln!("(Applications) Failed while finding the SHELL env variable: {e}");
    //             process::exit(1);
    //         })
    //     })
    // }
}

impl Default for Config {
    fn default() -> Self {
        // let shell = env::var("SHELL").unwrap_or_else(|e| {
        //     eprintln!(
        //         "(Applications) Failed while getting the SHELL env variable. Closing...:\n  {e}"
        //     );
        //     process::exit(1);
        // });
        Config {
            prefix: Some(String::default()),
            // desktop_actions: Some(false),
            max_entries: Some(5),
            // terminal: Some("kitty".to_string()),
            // shell: Some(shell),
            bib: Some(Bib::None),
            command_prefix: Some(String::default())
        }
    }
}

pub struct InitData<'a> {
    config: Config,
    entries: Vec<DesktopEntry<'a>>,
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Applications".into(),
        icon: "application-x-executable".into(),
    }
}

#[init]
pub fn init(config_dir: RString) -> InitData<'static> {
    let config = match fs::read_to_string(format!("{config_dir}/applications.ron")) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "(Applications) Failed while parsing applications config:\n  {e} Falling back to default..."
            );
            Config::default()
        }),
        Err(e) => {
            eprintln!("(Applications) Failed while reading applications config:\n  {e} Falling back to default.");
            Config::default()
        }
    };

    let entries = util::scrape_desktop_entries().unwrap_or_else(|e| {
        eprintln!("(Applications) Failed to load desktop entries:\n  {e} Crashing Anyrun...");
        process::exit(1)
    });

    InitData { config, entries }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData { config, entries } = data;

    // PLUGIN VALIDATION
    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    // MAIN
    // It is safe to unwrap here because of the early return above.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap();

    // Handling blank input behaviour:
    if stripped_input.is_empty() {
        match config.bib() {
            Bib::All => {
                return RVec::from_iter(entries.into_iter().take(config.max_entries()).map(|de| {
                    Match {
                        title: RString::from(
                            de.name::<&str>(&[]).unwrap_or("Desktop Entry".into()),
                        ),
                        description: RSome(RString::from(
                            de.comment::<&str>(&[]).unwrap_or_default(),
                        )),
                        use_pango: false,
                        icon: RSome(RString::from(
                            de.icon().unwrap_or("application-x-executable"),
                        )),
                        id: RNone,
                    }
                }))
            }
            Bib::None => return RVec::new(),
            Bib::Currated(v) => {
                return RVec::from_iter(
                    entries
                        .into_iter()
                        .filter(|de| v.contains(&de.appid.to_string()))
                        .take(config.max_entries())
                        .map(|de| Match {
                            title: RString::from(
                                de.name::<&str>(&[]).unwrap_or("Desktop Entry".into()),
                            ),
                            description: RSome(RString::from(
                                de.comment::<&str>(&[]).unwrap_or_default(),
                            )),
                            use_pango: false,
                            icon: RSome(RString::from(
                                de.icon().unwrap_or("application-x-executable"),
                            )),
                            id: RNone,
                        }),
                )
            }
        }
    }

    let matcher = SkimMatcherV2::default();
    // Shadowing "entries"; performing fuzzy matching:
    let mut entries: Vec<(i64, &DesktopEntry)> = entries
        .into_iter()
        .filter_map(|de| {
            let score =
                matcher.fuzzy_match(de.name::<&str>(&[])?.into_owned().as_str(), stripped_input)?;
            Some((score, de))
        })
        .collect();
    // Sorting entries by score in descending order.
    entries.sort_by(|a, b| b.0.cmp(&a.0));

    // SUCCESS
    RVec::from_iter(
        entries
            .into_iter()
            .take(config.max_entries())
            .map(|(_, de)| Match {
                title: RString::from(de.name::<&str>(&[]).unwrap_or("Desktop Entry".into())),
                description: RSome(RString::from(de.comment::<&str>(&[]).unwrap_or_default())),
                use_pango: false,
                icon: RSome(RString::from(de.icon().unwrap_or_default())),
                id: RNone,
            }),
    )
}

#[handler]
pub fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData { config: _, entries } = data;
    // It is safe to unwrap the ".find()" method because there is no way selection is not in the "entries" vector:
    let selected_de = entries
        .into_iter()
        .find(|de| de.name::<&str>(&[]).unwrap_or("Desktop Entry".into()) == selection.title)
        .unwrap();

    let exec = match selected_de.parse_exec() {
        Ok(v) => v.join(" "),
        Err(e) => {
            eprintln!("(Applications) Failed while parsing exec from selected Desktop Entry. Closing...\n  {e}.");
            return HandleResult::Close;
        }
    };
    
    println!("LOL THIS IS APPLICATIONS → {exec}");
    
    // if selected_de.terminal() {
    //     if let Err(e) = Command::new().args(["-e", &exec]).spawn() {
    //         eprintln!(
    //             "(Applications) Failed while executing Desktop Entry's exec using the terminal emulator:\n  {e}"
    //         )
    //     }
    // } else {
    //     if let Err(e) = Command::new(config.shell()).args(["-c", &exec]).spawn() {
    //         eprintln!("(Applications) Failed while executing Desktop Entry's exec using the shell:\n  {e}")
    //     }
    // }

    HandleResult::Close
}
