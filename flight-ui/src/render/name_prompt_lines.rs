// SPDX-License-Identifier: MIT

//! The shell-name prompt as plain lines. A pure function of the prompt.

use super::style::{bold, dim, key};
use crate::view::NamePrompt;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn name_prompt_lines(prompt: &NamePrompt) -> Vec<Line<'static>> {
    let title = match prompt.current() {
        Some(current) => format!(
            "Rename the shell \"{current}\" in {}",
            prompt.workspace_name()
        ),
        None => format!("Name a shell in {}", prompt.workspace_name()),
    };
    let mut lines = vec![
        Line::raw(""),
        Line::from(vec![Span::raw("  "), Span::styled(title, bold())]),
        Line::styled(
            "  A label for you; it changes nothing else, and need not be unique.",
            dim(),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Name: "),
            Span::styled(
                prompt.text().to_owned(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)),
        ]),
        Line::raw(""),
    ];
    lines.push(match (prompt.submitting(), prompt.error()) {
        (true, _) => Line::styled("  Saving the name…", dim()),
        (false, Some(e)) => Line::styled(format!("  {e}"), Style::default().fg(Color::Red)),
        (false, None) => Line::raw(""),
    });
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("  Enter", key()),
        Span::styled(" save (empty clears)   ", dim()),
        Span::styled("Esc", key()),
        Span::styled(" cancel", dim()),
    ]));
    lines
}
