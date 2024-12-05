use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub enum Bib {
    All,
    None,
    Currated(Vec<String>),
    // TODO 3.
}

#[derive(Debug, Deserialize)]
pub struct BrowserConfig {
    profile_name: Option<String>,
    command_prefix: Option<String>,
}

impl BrowserConfig {
    pub fn profile_name(&self) -> &str {
        &self.profile_name.as_deref().unwrap_or("default")
    }
    pub fn command_prefix(&self) -> &str {
        &self.command_prefix.as_deref().unwrap_or_default()
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        BrowserConfig {
            profile_name: Some("default".to_string()),
            command_prefix: Some(String::default()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    title: Option<String>,
    url: String,
    keyword: Option<String>,
}

impl Bookmark {
    pub fn title(&self) -> &str {
        // let domain = extract_domain(&self.url).unwrap_or_else(|e| {
        //     eprintln!(
        //         "Failed while extracting domain. Using \"shrug\" as the bookmark title:\n  {e}"
        //     );
        //     r"¯\_(ツ)_/¯".to_string()
        // });

        self.title.as_deref().unwrap_or(&self.url)
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn keyword(&self) -> &Option<String> {
        &self.keyword
    }
    pub fn new(title: Option<String>, url: String, keyword: Option<String>) -> Self {
        Bookmark {
            title,
            url,
            keyword,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ParameterData {
    name: String,
    value: String,
}

impl ParameterData {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct UrlData {
    params: Vec<ParameterData>,
    template: String,
}

impl UrlData {
    pub fn params(&self) -> &Vec<ParameterData> {
        &self.params
    }
    pub fn template(&self) -> &str {
        &self.template
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Engine {
    #[serde(rename = "_definedAliases")]
    defined_aliases: Vec<String>,

    #[serde(rename = "_iconURL")]
    icon_url: String,

    _name: String,
    _urls: Vec<UrlData>,
}

impl Engine {
    pub fn defined_aliases(&self) -> &Vec<String> {
        &self.defined_aliases
    }
    pub fn icon_url(&self) -> &str {
        &self.icon_url
    }
    pub fn _name(&self) -> &str {
        &self._name
    }
    pub fn _urls(&self) -> &Vec<UrlData> {
        &self._urls
    }
}

#[derive(Debug, Deserialize)]
pub struct MetaData {
    current: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchData {
    engines: Vec<Engine>,

    #[serde(rename = "metaData")]
    meta_data: MetaData,
}

impl SearchData {
    pub fn engines(&self) -> &Vec<Engine> {
        &self.engines
    }
    pub fn default_engine(&self) -> &str {
        &self.meta_data.current
    }
}
