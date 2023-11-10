use git2::{Repository, Tree};
use std::process::Command;

pub fn get_repo() -> Result<Repository, git2::Error> {
    Repository::discover(".")
}

pub fn staged_files(repo: &Repository) -> Result<Vec<String>, git2::Error> {
    let idx = repo.index()?;
    let mut head: Option<Tree> = None;
    if let Ok(h) = repo.head() {
        head = Some(h.peel_to_tree()?);
    }
    let diff = repo.diff_tree_to_index(head.as_ref(), Some(&idx), None)?;
    Ok(diff
        .deltas()
        .map(|d| {
            let path = d.new_file().path();
            path.map_or_else(String::new, |path| path.to_str().unwrap_or("").to_string())
        })
        .collect())
}

pub fn diff(repo: &Repository, files: &[String]) -> Result<String, git2::Error> {
    let mut ret = String::new();

    let idx = repo.index()?;
    let mut head: Option<Tree> = None;
    if let Ok(h) = repo.head() {
        head = Some(h.peel_to_tree()?);
    }
    let diff = repo.diff_tree_to_index(head.as_ref(), Some(&idx), None)?;
    diff.print(git2::DiffFormat::Patch, |delta, _, line| {
        if let Some(path) = delta.new_file().path() {
            if files.contains(&path.to_str().unwrap_or("").to_string()) {
                ret.push(line.origin());
                ret.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            }
        }
        true
    })?;
    Ok(ret)
}

// idk how this is really supposed to work
// pub fn commit(repo: &Repository, files: &[String], msg: &str) -> Result<(), git2::Error> {
//     let mut index = repo.index()?;
//     // let all_files = tracked_files(repo)?;
//     // for file in all_files {
//     //     if !files.contains(&file) {
//     //         index.remove_path(std::path::Path::new(&file))?;
//     //     }
//     // }
//     // index.write()?;
//     let oid = index.write_tree()?;
//     let parent_commit = repo.head()?.peel_to_commit()?;
//     let tree = repo.find_tree(oid)?;
//     let sig = repo.signature()?;
//     repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent_commit])?;
//     Ok(())
// }

pub fn commit(msg: String) -> anyhow::Result<()> {
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(msg)
        .output()?;
    Ok(())
}
