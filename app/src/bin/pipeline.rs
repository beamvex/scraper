use anyhow::{bail, Context, Result};
use std::cmp::Reverse;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let data_dir = env::args().nth(1).unwrap_or_else(|| "../data".to_string());
    let data_dir = PathBuf::from(data_dir);
    if !data_dir.is_dir() {
        bail!("data directory does not exist: {}", data_dir.display());
    }

    run_cargo_bin("scraper", &[])?;

    let mut pages = Vec::new();
    find_page_html(&data_dir, &data_dir, &mut pages)?;
    let mut pages: Vec<_> = pages
        .into_iter()
        .map(|p| {
            let t = fs::metadata(&p)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (t, p)
        })
        .collect();
    pages.sort_by_key(|(t, _)| Reverse(*t));

    if let Some((_, page)) = pages.first() {
        let md = page.with_file_name("output.md");
        let review = page.with_file_name("output.review.md");
        let html = page.with_file_name("index.html");
        println!("processing {}", page.display());
        run_html2md(page, &md)?;
        run_md2review(&md, &review)?;
        run_review2html(&review, &html)?;
    }

    Ok(())
}

fn find_page_html(_base: &Path, dir: &Path, pages: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            find_page_html(_base, &path, pages)?;
        } else if path.file_name().and_then(|n| n.to_str()) == Some("page.html") {
            pages.push(path);
        }
    }
    Ok(())
}

fn run_html2md(input: &Path, output: &Path) -> Result<()> {
    run_cargo_bin(
        "html2md",
        &[
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        ],
    )
}

fn run_md2review(input: &Path, output: &Path) -> Result<()> {
    run_cargo_bin(
        "md2review",
        &[
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        ],
    )
}

fn run_review2html(input: &Path, output: &Path) -> Result<()> {
    run_cargo_bin(
        "review2html",
        &[
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        ],
    )
}

fn run_cargo_bin(name: &str, args: &[String]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--bin", name, "--"]);
    for a in args {
        cmd.arg(a);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to start cargo run --bin {}", name))?;
    if !status.success() {
        bail!(
            "{} failed with exit code {}",
            name,
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
