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
// git.rs - snowpatch git functionality
//

use git2::build::CheckoutBuilder;
use git2::{Branch, Commit, Cred, Error, PushOptions, Remote, Repository};

use std::path::Path;
use std::process::{Command, Output};
use std::result::Result;

use settings::Git;

pub static GIT_REF_BASE: &'static str = "refs/heads";

pub fn get_latest_commit(repo: &Repository) -> Commit {
    let head = repo.head().unwrap();
    let oid = head.target().unwrap();
    repo.find_commit(oid).unwrap()
}

pub fn push_to_remote(
    remote: &mut Remote,
    branch: &Branch,
    delete: bool,
    mut opts: &mut PushOptions,
) -> Result<(), Error> {
    let action = if delete { ":" } else { "+" };
    let refspecs: &[&str] = &[&format!(
        "{}{}/{}",
        action,
        GIT_REF_BASE,
        branch.name().unwrap().unwrap()
    )];
    remote.push(refspecs, Some(&mut opts))
}

// TODO: rewrite this to use git2-rs, I think it's impossible currently
pub fn pull(repo: &Repository) -> Result<Output, &'static str> {
    let workdir = repo.workdir().unwrap(); // TODO: support bare repositories

    let output = Command::new("git")
        .arg("pull") // pull the cool kid's way
        .current_dir(&workdir) // in the repo's working directory
        .output() // run synchronously
        .unwrap(); // TODO

    if output.status.success() {
        debug!(
            "Pull: {}",
            String::from_utf8(output.clone().stdout).unwrap()
        );
        Ok(output)
    } else {
        Err("Error: couldn't pull changes")
    }
}

pub fn checkout_branch(repo: &Repository, branch: &str) {
    let workdir = repo.workdir().unwrap(); // TODO: support bare repositories

    // Make sure there's no junk lying around before we switch
    Command::new("git")
        .arg("reset")
        .arg("--hard")
        .current_dir(&workdir)
        .output()
        .unwrap();

    Command::new("git")
        .arg("clean")
        .arg("-f") // force remove files we don't need
        .arg("-d") // ...including directories
        .current_dir(&workdir)
        .output()
        .unwrap();

    repo.set_head(&format!("{}/{}", GIT_REF_BASE, &branch))
        .unwrap_or_else(|err| panic!("Couldn't set HEAD: {}", err));
    repo.checkout_head(Some(&mut CheckoutBuilder::new().force()))
        .unwrap_or_else(|err| panic!("Couldn't checkout HEAD: {}", err));

    // Clean up again, just to be super sure
    Command::new("git")
        .arg("reset")
        .arg("--hard")
        .current_dir(&workdir)
        .output()
        .unwrap();

    Command::new("git")
        .arg("clean")
        .arg("-f") // force remove files we don't need
        .arg("-d") // ...including directories
        .current_dir(&workdir)
        .output()
        .unwrap();
}

pub fn apply_patch(repo: &Repository, path: &Path) -> Result<Output, &'static str> {
    let workdir = repo.workdir().unwrap(); // TODO: support bare repositories

    // We call out to "git am" since libgit2 doesn't implement "am"
    let output = Command::new("git")
        .arg("am") // apply from mbox
        .arg("-3") // three way merge
        .arg(&path) // from our mbox file
        .current_dir(&workdir) // in the repo's working directory
        .output() // run synchronously
        .unwrap(); // TODO

    if output.status.success() {
        debug!(
            "Patch applied with text {}",
            String::from_utf8(output.clone().stdout).unwrap()
        );
        Ok(output)
    } else {
        info!(
            "Patch failed to apply with text {} {}",
            String::from_utf8(output.clone().stdout).unwrap(),
            String::from_utf8(output.clone().stderr).unwrap()
        );
        Command::new("git")
            .arg("am")
            .arg("--abort")
            .current_dir(&workdir)
            .output()
            .unwrap();
        Err("Patch did not apply successfully")
    }
}

pub fn cred_from_settings(settings: &Git) -> Result<Cred, Error> {
    // We have to convert from Option<String> to Option<&str>
    let public_key = settings.public_key.as_ref().map(String::as_ref);
    let passphrase = settings.passphrase.as_ref().map(String::as_ref);

    Cred::ssh_key(
        &settings.user,
        public_key,
        Path::new(&settings.private_key),
        passphrase,
    )
}

pub fn find_commit_with_title(repo: &Repository, title: &str) -> bool {
    let workdir = repo.workdir().unwrap();

    // TODO: Perhaps limit this to a certain number of commits, etc
    let result = Command::new("git")
        .arg("log")
        .arg("--oneline")
        .arg("--grep")
        .arg(title)
        .current_dir(&workdir)
        .output()
        .expect("Couldn't run git");
    !result.stdout.is_empty()
}
