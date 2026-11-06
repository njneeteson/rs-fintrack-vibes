//! Command-line launch options.

use super::state::View;
use std::path::PathBuf;

/// Command-line options: `fintrack [--demo] [--page <page>] [--import <file.csv>]...`
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LaunchOptions {
    /// Start in demo mode instead of opening the default database.
    pub demo: bool,
    /// Page to show first (a page key, see [`LaunchOptions::page_keys`]).
    pub page: Option<String>,
    /// CSV files to queue on the Import page.
    pub import: Vec<PathBuf>,
}

impl LaunchOptions {
    pub const USAGE: &str = "Usage: fintrack [--demo] [--page <page>] [--import <file.csv>]... [file.csv]...";

    pub fn page_keys() -> Vec<&'static str> {
        View::ALL.iter().map(|v| v.key()).collect()
    }

    /// Parse arguments (without the program name). Bare paths are treated as files to import,
    /// so FinTrack can be used with "Open with…" on CSV files.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut out = Self::default();
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            match a.as_str() {
                "--page" => {
                    let p = args.next().ok_or("--page needs a value")?;
                    if !Self::page_keys().contains(&p.as_str()) {
                        return Err(format!("unknown page '{p}'; one of: {}", Self::page_keys().join(", ")));
                    }
                    out.page = Some(p);
                }
                "--demo" => out.demo = true,
                "--import" => out.import.push(args.next().ok_or("--import needs a file")?.into()),
                "-h" | "--help" => return Err(format!("{}\nPages: {}", Self::USAGE, Self::page_keys().join(", "))),
                s if s.starts_with('-') => return Err(format!("unknown option '{s}'\n{}", Self::USAGE)),
                path => out.import.push(path.into()),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::LaunchOptions;

    fn parse(args: &[&str]) -> Result<LaunchOptions, String> {
        LaunchOptions::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn launch_options() {
        let o = parse(&["--page", "analytics", "--import", "a.csv", "b.csv"]).unwrap();
        assert_eq!(o.page.as_deref(), Some("analytics"));
        assert_eq!(o.import.len(), 2);
        assert!(!o.demo && parse(&["--demo"]).unwrap().demo);
        assert!(parse(&["--page", "nope"]).is_err());
        assert!(parse(&["--page"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
        assert_eq!(parse(&[]).unwrap(), LaunchOptions::default());
    }
}
