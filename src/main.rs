#![feature(specialization)]
use std::fmt::Write;
pub mod ui;
pub mod review;
pub mod config;

use std::collections::HashMap;
use std::fs;
use std::io::stdin;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use anyhow::{anyhow, bail};
use arboard::Clipboard;
use chrono::{Local, Utc};
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
const LOG_DIR: &'static str = r"C:\Users\{USER}\AppData\Local\clankrs\logs";

pub fn main() -> anyhow::Result<()> {
    let res = ratatui::run(|terminal| -> anyhow::Result<String> {

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
        // up/right/down/left neighbors are worked out from these areas. `add`
        // hands back a handle to the widget so it can still be read after the
        // tree (and the loop below) is done with it.
        let mut builder = UITreeBuilder::new();
        let reviews_handle = builder.add(reviews, AreaDescription::new(0, 0, 100, 80));

        let field_handles: Vec<_> = fields.into_iter().enumerate().map(|(i, field)| {
            let x = i as u8 % 4;
            let y = i as u8 / 4;

            builder.add(field, AreaDescription::new(x * 25, 80 + 10 * y, 25, 10))
        }).collect();

        let mut tree = builder.build();

        loop {
            terminal.draw(|frame| tree.draw(frame))?;

            if let Event::Key(key) = read()? {
                if tree.handle_input(key) {
                    break;
                }
            }
        }

        let reviews = reviews_handle.borrow();
        let fields: Vec<_> = field_handles.iter().map(|h| h.borrow()).collect();

        //todo: build review, save to clipboard, write to log, exit

        let mut res = String::new();

        let mut first = true;
        //Inefficient? maybe, but who cares since N < 50
        for status in config.status.iter() {
            if !first {
                res += " ";
            }
            first = false;
            let mut count = 0;
            for review in reviews.reviews.iter() {
                if review.status.eq(&status.name) {
                    count += 1;
                }
            }

            write!(res, "{} {}", count, status.name)?;
        }
        res.push('\n');

        for (i, review) in reviews.reviews.iter().enumerate() {
            writeln!(res, "VM{} {} {}", i + 1, review.status, review.message.lines().join("\n"))?;
        }

        writeln!(res)?;

        let mut first = false;
        for (value, field) in field_handles.iter().zip(config.field.iter()) {
            if !first {
                writeln!(res)?;
            }
            first = false;

            writeln!(res, "{}: {}", field.name, value.borrow().lines().join("\n"))?;
        }

        Ok(res)
    })?;

    //save to log, clipboard
    println!("{}", res);

    let res_clone = res.clone();

    let clipboard_thread = std::thread::spawn(move || {
        let mut clipboard = Clipboard::new().unwrap();

        clipboard.set_text(&res_clone).unwrap();

        std::thread::sleep(Duration::new(0, 100_000_000));

        println!("Saved results to clipboard");
    });

    let save_logs = || -> anyhow::Result<()> {
        let dir = LOG_DIR.to_string().replace("{USER}", whoami::username()?.as_str());
        fs::create_dir_all(&dir).map_err(|e| anyhow!("Failed to create log dir ({}): {}", dir, e))?;


        //todo: compress old logs and only keep today's log
        let file_path = PathBuf::from_str(&dir)?
            .join(format!("log_{}.txt", Local::now().format("%Y-%m-%d_%H-%M")));

        fs::write(&file_path, res).map_err(|e| anyhow!("Failed to save log due to error: {}", e))?;

        println!("\nSaved log to {:?}", file_path);

        Ok(())
    };

    let _ = save_logs().map_err(|e| println!("Failed to save logs due to error: {}", e));

    clipboard_thread.join().unwrap();

    println!("Press enter to exit the program");
    let mut buf = String::new();
    let _ = stdin().read_line(&mut buf);

    Ok(())
}
