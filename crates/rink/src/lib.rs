use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use rink_core::{ast, date, gnu_units, CURRENCY_FILE};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
}

impl Config {
    fn prefix(&self) -> &str {
        &self.prefix.as_deref().unwrap_or("#")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some(String::from("#")),
        }
    }
}

#[init]
fn init(config_dir: RString) -> (rink_core::Context, Config) {
    let config = match fs::read_to_string(format!("{config_dir}/rink.ron")) {
        Ok(v) => ron::from_str(&v)
            .map_err(|e| {
                format!(
                    "(Rink) Failed while parsing config file. Falling back to default...\n  {e}"
                )
            })
            .unwrap_or_default(),
        Err(e) => {
            eprintln!("(Rink) Failed while reading config file. Falling back to default...\n  {e}");
            Config::default()
        }
    };

    let mut ctx = rink_core::Context::new();

    let units = gnu_units::parse_str(rink_core::DEFAULT_FILE.unwrap());
    let dates = date::parse_datefile(rink_core::DATES_FILE);

    let mut currency_defs = Vec::new();

    match reqwest::blocking::get("https://rinkcalc.app/data/currency.json") {
        Ok(response) => match response.json::<ast::Defs>() {
            Ok(mut live_defs) => {
                currency_defs.append(&mut live_defs.defs);
            }
            Err(why) => println!("Error parsing currency json: {}", why),
        },
        Err(why) => println!("Error fetching up-to-date currency conversions: {}", why),
    }

    currency_defs.append(&mut gnu_units::parse_str(CURRENCY_FILE).defs);

    ctx.load(units);
    ctx.load(ast::Defs {
        defs: currency_defs,
    });
    ctx.load_dates(dates);

    return (ctx, config);
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Rink".into(),
        icon: "accessories-calculator".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, data: &mut (rink_core::Context, Config)) -> RVec<Match> {
    // VALIDATING PLUGIN
    // Early return when the prefix doesn't match:
    let config = &data.1;
    if config
        .prefix
        .as_deref()
        .is_some_and(|v| !input.starts_with(&v))
    {
        return RVec::new();
    }

    // MAIN
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    let ctx = &mut data.0;
    match rink_core::one_line(ctx, stripped_input) {
        Ok(result) => {
            let (title, desc) = parse_result(result);
            RVec::from(vec![Match {
                title: title.into(),
                description: desc.map(RString::from).into(),
                use_pango: false,
                icon: ROption::RSome("accessories-calculator".into()),
                id: ROption::RNone,
            }])
        }
        Err(_) => RVec::new(),
    }
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    HandleResult::Copy(selection.title.into_bytes())
}

/// Extracts the title and description from `rink` result.
/// The description is anything inside brackets from `rink`, if present.
fn parse_result(result: String) -> (String, Option<String>) {
    result
        .split_once(" (")
        .map(|(title, desc)| {
            (
                title.to_string(),
                Some(desc.trim_end_matches(')').to_string()),
            )
        })
        .unwrap_or((result, None))
}
