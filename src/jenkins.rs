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
// jenkins.rs - interface to Jenkins REST API
//

// TODO:
// * get Jenkins config details from somewhere
// * get status for the build
// * get artifacts + console log from completed build (do we make this configurable?)
// * integrate into snowpatch worker thread

extern crate hyper;
extern crate url;

use std::io::Read;
use std::time::Duration;
use std::thread::sleep;
use std::sync::Arc;
use std::collections::BTreeMap;

use hyper::Client;
use hyper::client::{IntoUrl, RequestBuilder};
use hyper::header::{Headers, Basic, Authorization, Location};
use serde_json::{self, Value};

use patchwork::TestState;

// Constants
const JENKINS_POLLING_INTERVAL: u64 = 5000; // Polling interval in milliseconds

// Jenkins API definitions

pub trait CIBackend { // TODO: Separate out
    fn start_test(&self, job_name: &str, params: Vec<(&str, &str)>) -> Result<String, &'static str>;
}

pub struct JenkinsBackend {
    pub base_url: String,
    pub hyper_client: Arc<Client>,
    pub username: Option<String>,
    pub token: Option<String>,
}

impl CIBackend for JenkinsBackend {
    /// Start a Jenkins build
    ///
    /// # Failures
    ///
    /// Returns Err when HTTP request fails or when no Location: header is returned
    fn start_test(&self, job_name: &str, params: Vec<(&str, &str)>)
                  -> Result<String, &'static str> {
        let params = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();

        let res = self.post(&format!("{}/job/{}/buildWithParameters?{}",
                                     self.base_url, job_name, params))
            .send().expect("HTTP request error"); // TODO don't panic here

        match res.headers.get::<Location>() {
            Some(loc) => Ok(loc.to_string()),
            None => Err("No Location header returned"),
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum JenkinsBuildStatus {
    Running,
    Done,
}

impl JenkinsBackend {
    fn headers(&self) -> Headers {
        let mut headers = Headers::new();
        if let Some(ref username) = self.username {
            headers.set(
                Authorization(
                    Basic {
                        username: username.clone(),
                        password: self.token.clone(),
                    }
                )
            );
        }
        headers
    }

    fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.hyper_client.get(url).headers(self.headers())
    }

    fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.hyper_client.post(url).headers(self.headers())
    }

    fn get_api_json_object(&self, base_url: &str) -> Value {
        // TODO: Don't panic on failure, fail more gracefully
        let url = format!("{}api/json", base_url);
        let mut resp = self.get(&url).send().expect("HTTP request error");
        let mut result_str = String::new();
        resp.read_to_string(&mut result_str)
            .unwrap_or_else(|err| panic!("Couldn't read from server: {}", err));
        serde_json::from_str(&result_str).unwrap_or_else(
            |err| panic!("Couldn't parse JSON from Jenkins: {}", err)
        )
        //json.as_object().unwrap().clone()
    }

    pub fn get_build_url(&self, build_queue_entry: &str) -> Option<String> {
        loop {
            let entry = self.get_api_json_object(build_queue_entry);
            match entry.get("executable") {
                Some(exec) => return Some(exec
                                          .as_object() // Option<BTreeMap>
                                          .unwrap() // BTreeMap
                                          .get("url") // Option<&str> ? TODO Check comments here...
                                          .unwrap() // &str ?
                                          .as_str()
                                          .unwrap()
                                          .to_string()),
                None => sleep(Duration::from_millis(JENKINS_POLLING_INTERVAL)),
            }
        }
    }

    pub fn get_build_status(&self, build_url: &str) -> JenkinsBuildStatus {
        if self.get_api_json_object(build_url)["building"].as_bool().unwrap() {
            JenkinsBuildStatus::Running
        } else {
            JenkinsBuildStatus::Done
        }
    }

    pub fn get_build_result(&self, build_url: &str) -> Option<TestState> {
        match self.get_api_json_object(build_url).get("result").unwrap()
            .as_str() {
            None => None,
            Some(result) => match result { // TODO: Improve this...
                "SUCCESS" => Some(TestState::Success),
                "FAILURE" => Some(TestState::Fail),
                "UNSTABLE" => Some(TestState::Warning),
                _ => Some(TestState::Pending),
            },
        }
    }

    pub fn get_results_url(&self, build_url: &str, job: &BTreeMap<String, String>) -> String {
        match job.get("artifact") {
            Some(artifact) => format!("{}/artifact/{}", build_url, artifact),
            None => format!("{}/consoleText/", build_url)
        }
    }

    pub fn wait_build(&self, build_url: &str) -> JenkinsBuildStatus {
        // TODO: Implement a timeout?
        while self.get_build_status(build_url) != JenkinsBuildStatus::Done {
            sleep(Duration::from_millis(JENKINS_POLLING_INTERVAL));
        }
        JenkinsBuildStatus::Done
    }
}
