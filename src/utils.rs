use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use udev::Device;

#[derive(Debug)]
pub(crate) struct NoSuchFile {
    path: PathBuf,
}

impl Display for NoSuchFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Could not find: {:?}. Maybe --wait is too short (or there is a typo)?",
            self.path
        )
    }
}

impl Error for NoSuchFile {}

/// Wait for a file to show up
pub(crate) fn wait_for_file(path: &Path, timeout: Duration) -> Result<(), NoSuchFile> {
    let last_time = Instant::now() + timeout;
    while Instant::now() < last_time {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(NoSuchFile {
        path: path.to_path_buf(),
    })
}

pub fn get_devnode_if_monitored<'a>(device: &'a Device, cli_inputs: &Vec<PathBuf>) -> Option<&'a Path> {
    let devnode = device.devnode()?;

    if cli_inputs.iter()
        .filter(|i| fs::exists(i).is_ok_and(|exists| exists))
        .map(|i| fs::canonicalize(i).unwrap())
        .any(|i| &i == devnode)
    {
        return Some(devnode);
    }

    get_devnode_if_default(device)
}

pub fn get_devnode_if_default(device: &Device) -> Option<&Path> {
    let devnode = device.devnode()?;

    if !devnode
        .file_name().unwrap().to_str().unwrap()
        .starts_with("event")
    {
        return None;
    }

    const MONITORED_PROPERTIES: [&str; 4] = [
        "ID_INPUT_KEYBOARD",
        //"ID_INPUT_KEY",
        "ID_INPUT_MOUSE",
        "ID_INPUT_TOUCHPAD",
        //"ID_INPUT_TOUCHSCREEN",
        //"ID_INPUT_TABLET",
        "ID_INPUT_JOYSTICK",
        //"ID_INPUT_ACCELEROMETER"
    ];

    if device.properties()
        .map(|prop| prop.name().to_string_lossy().to_string())
        .filter(|prop_name| MONITORED_PROPERTIES.contains(&prop_name.as_str()))
        .next().is_none()
    {
        return None;
    }

    Some(devnode)
}

pub fn get_default_devices() -> anyhow::Result<Vec<PathBuf>> {
    let mut default_devices = vec![];

    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("input")?;

    for device in enumerator.scan_devices()? {
        if let Some(devnode) = get_devnode_if_default(&device) {
            default_devices.push(devnode.to_path_buf());
        }
    }

    Ok(default_devices)
}

pub fn normalize_devices(
    mut provided_device_paths: Vec<PathBuf>,
    mut default_device_paths: Vec<PathBuf>
) -> anyhow::Result<Vec<PathBuf>> {
    let mut unique_canons = HashSet::new();
    for path in &provided_device_paths {
        // Disallow duplicates in provided inputs
        let canon = fs::canonicalize(path)?;
        if !unique_canons.insert(canon.clone()) {
            anyhow::bail!("Provided input devices have duplicates (provided: {}; its canon: {}.",
                path.to_string_lossy(),
                canon.to_string_lossy());
        }

        // Remove the corresponding duplicate (if any)
        if let Some(idx) = default_device_paths.iter().position(|p| p == &canon.as_path()) {
            default_device_paths.swap_remove(idx);
        }
    }

    provided_device_paths.append(&mut default_device_paths);
    Ok(provided_device_paths)
}
