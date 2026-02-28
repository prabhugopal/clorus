use std::env;
use std::process::Command;
use std::time::{Duration, Instant};

fn get_timeout() -> Duration {
    let ms = env::var("CORAL_GALLERY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(15_000);
    Duration::from_millis(ms)
}

#[test]
#[ignore = "Requires local coral gallery project path and a GUI environment"]
fn coral_gallery_main_does_not_crash() {
    let gallery_path = match env::var("CORAL_GALLERY_PATH") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Skipping: CORAL_GALLERY_PATH not set");
            return;
        }
    };

    let clorus_bin = match env::var("CARGO_BIN_EXE_clorus") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Skipping: CARGO_BIN_EXE_clorus not set by cargo");
            return;
        }
    };

    let timeout = get_timeout();

    let mut child = Command::new(clorus_bin)
        .args(["run", "--jit"])
        .current_dir(&gallery_path)
        .spawn()
        .expect("Failed to spawn clorus run --jit");

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("Failed to poll clorus process") {
            if !status.success() {
                panic!("clorus run --jit exited with status: {}", status);
            }
            break;
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            panic!(
                "clorus run --jit did not exit within {}ms (close the window or increase CORAL_GALLERY_TIMEOUT_MS)",
                timeout.as_millis()
            );
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
