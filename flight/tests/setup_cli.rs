// SPDX-License-Identifier: MIT

//! Guided setup against the real binary (ADR-012): with no terminal nothing is asked or run, and
//! on a terminal nothing is run without an explicit yes. The package manager is a script that
//! records what it was asked to do, and the PATH holds nothing else, so "it never ran" is
//! observable.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct World {
    root: PathBuf,
    bin: PathBuf,
    config: PathBuf,
    log: PathBuf,
}

/// A PATH with a recording `brew` and a stand-in `claude`, and no tmux: `brew install tmux`
/// puts a stand-in tmux there.
fn world(name: &str) -> World {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("setup-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let log = root.join("brew.log");
    let brew = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\nif [ \"$1 $2\" = \"install tmux\" ]; then\n  printf '#!/bin/sh\\necho \"tmux 3.5a\"\\n' > '{bin}/tmux'\n  /bin/chmod 755 '{bin}/tmux'\nfi\n",
        log = log.display(),
        bin = bin.display()
    );
    for (file, body) in [
        ("brew", brew.as_str()),
        ("claude", "#!/bin/sh\nexec sleep 3600\n"),
    ] {
        let p = bin.join(file);
        std::fs::write(&p, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    World {
        config: root.join("config"),
        root,
        bin,
        log,
    }
}

fn plain(w: &World, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_flight"))
        .args(args)
        .env("PATH", &w.bin)
        .env("FLIGHT_CONFIG_DIR", &w.config)
        .env_remove("FLIGHT_NO_SETUP")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Run flight in a terminal; for each `(wait, send)` pair wait until the output shows `wait`,
/// then type `send`. Returns whether it exited successfully, and everything it printed.
fn on_terminal(
    w: &World,
    args: &[&str],
    env: &[(&str, &str)],
    script: &[(&str, &str)],
) -> (bool, String) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_flight"));
    cmd.args(args);
    cmd.env("PATH", &w.bin);
    cmd.env("FLIGHT_CONFIG_DIR", &w.config);
    cmd.env_remove("FLIGHT_NO_SETUP");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 || tx.send(buf.get(..n).unwrap_or_default().to_vec()).is_err() {
                break;
            }
        }
    });
    let mut seen = String::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut next = 0;
    let mut status = None;
    while Instant::now() < deadline {
        while let Ok(chunk) = rx.try_recv() {
            seen.push_str(&String::from_utf8_lossy(&chunk));
        }
        if let Some((wait, send)) = script.get(next) {
            if seen.contains(wait) {
                writer.write_all(send.as_bytes()).unwrap();
                writer.flush().unwrap();
                next += 1;
                continue;
            }
        }
        if let Ok(Some(s)) = child.try_wait() {
            status = Some(s);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let status = status.unwrap_or_else(|| {
        let _ = child.kill();
        panic!("flight did not finish; so far:\n{seen}");
    });
    std::thread::sleep(Duration::from_millis(100));
    while let Ok(chunk) = rx.try_recv() {
        seen.push_str(&String::from_utf8_lossy(&chunk));
    }
    assert_eq!(
        next,
        script.len(),
        "the script was not used up; output:\n{seen}"
    );
    (status.success(), seen)
}

fn brew_calls(w: &World) -> String {
    std::fs::read_to_string(&w.log).unwrap_or_default()
}

fn exists(p: &Path) -> bool {
    p.exists()
}

#[test]
fn without_a_terminal_setup_check_reports_and_runs_nothing() {
    let w = world("check");
    let (ok, out, _) = plain(&w, &["setup", "--check"]);
    assert!(!ok, "tmux is missing, so --check must fail");
    assert!(out.contains("tmux: not found"), "{out}");
    assert!(out.contains("brew install tmux"), "{out}");
    assert_eq!(brew_calls(&w), "", "--check must run nothing");
    assert!(!exists(&w.config.join("settings.toml")));
}

#[test]
fn without_a_terminal_setup_itself_refuses_to_ask() {
    let w = world("noterm");
    let (ok, _, err) = plain(&w, &["setup"]);
    assert!(!ok);
    assert!(err.contains("needs a terminal"), "{err}");
    assert_eq!(brew_calls(&w), "");
}

#[test]
fn a_plain_flight_without_a_terminal_behaves_as_in_1_0_nothing_asked_nothing_run() {
    let w = world("solo-noterm");
    let (ok, _, err) = plain(&w, &["solo"]);
    assert!(!ok);
    assert!(err.contains("tmux is needed"), "{err}");
    assert_eq!(brew_calls(&w), "");
    assert!(!exists(&w.config), "nothing is created");
}

#[test]
fn declining_the_offer_runs_nothing_is_remembered_and_flight_still_refuses_clearly() {
    let w = world("decline");
    let (ok, out) = on_terminal(&w, &["solo"], &[], &[("Set it up now?", "n\n")]);
    assert!(!ok);
    assert!(out.contains("tmux is needed"), "{out}");
    assert_eq!(brew_calls(&w), "");
    let saved = std::fs::read_to_string(w.config.join("settings.toml")).unwrap();
    assert!(saved.contains("declined = true"), "{saved}");
    // Not offered again.
    let (_, again) = on_terminal(&w, &["solo"], &[], &[]);
    assert!(!again.contains("Set it up now?"), "{again}");
}

#[test]
fn the_offer_can_be_switched_off() {
    let w = world("off");
    let (_, flag) = on_terminal(&w, &["solo", "--no-setup"], &[], &[]);
    assert!(!flag.contains("Set it up now?"), "{flag}");
    let (_, var) = on_terminal(&w, &["solo"], &[("FLIGHT_NO_SETUP", "1")], &[]);
    assert!(!var.contains("Set it up now?"), "{var}");
    assert_eq!(brew_calls(&w), "");
}

#[test]
fn a_yes_runs_the_shown_command_once_and_the_result_is_checked() {
    let w = world("yes");
    let (ok, out) = on_terminal(
        &w,
        &["setup"],
        &[],
        &[("Choose 1-4", "1\n"), ("Run it?", "y\n")],
    );
    assert!(ok, "{out}");
    assert!(out.contains("command: brew install tmux"), "{out}");
    assert_eq!(brew_calls(&w), "install tmux\n");
    assert!(out.contains("Ready."), "{out}");
    assert!(exists(&w.bin.join("tmux")));
    let _ = std::fs::remove_dir_all(&w.root);
}

#[test]
fn a_no_to_the_command_leaves_the_machine_alone_and_setup_says_it_is_incomplete() {
    let w = world("no");
    let (ok, out) = on_terminal(
        &w,
        &["setup"],
        &[],
        &[("Choose 1-4", "1\n"), ("Run it?", "n\n")],
    );
    assert!(!ok, "{out}");
    assert_eq!(brew_calls(&w), "");
    assert!(!exists(&w.bin.join("tmux")));
}
