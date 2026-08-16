#![feature(specialization)]
pub mod ui;
pub mod review;
pub mod config;

use std::collections::HashMap;
use std::fs;
use std::process::exit;
use clap::Parser;
use crossterm::event::{read, Event};
use ratatui::prelude::Stylize;
use ratatui::widgets::Block;
use ratatui_textarea::TextArea;
use crate::config::Config;
use crate::review::{Review, Reviews};
use crate::ui::{AreaDescription, UITreeBuilder};


#[derive(Parser)]
struct ProgramArgs {
    #[arg(long="config", short='c')]
    config_dir: Option<String>,
}

#[cfg(target_os = "linux")]
const CONFIG_DIR: &'static str = "/home/{USER}/.config/clankrs/config.toml";

#[cfg(target_os = "linux")]
const LOG_DIR: &'static str = "/home/{USER}/.local/state/clankrs";

#[cfg(target_os = "windows")]
const CONFIG_DIR: &'static str = r"C:\Users\{USER}\AppData\Local\clankrs\config.toml";

#[cfg(target_os = "windows")]
const LOG_DIR: &'static str = r"C:\Users\{USER}\AppData\Local\clanrks\logs";

pub fn main() -> anyhow::Result<()> {
    ratatui::run(|terminal| -> anyhow::Result<()> {

        let args = ProgramArgs::parse();
        let config_path = if let Some(cfg) = args.config_dir {
            cfg
        } else {
            CONFIG_DIR.to_string().replace("{USER}", &whoami::username().unwrap())
        };

        let config: Config = toml::from_str(fs::read_to_string(&config_path)
            .map_err(|_| anyhow::Error::msg(format!("Failed to find config in path {}", config_path)))?
            .as_str())?;

        if config.field.len() > 8 {
            panic!("Cannot have more than 8 fields in config! If this is vital, make a issue on https://github.com/Austin-Bennett/rs-breviewer/issues");
        }

        let mut keybinds = HashMap::new();
        for k in &config.status {
            keybinds.insert(k.key, k.name.clone());
        }

        let mut fields = Vec::new();

        for field in &config.field {
            fields.push({
                let mut area = TextArea::default();
                area.set_block(Block::bordered().dark_gray().title(field.name.clone()));
                area
            })
        }

        let reviews = Reviews::new(vec![], keybinds, Block::bordered().dark_gray().title("Reviews"));

        // no manual Rc<RefCell<..>> wrapping or linking needed: each widget's
        // up/right/down/left neighbors are worked out from these areas.
        let mut tree = UITreeBuilder::new()
            .add(reviews, AreaDescription::new(0, 0, 100, 80));

        for (i, field) in fields.into_iter().enumerate() {
            let x = i as u8 % 4;
            let y = i as u8 / 4;

            tree = tree
                .add(field, AreaDescription::new(x * 25, 80 + 10 * y, 25, 10));
        }

        let mut tree = tree.build();

        loop {
            terminal.draw(|frame| tree.draw(frame))?;

            if let Event::Key(key) = read()? {
                if tree.handle_input(key) {
                    break;
                }
            }
        }

        //todo: build review, save to clipboard, write to log, exit

        Ok(())
    })
}
