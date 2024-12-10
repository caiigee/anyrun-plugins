use serde::{de::DeserializeOwned, Deserialize};
use std::{
    env,
    error::Error,
    fmt::Debug,
    fs::{self},
    process::Command,
};

#[derive(Debug, Deserialize)]
pub enum Bib {
    All,
    None,
    Currated(Vec<String>),
    // TODO 3.
}

#[derive(Debug, Deserialize)]
pub struct CommonConfig {
    browser_profile_name: Option<String>,
    prefix_args: Option<Vec<String>>,
}

impl CommonConfig {
    pub fn browser_profile_name(&self) -> &str {
        self.browser_profile_name.as_deref().unwrap_or("default")
    }
    pub fn prefix_args(&self) -> &[String] {
        self.prefix_args.as_deref().unwrap_or_default()
    }
}

impl Default for CommonConfig {
    fn default() -> Self {
        CommonConfig {
            browser_profile_name: Some("default".to_string()),
            prefix_args: Some(Vec::default()),
        }
    }
}

pub fn common_config(config_dir: &str, plugin: &str) -> CommonConfig {
    match fs::read_to_string(format!("{config_dir}/Common.ron")) {
        Ok(s) => ron::from_str(&s)
            .map_err(|e| {
                format!(
                    "({plugin}) Failed while parsing common config file. \
                Falling back to default...\n  {e}"
                )
            })
            .unwrap_or_default(),
        Err(e) => {
            eprintln!(
                "({plugin}) Failed while reading common config file. Falling back to default...\n  {e}"
            );
            CommonConfig::default()
        }
    }
}

pub fn config<T>(config_dir: &str, plugin: &str) -> T
where
    T: DeserializeOwned + Default + Debug,
{
    match fs::read_to_string(format!("{config_dir}/{plugin}.ron")) {
        Ok(s) => ron::from_str(&s)
            .map_err(|e| {
                format!(
                "({plugin}) Failed while parsing config file. Falling back to default...:\n  {e}"
            )
            })
            .unwrap_or_default(),
        Err(e) => {
            eprintln!(
                "({plugin}) Failed while reading config file. Falling back to default...\n  {e}"
            );
            T::default()
        }
    }
}

// The Send + Sync are necessary for creating structs:
pub trait Browser: Send + Sync + Debug {
    fn new_window(&self, url: &str, prefix: &[String]) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug)]
pub struct Firefox {
    profile_name: String,
}

impl Firefox {
    pub fn new(profile_name: &str) -> Self {
        Firefox {
            profile_name: profile_name.to_string(),
        }
    }
    pub fn profile_dir(profile_name: &str) -> Result<String, Box<dyn Error>> {
        let home_dir = env::var("HOME").map_err(|e| {
            format!("HOME env variable not set! Bookmarks cannot be obtained:\n    {e}")
        })?;

        let firefox_path = format!("{home_dir}/.mozilla/firefox");

        // The profile directory may have a name like "<random_characters>.<name>"
        // or just "<name>". Because it cannot be inferred from the profile name alone,
        // a .find() method is required to find the target dir:
        let profile_dirname = fs::read_dir(&firefox_path)
            .map_err(|e| format!("Failed while reading firefox directory:\n    {e}"))?
            // At this point we have an Iterator over Result<DirEntry, Error> and
            // we want the DirEntry which corresponds to the profile directory:
            .find_map(|entry| match entry {
                Ok(dir_entry) => {
                    if dir_entry
                        .file_name()
                        .to_string_lossy()
                        .contains(profile_name)
                    {
                        Some(dir_entry)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    eprintln!("Failed while unwrapping entry of {firefox_path}:\n    {e}");
                    None
                }
            })
            // At this point we either have the DirEntry we need or we don't. We have to
            // return from the function with an Err if we didn't find the directory:
            .ok_or_else(|| {
                format!(
                    "Cannot find the profile directory, please make sure that \
                    the profile with name {profile_name} exists!"
                )
            })?
            // In this part of the code we are converting DirEntry to String:
            .file_name()
            .to_string_lossy()
            .into_owned();

        Ok(format!("{firefox_path}/{profile_dirname}"))
    }

    pub fn profile_dir2(&self) -> Result<String, Box<dyn Error>> {
        let home_dir = env::var("HOME").map_err(|e| {
            format!("HOME env variable not set! Bookmarks cannot be obtained:\n    {e}")
        })?;

        let firefox_path = format!("{home_dir}/.mozilla/firefox");

        // The profile directory may have a name like "<random_characters>.<name>"
        // or just "<name>". Because it cannot be inferred from the profile name alone,
        // a .find() method is required to find the target dir:
        let profile_dirname = fs::read_dir(&firefox_path)
            .map_err(|e| format!("Failed while reading firefox directory:\n    {e}"))?
            // At this point we have an Iterator over Result<DirEntry, Error> and
            // we want the DirEntry which corresponds to the profile directory:
            .find_map(|entry| match entry {
                Ok(dir_entry) => {
                    if dir_entry
                        .file_name()
                        .to_string_lossy()
                        .contains(&self.profile_name)
                    {
                        Some(dir_entry)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    eprintln!("Failed while unwrapping entry of {firefox_path}:\n    {e}");
                    None
                }
            })
            // At this point we either have the DirEntry we need or we don't. We have to
            // return from the function with an Err if we didn't find the directory:
            .ok_or_else(|| {
                format!(
                    "Cannot find the profile directory, please make sure that \
                    the profile with name {} exists!",
                    self.profile_name
                )
            })?
            // In this part of the code we are converting DirEntry to String:
            .file_name()
            .to_string_lossy()
            .into_owned();

        Ok(format!("{firefox_path}/{profile_dirname}"))
    }
}

impl Browser for Firefox {
    fn new_window(&self, url: &str, prefix_args: &[String]) -> Result<(), Box<dyn Error>> {
        if prefix_args.is_empty() {
            Command::new("firefox")
                .args(["--new-window", url])
                .spawn()?;
        } else {
            Command::new(&prefix_args[0])
                .args(prefix_args[1..].iter())
                .args(["firefox", "--new-window", url])
                .spawn()?;
        }
        Ok(())
    }
}

pub fn default_browser_id() -> Result<String, Box<dyn Error>> {
    let output = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .map_err(|e| format!("Failed while executing \"xdg-settings\":\n   {e}"))?;
    let appid = String::from_utf8(output.stdout)
        .map_err(|e| {
            format!("Failed while converting \"xdg-settings\" stdout to a String:\n   {e}")
        })?
        .trim()
        .replace(".desktop", "");

    // match appid.as_str() {
    //     "firefox" => Ok(Box::new(Firefox::new())),
    //     _ => Err("Unsupported default browser!".into()),
    // }
    Ok(appid)
}

// Utility function for extracting the domain from a URL:
// fn extract_domain(url: &str) -> Result<String, Box<dyn Error>> {
//     let domain_re = Regex::new(r"^https?://([^/]+)")
//         .map_err(|e| format!("Failed while creating domain regex:\n    {e}"))?;
//     // Getting the captures:
//     let captures = domain_re
//         .captures(&url)
//         .ok_or_else(|| "Failed while getting domain regex captures!")?;
//     // Getting the capture group 1:
//     let domain = captures
//         .get(1)
//         .ok_or_else(|| "Failed while getting domain regex first capture group!")?;

//     // SUCCESS
//     Ok(domain.as_str().to_string())
// }
