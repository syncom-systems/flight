// SPDX-License-Identifier: MIT

use crate::{RemoveError, Tmux, TmuxRunner, SURFACE_NAME_OPTION};

impl<R: TmuxRunner> Tmux<R> {
    /// Set the label of the window of a session that carries `surface_id`, or clear it with
    /// `None`. The window is found here, in one listing, by the surface id it was marked with
    /// and then addressed by its own tmux id. Only the label option is written: the window's
    /// tmux name, which the backend addresses it by, is not touched, and `name` is never used
    /// to find anything. The caller has already checked that it is a clean label.
    pub fn rename_surface_window(
        &self,
        session_id: &str,
        surface_id: &str,
        name: Option<&str>,
    ) -> Result<(), RemoveError> {
        let listing = self.runner().run(&[
            "list-windows",
            "-t",
            session_id,
            "-F",
            "#{window_id}\t#{@flight_surface_id}",
        ])?;
        let (id, _) = listing
            .stdout
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .find(|(_, marked)| !surface_id.is_empty() && *marked == surface_id)
            .ok_or(RemoveError::NotFound)?;
        match name {
            Some(name) => {
                self.runner()
                    .run(&["set-option", "-w", "-t", id, SURFACE_NAME_OPTION, name])?
            }
            None => {
                self.runner()
                    .run(&["set-option", "-w", "-u", "-t", id, SURFACE_NAME_OPTION])?
            }
        };
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
    }

    impl TmuxRunner for &Script {
        fn run(&self, args: &[&str]) -> Result<TmuxOutput, TmuxError> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|s| (*s).to_owned()).collect());
            Ok(TmuxOutput {
                stdout: if args.first() == Some(&"list-windows") {
                    "@1\ts-a\n@2\ts-b\n@3\t\n".to_owned()
                } else {
                    String::new()
                },
            })
        }
    }

    fn writes(surface: &str, name: Option<&str>) -> (Result<(), RemoveError>, Vec<String>) {
        let script = Script {
            calls: RefCell::default(),
        };
        let result = Tmux::with_runner(&script).rename_surface_window("$1", surface, name);
        let sets = script
            .calls
            .borrow()
            .iter()
            .filter(|c| c[0] == "set-option")
            .map(|c| c.join(" "))
            .collect();
        (result, sets)
    }

    #[test]
    fn only_the_label_of_the_window_with_that_surface_id_is_written_and_by_its_own_id() {
        let (result, sets) = writes("s-b", Some("logs"));
        assert!(result.is_ok());
        assert_eq!(sets, ["set-option -w -t @2 @flight_surface_name logs"]);
    }

    #[test]
    fn clearing_unsets_the_option() {
        let (_, sets) = writes("s-a", None);
        assert_eq!(sets, ["set-option -w -u -t @1 @flight_surface_name"]);
    }

    #[test]
    fn an_unknown_or_empty_surface_id_writes_nothing() {
        for surface in ["s-x", ""] {
            let (result, sets) = writes(surface, Some("logs"));
            assert!(matches!(result, Err(RemoveError::NotFound)));
            assert!(sets.is_empty());
        }
    }
}
