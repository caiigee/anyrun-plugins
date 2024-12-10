use common::Firefox;
use std::{error::Error, fs::File, io::Read};

use crate::{Engine, SearchEngines};

// #[derive(Debug, Deserialize, Clone)]
// pub struct ParameterData {
//     name: String,
//     value: String,
// }

// impl ParameterData {
//     pub fn name(&self) -> &str {
//         &self.name
//     }
//     pub fn value(&self) -> &str {
//         &self.value
//     }
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct UrlData {
//     params: Vec<ParameterData>,
//     template: String,
// }

// impl UrlData {
//     pub fn params(&self) -> &Vec<ParameterData> {
//         &self.params
//     }
//     pub fn template(&self) -> &str {
//         &self.template
//     }
// }

// #[derive(Debug, Deserialize, Clone)]
// pub struct FirefoxEngine {
//     #[serde(rename = "_definedAliases")]
//     defined_aliases: Vec<String>,

//     #[serde(rename = "_iconURL")]
//     icon_url: String,

//     #[serde(rename = "_isAppProvided")]
//     is_app_provided: bool,

//     _name: String,
//     _urls: Vec<UrlData>,
// }

// #[derive(Debug, Deserialize)]
// pub struct SearchData {
//     engines: Vec<FirefoxEngine>,
// }

// impl SearchData {
//     pub fn engines(&self) -> &Vec<FirefoxEngine> {
//         &self.engines
//     }
// }

impl SearchEngines for Firefox {
    fn search_engines(&self, profile_name: &str) -> Result<Vec<Engine>, Box<dyn Error>> {
        let profile_dir = Firefox::profile_dir(profile_name)
            .map_err(|e| format!("Failed while getting the profile directory:\n    {e}"))?;

        // PROCESSING THE mozlz4 FILE
        let mut file = File::open(format!("{profile_dir}/search.json.mozlz4"))
            .map_err(|e| format!("Failed while opening mozlz4 file:\n    {e}"))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed while reading mozlz4 file:\n    {e}"))?;
        let decompressed = mozlz4::decompress(buffer)
            .map_err(|e| format!("Failed while decompressing the mozlz4 file:\n    {e}"))?;

        let data: serde_json::Value = serde_json::from_slice(&decompressed)
            .map_err(|e| format!("Failed while parsing JSON:\n    {e}"))?;

        // MAPPING TO THE Engine STRUCT
        // The below code sucks ass:
        let engines = data["engines"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|engine_data| {
                if engine_data["_isAppProvided"].as_bool() == Some(true) {
                    return None;
                }

                let name = engine_data["_name"].as_str()?;

                let alias = engine_data["_definedAliases"]
                    .as_array()?
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let icon = engine_data["_iconURL"].as_str()?;
                // Removing the scheme.
                let icon = &icon[icon.find("://").unwrap() + 3..];

                let url_data = &engine_data["_urls"].as_array()?[0];
                let params = url_data["params"]
                    .as_array()?
                    .iter()
                    .map(|param_data| {
                        let name = param_data["name"].as_str().unwrap();
                        let value = param_data["value"].as_str().unwrap();
                        format!("{name}={value}")
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                let template = url_data["template"].as_str()?;

                let url = format!("{template}?{params}");

                Some(Engine::new(name, &url, alias, icon))
            })
            .collect();

        Ok(engines)

        // let data: SearchData = serde_json::from_slice(&decompressed)
        //     .map_err(|e| format!("Failed while parsing JSON:\n    {e}"))?;

        // let engines: Vec<Engine> = data
        //     .engines()
        //     .iter()
        //     .filter_map(|engine| {
        //         if engine.is_app_provided {
        //             return None;
        //         };

        // let params = engine._urls[0]
        //     .params
        //     .iter()
        //     .map(|param| format!("{}={}", param.name, param.value))
        //     .collect::<Vec<String>>()
        //     .join("&");
        // let url = format!("{}?{}", engine._urls[0].template, params);

        //         Some(Engine::new(
        //             &engine._name,
        //             &url,
        //             &engine.defined_aliases[0],
        //             &engine.icon_url,
        //         ))
        //     })
        //     .collect();

        // let default_engine_index = search_data
        //     .engines()
        //     .iter()
        //     .position(|engine| engine._name() == search_data.default_engine())
        //     .ok_or_else(|| "Could not find the default engine index from browser's search data!")?;

        // // Cloning the Vec<Engine> and placing the default_engine at the start:
        // let mut search_engines = search_data.engines().to_vec();

        // search_engines.swap(0, default_engine_index);
    }
}
