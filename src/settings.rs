//
// snowpatch - continuous integration for patch-based workflows
//
// Copyright (C) 2016 IBM Corporation
// Authors:
//     Russell Currey <ruscur@russell.cc>
//     Andrew Donnellan <andrew.donnellan@au1.ibm.com>
//
// This program is free software; you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation; either version 2 of the License, or (at your option)
// any later version.
//
// settings.rs - handle snowpatch settings parsing from TOML
//

use toml;

use serde::{self, Deserializer};

use shellexpand;

use git2::{Repository, Error};

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;
use std::path::PathBuf;

// TODO: Give more informative error messages when we fail to parse.

#[derive(Deserialize, Clone)]
pub struct Git {
    pub user: String,
    // TODO deserialize with for option<pathbuf>
    //#[serde(deserialize_with = "pathbuf_deserialize_option")]
    pub public_key: Option<PathBuf>,
    #[serde(deserialize_with = "pathbuf_deserialize")]
    pub private_key: PathBuf,
    pub passphrase: Option<String>
}

#[derive(Deserialize, Clone)]
pub struct Patchwork {
    pub url: String,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub polling_interval: u64,
}

// TODO: make this CI server agnostic (i.e buildbot or whatever)
#[derive(Deserialize, Clone)]
pub struct Jenkins {
    pub url: String,
    pub port: Option<u16>,
    // TODO: fail if we only get one of username or token
    pub username: Option<String>,
    pub token: Option<String>
}

#[derive(Deserialize, Clone)]
pub struct Project {
    #[serde(deserialize_with = "pathbuf_deserialize")]
    pub repository: PathBuf,
    pub branches: Vec<String>,
    pub test_all_branches: Option<bool>,
    pub remote_name: String,
    pub remote_uri: String,
    pub jobs: Vec<BTreeMap<String, String>>,
    pub push_results: bool
}

impl Project {
    pub fn get_repo(&self) -> Result<Repository, Error> {
        Repository::open(&self.repository)
    }
}


fn pathbuf_deserialize<D: Deserializer>(d: D) -> Result<PathBuf, D::Error> {
    let deserialised = serde::Deserialize::deserialize(d);
    match deserialised {
        Ok(toml::Value::String(ref s)) => {
            let expanded = shellexpand::full(s).unwrap(); // TODO unwrap
            Ok(PathBuf::from((*expanded).to_string()))
        },
        _ => Err(serde::de::Error::custom("Path needs to be a string")),
    }
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub git: Git,
    pub patchwork: Patchwork,
    pub jenkins: Jenkins,
    pub projects: BTreeMap<String, Project>
}

pub fn get_job_title(job: &BTreeMap<String, String>) -> String {
    match job.get("title") {
        Some(title) => title.to_string(),
        None => job.get("job").unwrap().to_string()
    }
}

pub fn parse(path: String) -> Config {
    let mut toml_config = String::new();

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => panic!("Couldn't open config file, exiting.")
    };

    file.read_to_string(&mut toml_config)
        .unwrap_or_else(|err| panic!("Couldn't read config: {}", err));

    let toml_config = toml::de::from_str::<Config>(&toml_config);

    match toml_config {
        Ok(config_inside) => config_inside, // Value::Table(config_inside),
        Err(err) => {
            error!("TOML error: {:?}", err);
            panic!("Syntax error, exiting");
            // TODO: Restore multi-error functionality
        }
    }
    /*if toml.is_none() {
        for err in &parser.errors {
            let (loline, locol) = parser.to_linecol(err.lo);
            let (hiline, hicol) = parser.to_linecol(err.hi);
            error!("TOML parsing error: {} in {} at {}:{}-{}:{}",
                    err.desc, path, loline, locol, hiline, hicol);
        }
        panic!("Syntax error in TOML file, exiting.");
    }*/

    //match Config::deserialize(&mut toml::Decoder::new(config)) {
    //    Ok(t) => t,
    //    Err(err) => panic!(format!("Couldn't deserialise config: {:?}", err))
    //}
}
