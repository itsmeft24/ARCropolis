// #![feature(proc_macro_hygiene)]

use crate::config;
use log::info;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use skyline::nn;
use skyline_web::{ramhorns, Webpage, Visibility};
use smash_arc::Hash40;
use std::collections::HashSet;
use std::ffi::CString;
use std::path::Path;
use std::path::PathBuf;

static HTML_TEXT: &str = include_str!("../../../resources/templates/configurator.html");
static CSS_TEXT: &str = include_str!("../../../resources/css/configurator.css");
static JAVASCRIPT_TEXT: &str = include_str!("../../../resources/js/configurator.js");

const LOCALHOST: &str = "http://localhost/";

#[derive(Debug, Deserialize)]
pub struct ConfigChanged {
    category: String,
    value: String,
}

// Is this trash? Yes
// Did I have a choice? No
pub fn show_config_editor() {
    let session = std::boxed::Box::new(Webpage::new()
        .htdocs_dir("contents")
        .file("index.html", HTML_TEXT)
        .file("configurator.css", CSS_TEXT)
        .file("configurator.js", JAVASCRIPT_TEXT)
        .background(skyline_web::Background::Default)
        .boot_display(skyline_web::BootDisplay::Default)
        .open_session(Visibility::Default)
        .unwrap());

        let mut storage = skyline_config::acquire_storage("arcropolis").unwrap();

        // Loaded
        let _ = session.recv();

        if storage.get_flag("debug") {
            session.send("debug");
        }

        if storage.get_flag("beta_updates") {
            session.send("beta");
        }

        let region: String = storage.get_field("region").unwrap();
        session.send(&region);


        let logging: String = storage.get_field("logging_level").unwrap();
        session.send(&logging);


        while let Ok(msg) = session.recv_json::<ConfigChanged>() {
            match msg.category.as_str() {
                "lang" => {
                    let curr_value: String = storage.get_field("region").unwrap();
                    session.send(&curr_value);
                    storage.set_field("region", &msg.value).unwrap();
                    session.send(&msg.value);
                    println!("Set region to {}", &msg.value);
                },
                "log" => {
                    let curr_value: String = storage.get_field("logging_level").unwrap();
                    session.send(&curr_value);
                    storage.set_field("logging_level", &msg.value).unwrap();
                    session.send(&msg.value);
                    println!("Set logger to {}", &msg.value);
                },
                "beta" => {
                    let curr_value = !storage.get_flag("beta_updates");
                    storage.set_flag("beta_updates", curr_value).unwrap();
                    println!("Set beta update flag to {}", curr_value);
                    session.send("beta");
                }
                "debug" => {
                    let curr_value = !storage.get_flag("debug");
                    storage.set_flag("debug", curr_value).unwrap();
                    println!("Set debug flag to {}", curr_value);
                    session.send("debug");
                },
                _ => {
                    break;
                },
            }
        }
        
        session.exit();
        session.wait_for_exit();
}
