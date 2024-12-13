use common::Firefox;
use std::{error::Error, fs::File, io::Read};

use crate::{Engine, SearchEngines};

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
                
                // When the version is 10, the engines with no aliases have an empty list
                // for "_definedAliases", but when the version is 6 the "_definedAliases" doesn't
                // exist at all. I have no idea what determines the versions, but I hope it always just stays
                // version 6:
                // let alias = engine_data["_definedAliases"]
                //     .as_array()?
                //     .first()
                //     .and_then(|v| v.as_str())
                //     .unwrap_or_default();
                let alias = engine_data
                    .get("_definedAliases")
                    .and_then(|v| Some(v.as_array().unwrap()[0].as_str().unwrap()))
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
    }
}
