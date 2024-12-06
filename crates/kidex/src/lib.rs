use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::{anyrun_interface::HandleResult, *};
use fuzzy_matcher::FuzzyMatcher;
use kidex_common::IndexEntry;
use serde::Deserialize;
use std::{fs, os::unix::prelude::OsStrExt, process::Command};

mod util;

#[derive(Deserialize)]
struct Config {
    max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self { max_entries: 3 }
    }
}

#[handler]
pub fn handler(selection: Match, state: &mut InitData) -> HandleResult {
    match &state.selection {
        Some(index_entry) => match selection.id.unwrap().into() {
            util::IndexAction::Open => {
                if let Err(why) = Command::new("xdg-open").arg(&index_entry.path).spawn() {
                    println!("Error running xdg-open: {}", why);
                }
                HandleResult::Close
            }
            util::IndexAction::CopyPath => {
                HandleResult::Copy(index_entry.path.clone().into_os_string().as_bytes().into())
            }
            util::IndexAction::Back => {
                state.selection = None;
                HandleResult::Refresh(false)
            }
        },
        None => {
            let (_, index_entry) = state
                .index
                .iter()
                .find(|(id, _)| selection.id == ROption::RSome(*id as u64))
                .unwrap();

            state.selection = Some(index_entry.clone());
            HandleResult::Refresh(true)
        }
    }
}

struct InitData {
    config: Config,
    index: Vec<(usize, IndexEntry)>,
    selection: Option<IndexEntry>,
}

#[init]
pub fn init(config_dir: RString) -> InitData {
    let config = match fs::read_to_string(format!("{}/kidex.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    };
    let index = match kidex_common::util::get_index(None) {
        Ok(index) => index.into_iter().enumerate().collect(),
        Err(why) => {
            println!("Failed to get kidex index: {}", why);
            Vec::new()
        }
    };
    InitData {
        config,
        index,
        selection: None,
    }
}

#[get_matches]
pub fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    match &data.selection {
        Some(index_entry) => {
            let path = index_entry.path.to_string_lossy();
            vec![
                Match {
                    title: "Open File".into(),
                    description: ROption::RSome(path.clone().into()),
                    use_pango: false,
                    id: ROption::RSome(util::IndexAction::Open as u64),
                    icon: ROption::RSome("document-open".into()),
                },
                Match {
                    title: "Copy Path".into(),
                    description: ROption::RSome(path.into()),
                    use_pango: false,
                    id: ROption::RSome(util::IndexAction::CopyPath as u64),
                    icon: ROption::RSome("edit-copy".into()),
                },
                Match {
                    title: "Back".into(),
                    description: ROption::RNone,
                    use_pango: false,
                    id: ROption::RSome(util::IndexAction::Back as u64),
                    icon: ROption::RSome("edit-undo".into()),
                },
            ]
            .into()
        }
        None => {
            let matcher = fuzzy_matcher::skim::SkimMatcherV2::default().smart_case();
            let mut index = data
                .index
                .clone()
                .into_iter()
                .filter_map(|(id, index_entry)| {
                    matcher
                        .fuzzy_match(&index_entry.path.as_os_str().to_string_lossy(), &input)
                        .map(|val| (index_entry, id, val))
                })
                .collect::<Vec<_>>();

            index.sort_by(|a, b| b.2.cmp(&a.2));

            index.truncate(data.config.max_entries);
            index
                .into_iter()
                .map(|(entry_index, id, _)| Match {
                    title: entry_index
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy().into())
                        .unwrap_or("N/A".into()),
                    description: entry_index
                        .path
                        .parent()
                        .map(|path| path.display().to_string().into())
                        .into(),
                    use_pango: false,
                    icon: ROption::RSome(if entry_index.directory {
                        "folder".into()
                    } else {
                        "text-x-generic".into()
                    }),
                    id: ROption::RSome(id as u64),
                })
                .collect()
        }
    }
}

#[info]
pub fn info() -> PluginInfo {
    PluginInfo {
        name: "Kidex".into(),
        icon: "folder".into(),
    }
}