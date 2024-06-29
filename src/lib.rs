// See README.md and LICENSE for more details.

//! Get os native machine id without root permission.

//! ## About machine id
//!
//! In Linux, machine id is a single newline-terminated, hexadecimal, 32-character, lowercase ID.
//! When decoded from hexadecimal, this corresponds to a 16-byte/128-bit value.
//! This ID may not be all zeros.
//! This ID uniquely identifies the host. It should be considered "confidential",
//! and must not be exposed in untrusted environments.
//! And do note that the machine id can be re-generated by root.
//!
//! ## Usage
//!
//! ```Rust
//! extern crate machine_uid;
//!
//! fn main() {
//!     let id: String = machine_uid::get().unwrap();
//!     println!("{}", id);
//! }
//! ```
//!
//! ## How it works
//!
//! It get machine id from following source:
//!
//! Linux or who use systemd:
//!
//! ```Bash
//! cat /var/lib/dbus/machine-id # or /etc/machine-id
//! ```
//!
//! BSD:
//!
//! ```Bash
//! cat /etc/hostid # or kenv -q smbios.system.uuid
//! ```
//!
//! OSX:
//!
//! ```Bash
//! ioreg -rd1 -c IOPlatformExpertDevice | grep IOPlatformUUID
//! ```
//!
//! Windows:
//!
//! ```powershell
//! (Get-ItemProperty -Path Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography).MachineGuid
//! ```
//!
//! illumos:
//!
//! ```Bash
//! gethostid(3C)
//! ```
//!
//! ## Supported Platform
//!
//! I have tested in following platform:
//!
//! - Debian 8
//! - OS X 10.6
//! - FreeBSD 10.4
//! - Fedora 28
//! - Windows 10
//! - OmniOS r151050
//!

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

#[allow(dead_code)]
fn read_file(file_path: &str) -> Result<String, Box<dyn Error>> {
    let mut fd = File::open(file_path)?;
    let mut content = String::new();
    fd.read_to_string(&mut content)?;
    Ok(content.trim().to_string())
}

#[cfg(target_os = "linux")]
pub mod machine_id {
    use super::read_file;
    use std::error::Error;

    // dbusPath is the default path for dbus machine id.
    const DBUS_PATH: &str = "/var/lib/dbus/machine-id";
    // or when not found (e.g. Fedora 20)
    const DBUS_PATH_ETC: &str = "/etc/machine-id";

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        match read_file(DBUS_PATH) {
            Ok(machine_id) => Ok(machine_id),
            Err(_) => Ok(read_file(DBUS_PATH_ETC)?),
        }
    }
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub mod machine_id {
    use super::read_file;
    use std::error::Error;
    use std::process::Command;

    const HOST_ID_PATH: &str = "/etc/hostid";

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        match read_file(HOST_ID_PATH) {
            Ok(machine_id) => Ok(machine_id),
            Err(_) => Ok(read_from_kenv()?),
        }
    }

    fn read_from_kenv() -> Result<String, Box<dyn Error>> {
        let output = Command::new("kenv")
            .args(&["-q", "smbios.system.uuid"])
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.trim().to_string())
    }
}

#[cfg(target_os = "macos")]
mod machine_id {
    // machineID returns the uuid returned by `ioreg -rd1 -c IOPlatformExpertDevice`.
    use std::error::Error;
    use std::process::Command;

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        let output = Command::new("ioreg")
            .args(&["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        extract_id(&content)
    }

    fn extract_id(content: &str) -> Result<String, Box<dyn Error>> {
        let lines = content.split('\n');
        for line in lines {
            if line.contains("IOPlatformUUID") {
                let k: Vec<&str> = line.rsplitn(2, '=').collect();
                let id = k[0].trim_matches(|c: char| c == '"' || c.is_whitespace());
                return Ok(id.to_string());
            }
        }
        Err(From::from(
            "No matching IOPlatformUUID in `ioreg -rd1 -c IOPlatformExpertDevice` command.",
        ))
    }
}

#[cfg(target_os = "windows")]
pub mod machine_id {
    use std::error::Error;
    use std::ffi::c_int;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
    use winreg::RegKey;

    extern "C" {
        fn MachineUidIsWow64() -> c_int;
    }

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        let flag = if unsafe { MachineUidIsWow64() == 1 } && cfg!(target_pointer_width = "32") {
            KEY_READ | KEY_WOW64_64KEY
        } else {
            KEY_READ
        };

        let crypto = hklm.open_subkey_with_flags("SOFTWARE\\Microsoft\\Cryptography", flag)?;
        let id: String = crypto.get_value("MachineGuid")?;

        Ok(id.trim().to_string())
    }
}

#[cfg(target_os = "illumos")]
pub mod machine_id {
    use std::error::Error;

    /// Return machine id
    pub fn get_machine_id() -> Result<String, Box<dyn Error>> {
        Ok(format!("{:x}", unsafe { libc::gethostid() }))
    }
}

pub use machine_id::get_machine_id as get;
