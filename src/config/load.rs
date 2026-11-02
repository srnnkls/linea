//! The built-in defaults, then the global file, then the project files from the
//! outermost directory inwards, `linea.local.toml` after `linea.toml` in each.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::{Config, Error, raw};

pub const DEFAULTS: &str = include_str!("../../assets/config/default.toml");

/// The defaults with every file that applies to `dir` merged over them.
pub fn load(global: Option<&Path>, dir: &Path) -> Result<Config, Error> {
    let mut merged = parse(DEFAULTS, "defaults")?;
    for path in global
        .map(Path::to_owned)
        .into_iter()
        .chain(project_files(dir))
    {
        match std::fs::read_to_string(&path) {
            Ok(text) => merge(&mut merged, parse(&text, &path.display().to_string())?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(Error::Read { path, source }),
        }
    }
    let raw: raw::Config = Value::Table(merged)
        .try_into()
        .map_err(|source| Error::Parse {
            origin: "linea.toml".into(),
            source,
        })?;
    Config::try_from(raw)
}

/// `linea.toml` and `linea.local.toml` in every directory from the root down to `dir`.
fn project_files(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<&Path> = dir.ancestors().collect();
    dirs.reverse();
    dirs.into_iter()
        .flat_map(|dir| [dir.join("linea.toml"), dir.join("linea.local.toml")])
        .collect()
}

fn parse(text: &str, origin: &str) -> Result<Table, Error> {
    text.parse::<Table>().map_err(|source| Error::Parse {
        origin: origin.into(),
        source,
    })
}

/// Tables merge key by key; any other value replaces the one beneath it.
pub(super) fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base)), Value::Table(over)) => merge(base, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    #[test]
    fn tables_merge_and_arrays_replace() {
        let mut base = table(
            "[line]\nsections = [\"a\", \"b\"]\nmargin = 4\n[palette.dark]\nred = \"#ff0000\"",
        );
        merge(
            &mut base,
            table("[line]\nsections = [\"c\"]\n[palette.dark]\nblue = \"#0000ff\""),
        );
        assert_eq!(base["line"]["sections"].as_array().unwrap().len(), 1);
        assert_eq!(base["line"]["margin"].as_integer(), Some(4));
        assert_eq!(base["palette"]["dark"].as_table().unwrap().len(), 2);
    }

    #[test]
    fn project_files_run_outermost_first() {
        let files = project_files(Path::new("/a/b"));
        assert_eq!(files.first().unwrap(), Path::new("/linea.toml"));
        assert_eq!(files.last().unwrap(), Path::new("/a/b/linea.local.toml"));
    }
}
