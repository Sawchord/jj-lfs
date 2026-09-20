use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    fs::remove_dir_all("web/docs/src/content/docs")?;
    Command::new("cp")
        .args(["-r", "docs", "web/docs/src/content/"])
        .output()?;

    let output = Command::new("find")
        .args(["web/docs/src/content/docs/", "-type", "f", "-name", "*.md"])
        .output()?;

    for file in String::try_from(output.stdout)?.split_whitespace() {
        process_file(file)?;
    }

    // restore special files
    let status = Command::new("jj")
        .args([
            "--quiet",
            "restore",
            "web/docs/src/content/docs/changelog.md",
            "web/docs/src/content/docs/cli-reference.md",
            "web/docs/src/content/docs/governance/GOVERNANCE.md",
        ])
        .status()?;
    assert!(status.success());

    Ok(())
}

fn process_file(file: &str) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(file)?;
    let mut buf = String::new();
    let mut indent = None; // also indicates if currently processing "aside"
    let mut lines = content.lines().peekable();
    if let Some(first_line) = lines.next() {
        if let Some(title) = first_line
            .strip_prefix("# ")
            // There are outliers (e.g. docs/core_tenets.md) which start with a
            // level 2 heading.
            .or_else(|| first_line.strip_prefix("## "))
        {
            // starlight uses frontmatter for titles.
            writeln!(buf, "---")?;
            if title.contains(['\'', '`']) {
                writeln!(buf, "title: \"{title}\"")?;
            } else {
                writeln!(buf, "title: {title}")?;
            }
            writeln!(buf, "---")?;
        } else {
            writeln!(buf, "{first_line}")?;
        }
    }
    while let Some(line) = lines.next() {
        if line.trim().starts_with("!!!") {
            // mkdocs "admonition" detected, translate to
            // starlight "aside"
            let (ind, rest) = line.split_once("!!! ").unwrap();
            indent = Some(ind);
            let (kind, title) = match rest.split_once(' ') {
                Some((kind, title)) => (kind, Some(&title[1..title.len() - 1])),
                None => (rest, None),
            };
            let kind = match kind {
                "warning" => "caution",
                _ => kind,
            };
            if let Some(title) = title {
                writeln!(buf, "{ind}:::{kind}[{title}]")?;
            } else {
                writeln!(buf, "{ind}:::{kind}")?;
            }
            if lines.peek() == Some(&"") {
                lines.next();
            }
            continue;
        }
        if line.is_empty() {
            writeln!(buf)?;
            continue;
        }
        let Some(ind) = indent else {
            // not currently processing aside.
            writeln!(buf, "{line}")?;
            continue;
        };
        if !line.starts_with(&format!("{ind}    ")) {
            // line isn't indented enough, end of aside reached.
            indent = None;
            buf.pop(); // remove unneeded empty line
            writeln!(buf, "{ind}:::")?;
            writeln!(buf)?;
            writeln!(buf, "{line}")?;
            continue;
        }
        // still inside "aside" block, write line but strip indentation
        let line = line.strip_prefix("    ").unwrap();
        writeln!(buf, "{line}")?;
    }
    fs::write(file, buf)?;
    Ok(())
}
