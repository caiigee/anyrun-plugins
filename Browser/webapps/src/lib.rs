use abi_stable::
    std_types::{
        ROption::{RNone, RSome},
        RString, RVec,
    }
;
use anyrun_plugin::*;
use br_common::{Bookmark, Browser};
use common::Bib;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::Deserialize;
use std::{fs, process};

#[derive(Debug, Deserialize)]
struct Config {
    prefix: Option<String>,
    // It has to be usize because the .take() method takes usize...
    max_entries: Option<usize>,
    profile_name: Option<String>,
    bib: Option<Bib>,
}

// QoL methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("")
    }
    fn max_entries(&self) -> usize {
        self.max_entries.unwrap_or(7)
    }
    fn profile_name(&self) -> &str {
        self.profile_name.as_deref().unwrap_or("default")
    }
    fn bib(&self) -> &Bib {
        self.bib.as_ref().unwrap_or(&Bib::None)
    }
}
// Defaults
impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some("".to_string()),
            max_entries: Some(5),
            profile_name: Some("default".to_string()),
            bib: Some(Bib::None),
        }
    }
}

// This exists so I don't have to call br_common::get_default_browser() in get_matches() AND in handle():
struct InitData {
    config: Config,
    default_browser: Box<dyn Browser>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = match fs::read_to_string(format!("{config_dir}/webapps.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "Failed while parsing webapps config file: {e}. Falling back to default..."
            );
            Config::default()
        }),
        Err(e) => {
            eprintln!(
                "Failed while reading webapps config file: {e}. Falling back to default..."
            );
            Config::default()
        }
    };

    let default_browser = br_common::get_default_browser().unwrap_or_else(|e| {
        eprintln!("Failed while getting default browser in init: {e}");
        process::exit(1);
    });

    InitData {
        config,
        default_browser,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Webapps".into(),
        icon: "face-monkey".into(), // Icon from the icon theme
    }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData {
        config,
        default_browser,
    } = data;

    // VALIDATING PLUGIN
    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    };

    // MAIN
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();
    let webapps: Vec<Bookmark> = match default_browser.bookmarks(config.profile_name()) {
        Ok(v) => v
            .into_iter()
            .filter(|bookmark| bookmark.title().ends_with("- Web"))
            .collect(),
        Err(e) => {
            eprintln!("Failed while getting bookmarks in webapps plugin: {e:?}");
            return RVec::new();
        }
    };

    // Handling blank input:
    if stripped_input.is_empty() {
        match config.bib() {
            Bib::All => return RVec::from_iter(webapps.into_iter().take(config.max_entries()).map(
                |bookmark| Match {
                    title: RString::from(bookmark.title()),
                    description: RSome(RString::from(bookmark.url)),
                    use_pango: false,
                    icon: RSome(RString::from(default_browser.icon())),
                    id: RNone,
                },
            )),
            Bib::None => return RVec::new(),
            Bib::Currated(v) => return RVec::from_iter(
                webapps
                    .into_iter()
                    .filter(|bookmark| v.contains(&bookmark.title().to_string()))
                    .take(config.max_entries())
                    .map(|bookmark| Match {
                        title: RString::from(bookmark.title()),
                        description: RSome(RString::from(bookmark.url)),
                        use_pango: false,
                        icon: RSome(RString::from(default_browser.icon())),
                        id: RNone,
                    }),
            ),
        }
    }

    let matcher = SkimMatcherV2::default();
    // Shadowing "webapps"; performing fuzzy matching:
    let mut webapps: Vec<(i64, Bookmark)> = webapps
        .into_iter()
        .filter_map(|bookmark| {
            let score = matcher.fuzzy_match(bookmark.title(), &stripped_input)?;
            Some((score, bookmark))
        })
        .collect();
    // Sorting webapps by score in descending order.
    webapps.sort_by(|a, b| b.0.cmp(&a.0));

    // SUCCESS
    RVec::from_iter(
        webapps
            .into_iter()
            .take(config.max_entries())
            .map(|(_, bookmark)| Match {
                title: RString::from(bookmark.title()),
                description: RSome(RString::from(bookmark.url)),
                use_pango: false,
                icon: RSome(RString::from(default_browser.icon())),
                id: RNone,
            }),
    )
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData {
        config: _,
        default_browser,
    } = data;

    default_browser
        // Description MUST be Some, this is just how I have implemented it, that's why it is safe to .unwrap() here.
        .open(&selection.description.unwrap())
        .unwrap_or_else(|e| eprintln!("Failed while opening URL in browser: {e}"));

    HandleResult::Close
}
