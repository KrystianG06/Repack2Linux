use std::path::PathBuf;
use std::process::Command;

pub struct Mounter;

impl Mounter {
    pub fn mount(iso_path: &PathBuf) -> Result<(String, String), String> {
        // 1. Setup loop device
        let output = Command::new("udisksctl")
            .arg("loop-setup")
            .arg("-f")
            .arg(iso_path)
            .output()
            .map_err(|e| format!("Failed to run udisksctl: {}", e))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: "Mapped file /path/to.iso as /dev/loopX."
        let loop_dev = stdout
            .split("as")
            .last()
            .ok_or("Failed to parse loop device from udisksctl output")?
            .trim()
            .trim_end_matches('.');

        // 2. Mount the loop device
        let mount_output = Command::new("udisksctl")
            .arg("mount")
            .arg("-b")
            .arg(loop_dev)
            .output()
            .map_err(|e| format!("Failed to mount loop device: {}", e))?;

        if !mount_output.status.success() {
            // Cleanup on failure
            let _ = Command::new("udisksctl")
                .arg("loop-delete")
                .arg("-b")
                .arg(loop_dev)
                .status();
            return Err(String::from_utf8_lossy(&mount_output.stderr).to_string());
        }

        let mount_stdout = String::from_utf8_lossy(&mount_output.stdout);
        // Output format: "Mounted /dev/loopX at /media/user/Label"
        let mount_point = mount_stdout
            .split("at")
            .last()
            .ok_or("Failed to parse mount point")?
            .trim()
            .trim_end_matches('.')
            .to_string();

        Ok((mount_point, loop_dev.to_string()))
    }

    pub fn unmount(loop_dev: &str) -> Result<(), String> {
        // 1. Unmount filesystem
        let unmount_status = Command::new("udisksctl")
            .arg("unmount")
            .arg("-b")
            .arg(loop_dev)
            .status()
            .map_err(|e| e.to_string())?;

        if !unmount_status.success() {
            return Err(format!("Failed to unmount {}", loop_dev));
        }

        // 2. Delete loop device
        let delete_status = Command::new("udisksctl")
            .arg("loop-delete")
            .arg("-b")
            .arg(loop_dev)
            .status()
            .map_err(|e| e.to_string())?;

        if !delete_status.success() {
            return Err(format!("Failed to delete loop device {}", loop_dev));
        }

        Ok(())
    }
}
