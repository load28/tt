use std::io;
use std::path::{Component, Path, PathBuf, Prefix};

pub(crate) fn canonical(path: &Path) -> io::Result<PathBuf> {
    Ok(simplified(std::fs::canonicalize(path)?))
}

pub(crate) fn simplified(path: PathBuf) -> PathBuf {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path;
    };
    let plain = match prefix.kind() {
        Prefix::VerbatimDisk(disk) => format!("{}:\\", char::from(disk)),
        Prefix::VerbatimUNC(server, share) => format!(
            "\\\\{}\\{}\\",
            server.to_string_lossy(),
            share.to_string_lossy()
        ),
        _ => return path,
    };
    let rest: PathBuf = components
        .filter(|component| !matches!(component, Component::RootDir))
        .collect();
    let simplified = Path::new(&plain).join(&rest);
    let representable = simplified.as_os_str().len() < 260
        && simplified.components().all(|component| match component {
            Component::Normal(name) => {
                let name = name.to_string_lossy();
                !name.ends_with(' ') && !name.ends_with('.') && !is_reserved_name(&name)
            }
            _ => true,
        });
    if representable { simplified } else { path }
}

fn is_reserved_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_verbatim_disk_path_loses_its_prefix() {
        assert_eq!(
            simplified(PathBuf::from(r"\\?\C:\repo\src\a.tt")),
            PathBuf::from(r"C:\repo\src\a.tt")
        );
    }

    #[test]
    fn a_path_that_only_the_verbatim_form_can_name_keeps_it() {
        let verbatim = PathBuf::from(r"\\?\C:\repo\trailing. \a.tt");
        assert_eq!(simplified(verbatim.clone()), verbatim);
    }
}
