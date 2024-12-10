use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use freedesktop_desktop_entry::DesktopEntry;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::Deserialize;
use std::
    process::{self, Command}
;

mod util;

#[derive(Deserialize, Debug)]
struct Config {
    prefix: Option<String>,
    // TODO 5.
    // desktop_actions: Option<bool>,
    max_entries: Option<usize>,
    // TODO 4.
    // terminal: Option<String>,
    // shell: Option<String>,
    bib: Option<common::Bib>,
}

// QoL methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("")
    }
    fn bib(&self) -> &common::Bib {
        self.bib.as_ref().unwrap_or(&common::Bib::None)
    }
    fn max_entries(&self) -> usize {
        self.max_entries.unwrap_or(5)
    }
    // fn terminal(&self) -> &str {
    //     self.terminal.as_deref().unwrap_or("kitty")
    // }
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
            bib: Some(common::Bib::None),
        }
    }
}

pub struct InitData<'a> {
    config: Config,
    common_config: common::CommonConfig,
    // I am not sure if this is supposed to be like this, but the idea of mapping
    // paths to DesktopEntry types in the get_matches() function sounds absurd.
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
    let config = common::config(&config_dir, "Applications");
    let common_config = common::common_config(&config_dir, "Applications");
    let entries = util::scrape_desktop_entries().unwrap_or_else(|e| {
        eprintln!("(Applications) Failed to load desktop entries. Closing...:\n  {e}");
        process::exit(1)
    });

    InitData {
        config,
        entries,
        common_config,
    }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData {
        config,
        entries,
        common_config: _,
    } = data;

    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    // It is safe to unwrap here because of the early return above.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap();

    // Handling blank input behaviour:
    if stripped_input.is_empty() {
        match config.bib() {
            common::Bib::All => {
                return RVec::from_iter(entries.iter().take(config.max_entries()).map(|de| Match {
                    title: RString::from(de.name::<&str>(&[]).unwrap_or("Desktop Entry".into())),
                    description: RSome(RString::from(de.comment::<&str>(&[]).unwrap_or_default())),
                    use_pango: false,
                    icon: RSome(RString::from(
                        de.icon().unwrap_or("application-x-executable"),
                    )),
                    id: RNone,
                }))
            }
            common::Bib::None => return RVec::new(),
            common::Bib::Currated(v) => {
                return RVec::from_iter(
                    entries
                        .iter()
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
        .iter()
        .filter_map(|de| {
            let score =
                matcher.fuzzy_match(de.name::<&str>(&[])?.into_owned().as_str(), stripped_input)?;
            Some((score, de))
        })
        .collect();
    // Sorting entries by score in descending order.
    entries.sort_by(|a, b| b.0.cmp(&a.0));

    RVec::from_iter(
        entries
            .iter()
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
    let InitData {
        config: _,
        entries,
        common_config,
    } = data;
    let selected_de = entries
        .iter()
        .find(|de| de.name::<&str>(&[]).unwrap() == selection.title)
        // It is safe to unwrap the ".find()" method because there is no
        // way selection is not in the "entries" vector.
        .unwrap();

    let exec = match selected_de.parse_exec() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("(Applications) Failed while parsing exec from selected Desktop Entry. Closing...\n  {e}.");
            return HandleResult::Close;
        }
    };

    if common_config.prefix_args().is_empty() {
        match Command::new(&exec[0]).args(&exec[1..]).spawn() {
            Ok(_) => (),
            Err(e) => eprintln!("(Applications) Failed while executing command:\n  {e}"),
        }
    } else {
        match Command::new(&common_config.prefix_args()[0])
            .args(common_config.prefix_args()[1..].iter())
            .args(exec.iter())
            .spawn()
        {
            Ok(_) => (),
            Err(e) => eprintln!("(Applications) Failed while executing command:\n  {e}"),
        }
    }

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
