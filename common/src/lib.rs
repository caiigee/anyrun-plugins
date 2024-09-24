use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum Bib {
    All,
    None,
    Currated(Vec<String>),
    // TODO 
    // Frequent,
}
