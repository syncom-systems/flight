// SPDX-License-Identifier: MIT

use crate::{RemoveError, Tmux, TmuxRunner};

impl<R: TmuxRunner> Tmux<R> {
    /// Remove the window of a session that carries `surface_id`, and nothing else. The session
    /// and the window are found here, in one listing, by the surface id the window was marked
    /// with; the window is then killed by its own tmux id. The session's last window is never
    /// removed: that would end the session.
    pub fn remove_surface_window(
        &self,
        session_id: &str,
        surface_id: &str,
    ) -> Result<(), RemoveError> {
        let listing = self.runner().run(&[
            "list-windows",
            "-t",
            session_id,
            "-F",
            "#{window_id}\t#{@flight_surface_id}",
        ])?;
        let windows: Vec<(&str, &str)> = listing
            .stdout
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .collect();
        let (id, _) = windows
            .iter()
            .find(|(_, marked)| !surface_id.is_empty() && *marked == surface_id)
            .ok_or(RemoveError::NotFound)?;
        if windows.len() < 2 {
            return Err(RemoveError::OnlyWindow);
        }
        self.runner().run(&["kill-window", "-t", id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TmuxError, TmuxOutput};
    use std::cell::RefCell;

    struct Script {
        calls: RefCell<Vec<Vec<String>>>,
        windows: &'static str,
    }

    impl TmuxRunner for &Script {
        fn run(&self, args: &[&str]) -> Result<TmuxOutput, TmuxError> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|s| (*s).to_owned()).collect());
            Ok(TmuxOutput {
                stdout: if args.first() == Some(&"list-windows") {
                    self.windows.to_owned()
                } else {
                    String::new()
                },
            })
        }
    }

    fn remove(windows: &'static str, surface: &str) -> (Result<(), RemoveError>, Vec<String>) {
        let script = Script {
            calls: RefCell::default(),
            windows,
        };
        let result = Tmux::with_runner(&script).remove_surface_window("$1", surface);
        let kills = script
            .calls
            .borrow()
            .iter()
            .filter(|c| c[0] == "kill-window")
            .map(|c| c.join(" "))
            .collect();
        (result, kills)
    }

    #[test]
    fn only_the_window_with_that_surface_id_is_killed_and_by_its_own_id() {
        let (result, kills) = remove("@1\ts-a\n@2\ts-b\n@3\ts-c\n", "s-b");
        assert!(result.is_ok());
        assert_eq!(kills, ["kill-window -t @2"]);
    }

    #[test]
    fn an_unknown_or_empty_surface_id_kills_nothing() {
        for surface in ["s-x", ""] {
            let (result, kills) = remove("@1\ts-a\n@2\t\n", surface);
            assert!(matches!(result, Err(RemoveError::NotFound)));
            assert!(kills.is_empty());
        }
    }

    #[test]
    fn the_only_window_is_never_removed() {
        let (result, kills) = remove("@1\ts-a\n", "s-a");
        assert!(matches!(result, Err(RemoveError::OnlyWindow)));
        assert!(kills.is_empty());
    }
}
