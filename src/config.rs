use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Status {
    pub name: String,
    pub key: char,
}


#[derive(Deserialize)]
pub struct Field {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub status: Vec<Status>,
    pub field: Vec<Field>
}

