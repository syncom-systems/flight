// SPDX-License-Identifier: MIT

//! The whole flow against a pretend machine and a scripted person: what runs, and what never does.

use super::console::Answer;
use super::flow::{run, Mode, Outcome};
use super::{Console, Machine, Platform, Settings, SettingsStore, Step};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

struct Pretend {
    platform: Platform,
    tools: HashSet<&'static str>,
    tmux: Option<&'static str>,
    ran: Vec<String>,
    /// What running `brew install tmux` does to the machine, and how it exits.
    install_works: bool,
    exit: i32,
}

impl Pretend {
    fn mac(tools: &[&'static str], tmux: Option<&'static str>) -> Self {
        Self {
            platform: Platform::MacOs,
            tools: tools.iter().copied().collect(),
            tmux,
            ran: Vec::new(),
            install_works: true,
            exit: 0,
        }
    }
}

impl Machine for Pretend {
    fn platform(&self) -> Platform {
        self.platform
    }
    fn which(&self, program: &str) -> bool {
        self.tools.contains(program)
    }
    fn tmux_version(&self) -> Option<String> {
        self.tmux.map(str::to_owned)
    }
    fn run(&mut self, step: &Step) -> Result<i32, String> {
        self.ran.push(step.command_line());
        if self.exit == 0 && self.install_works {
            match step.command_line().as_str() {
                "brew install tmux" => self.tmux = Some("tmux 3.5a"),
                "brew install --cask claude-code" => {
                    self.tools.insert("claude");
                }
                _ => {}
            }
        }
        Ok(self.exit)
    }
}

#[derive(Default)]
struct Scripted {
    confirms: VecDeque<Answer>,
    lines: VecDeque<String>,
    said: Vec<String>,
    asked: usize,
}

impl Console for Scripted {
    fn say(&mut self, text: &str) {
        self.said.push(text.to_owned());
    }
    fn confirm(&mut self, _question: &str) -> Answer {
        self.asked += 1;
        self.confirms.pop_front().unwrap_or(Answer::No)
    }
    fn line(&mut self, _question: &str) -> Option<String> {
        self.lines.pop_front()
    }
}

fn yes(n: usize) -> Scripted {
    Scripted {
        confirms: std::iter::repeat_n(Answer::Yes, n).collect(),
        ..Scripted::default()
    }
}

fn store(name: &str) -> (SettingsStore, PathBuf) {
    let dir = std::env::temp_dir().join(format!("flight-setup-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    (SettingsStore::in_dir(&dir), dir)
}

fn said(c: &Scripted) -> String {
    c.said.join("\n")
}

#[test]
fn nothing_runs_without_a_yes_and_end_of_input_is_a_no() {
    let (st, dir) = store("noyes");
    let mut m = Pretend::mac(&["brew", "claude"], None);
    let mut c = Scripted::default();
    let out = run(Mode::Asked, &mut m, &mut c, &st);
    assert!(m.ran.is_empty(), "ran {:?}", m.ran);
    assert_eq!(out, Outcome::Incomplete);
    assert!(said(&c).contains("command: brew install tmux"));
    let mut c = Scripted {
        confirms: VecDeque::from([Answer::No]),
        ..Scripted::default()
    };
    run(Mode::Asked, &mut m, &mut c, &st);
    assert!(m.ran.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_yes_runs_exactly_the_command_that_was_shown_and_success_is_checked_by_looking() {
    let (st, dir) = store("yes");
    let mut m = Pretend::mac(&["brew", "claude"], None);
    let mut c = yes(1);
    let out = run(Mode::Asked, &mut m, &mut c, &st);
    assert_eq!(m.ran, vec!["brew install tmux"]);
    assert_eq!(out, Outcome::Ready);
    assert!(said(&c).contains("tmux: 3.5"));
    let saved = st.load().unwrap().unwrap();
    assert!(saved.setup.completed);
    assert_eq!(saved.agents.enabled, vec!["claude"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_command_that_exits_zero_but_changes_nothing_is_not_success() {
    let (st, dir) = store("liar");
    let mut m = Pretend::mac(&["brew", "claude"], None);
    m.install_works = false;
    let mut c = yes(1);
    assert_eq!(run(Mode::Asked, &mut m, &mut c, &st), Outcome::Incomplete);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failed_command_is_reported_and_not_retried() {
    let (st, dir) = store("fail");
    let mut m = Pretend::mac(&["brew", "claude"], None);
    m.exit = 1;
    let mut c = yes(5);
    let out = run(Mode::Asked, &mut m, &mut c, &st);
    assert_eq!(m.ran, vec!["brew install tmux"], "one attempt only");
    assert_eq!(out, Outcome::Incomplete);
    assert!(said(&c).contains("exit status 1"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn each_command_needs_its_own_yes() {
    let (st, dir) = store("each");
    let mut m = Pretend::mac(&["brew"], None);
    let mut c = Scripted {
        confirms: VecDeque::from([Answer::Yes, Answer::No]),
        lines: VecDeque::from(["1".to_owned()]),
        ..Scripted::default()
    };
    run(Mode::Asked, &mut m, &mut c, &st);
    assert_eq!(m.ran, vec!["brew install tmux"], "the second was declined");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn check_mode_asks_nothing_runs_nothing_and_says_what_is_missing() {
    let (st, dir) = store("check");
    let mut m = Pretend::mac(&["brew", "claude"], None);
    let mut c = yes(5);
    let out = run(Mode::Check, &mut m, &mut c, &st);
    assert!(m.ran.is_empty());
    assert_eq!(c.asked, 0);
    assert_eq!(out, Outcome::Incomplete);
    assert!(said(&c).contains("brew install tmux"));
    let mut ok = Pretend::mac(&["claude"], Some("tmux 3.4"));
    assert_eq!(
        run(Mode::Check, &mut ok, &mut Scripted::default(), &st),
        Outcome::Ready
    );
    assert!(!dir.join("settings.toml").exists(), "check writes nothing");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn declining_the_offer_runs_nothing_and_is_remembered() {
    let (st, dir) = store("declined");
    let mut m = Pretend::mac(&["brew"], None);
    let mut c = Scripted::default();
    assert_eq!(run(Mode::Offered, &mut m, &mut c, &st), Outcome::Declined);
    assert!(m.ran.is_empty());
    assert!(st.load().unwrap().unwrap().setup.declined);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn without_homebrew_on_a_mac_setup_explains_and_installs_nothing() {
    let (st, dir) = store("nobrew");
    let mut m = Pretend::mac(&[], None);
    let mut c = yes(5);
    run(Mode::Asked, &mut m, &mut c, &st);
    assert!(m.ran.is_empty());
    assert!(said(&c).contains("https://brew.sh"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_agents_the_person_chooses_are_remembered_and_considered() {
    let (st, dir) = store("agents");
    let mut m = Pretend::mac(&["brew"], Some("tmux 3.4"));
    let mut c = Scripted {
        confirms: VecDeque::from([Answer::No, Answer::No]),
        lines: VecDeque::from(["3".to_owned()]),
        ..Scripted::default()
    };
    run(Mode::Asked, &mut m, &mut c, &st);
    let saved = st.load().unwrap().unwrap();
    assert_eq!(saved.agents.enabled, vec!["claude", "codex"]);
    assert_eq!(saved.agents.default, "claude");
    assert!(said(&c).contains("brew install --cask claude-code"));
    assert!(said(&c).contains("brew install --cask codex"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_settings_file_that_cannot_be_used_is_never_overwritten() {
    let (st, dir) = store("newer");
    std::fs::write(dir.join("settings.toml"), "version = 9\n").unwrap();
    let mut m = Pretend::mac(&["claude"], Some("tmux 3.4"));
    run(Mode::Asked, &mut m, &mut Scripted::default(), &st);
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.toml")).unwrap(),
        "version = 9\n"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn linux_apt_is_run_with_sudo_in_the_open() {
    let (st, dir) = store("apt");
    let mut m = Pretend::mac(&["apt-get", "claude"], None);
    m.platform = Platform::Linux;
    let mut c = yes(1);
    run(Mode::Asked, &mut m, &mut c, &st);
    assert_eq!(m.ran, vec!["sudo apt-get install -y tmux"]);
    assert!(said(&c).contains("command: sudo apt-get install -y tmux"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn defaults_are_a_claude_workspace_with_nothing_done_yet() {
    let s = Settings::default();
    assert_eq!(s.agents.enabled, vec!["claude"]);
    assert!(!s.setup.completed && !s.setup.declined);
}
