//! Markdown task checkboxes, collected across the whole vault.
//!
//! The checkbox parsing already existed, inlined in the landing page's counter.
//! Pulling it out means the count on the landing page and the list in the panel
//! can never disagree about what a task is — and gives the rule one place to be
//! stated and tested.

use uuid::Uuid;

use crate::models::NotebookData;

/// One checkbox, and enough context to jump back to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub note_id: Uuid,
    pub note_title: String,
    /// 0-based line within the note, for jumping the cursor there.
    pub line: usize,
    /// The task text with its bullet and checkbox stripped.
    pub text: String,
    pub done: bool,
}

/// Marks that mean something inside a checkbox.
///
/// A whitelist rather than "any single character", because shape alone cannot tell
/// a checkbox from prose: `- [a] see appendix` has exactly the same form as
/// `- [x] buy milk`. Space is open; the rest are the states Obsidian and similar
/// tools write for done, in-progress and cancelled — none of which mean "still to
/// do", so all of them count as not-open.
const TASK_MARKS: [char; 5] = [' ', 'x', 'X', '/', '-'];

/// Recognise a task line, returning `(done, text)`.
///
/// Accepts `-`, `*` and `+` bullets.
pub fn parse_task_line(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))?;

    let rest = rest.trim_start();
    let inner = rest.strip_prefix('[')?;
    let mark = inner.chars().next()?;
    if !TASK_MARKS.contains(&mark) {
        return None;
    }
    let after = inner.strip_prefix(mark)?.strip_prefix(']')?;

    // `[ ]x` is not a checkbox either: the bracket has to be followed by a space or
    // end the line.
    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }

    Some((mark != ' ', after.trim().to_string()))
}

/// Every task in the vault, skipping fenced code blocks.
///
/// Sorted by note title then line, so the list is stable between openings — a
/// panel whose rows reshuffle on every keystroke is unusable, and `notes` is a
/// HashMap with no order of its own.
pub fn collect(notebook: &NotebookData, include_done: bool) -> Vec<Task> {
    let mut tasks = Vec::new();

    for note in notebook.notes.values() {
        let mut in_code = false;
        for (line_no, line) in note.content.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if in_code {
                continue;
            }
            let Some((done, text)) = parse_task_line(line) else {
                continue;
            };
            if done && !include_done {
                continue;
            }
            tasks.push(Task {
                note_id: note.id,
                note_title: note.title.clone(),
                line: line_no,
                text,
                done,
            });
        }
    }

    tasks.sort_by(|a, b| {
        a.note_title
            .to_lowercase()
            .cmp(&b.note_title.to_lowercase())
            .then(a.line.cmp(&b.line))
    });
    tasks
}

/// How many notes the given tasks are spread across.
pub fn notes_covered(tasks: &[Task]) -> usize {
    let mut ids: Vec<Uuid> = tasks.iter().map(|t| t.note_id).collect();
    ids.sort();
    ids.dedup();
    ids.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;

    fn notebook_with(notes: &[(&str, &str)]) -> NotebookData {
        let mut nb = NotebookData::new();
        for (title, content) in notes {
            let mut note = Note::new(title.to_string(), None);
            note.content = content.to_string();
            nb.add_note(note);
        }
        nb
    }

    #[test]
    fn a_plain_checkbox_is_a_task() {
        assert_eq!(parse_task_line("- [ ] buy milk"), Some((false, "buy milk".into())));
        assert_eq!(parse_task_line("- [x] buy milk"), Some((true, "buy milk".into())));
    }

    #[test]
    fn indented_and_starred_bullets_count_too() {
        assert_eq!(parse_task_line("    - [ ] nested"), Some((false, "nested".into())));
        assert_eq!(parse_task_line("* [ ] starred"), Some((false, "starred".into())));
        assert_eq!(parse_task_line("+ [ ] plus"), Some((false, "plus".into())));
    }

    /// Obsidian and friends write `[/]` and `[-]` for in-progress and cancelled.
    /// None of those mean "still to do", so none of them should show up as open.
    #[test]
    fn any_non_space_mark_counts_as_done() {
        assert_eq!(parse_task_line("- [X] shouty"), Some((true, "shouty".into())));
        assert_eq!(parse_task_line("- [/] partial"), Some((true, "partial".into())));
        assert_eq!(parse_task_line("- [-] cancelled"), Some((true, "cancelled".into())));
    }

    /// Ordinary prose must not become a task just for containing brackets.
    #[test]
    fn prose_with_brackets_is_not_a_task() {
        assert_eq!(parse_task_line("- [a] see appendix"), None);
        assert_eq!(parse_task_line("- not a task"), None);
        assert_eq!(parse_task_line("just text"), None);
        assert_eq!(parse_task_line("- [ref] a citation"), None);
    }

    #[test]
    fn an_empty_task_is_still_a_task() {
        assert_eq!(parse_task_line("- [ ]"), Some((false, String::new())));
    }

    /// A checklist inside a code fence is documentation about tasks, not a task.
    /// The landing page's inline counter got this wrong.
    #[test]
    fn checkboxes_inside_code_fences_are_ignored() {
        let nb = notebook_with(&[(
            "Doc",
            "- [ ] real\n```\n- [ ] example in a code block\n```\n- [ ] also real\n",
        )]);
        let tasks = collect(&nb, false);
        assert_eq!(tasks.len(), 2, "code fence tasks leaked in: {:?}", tasks);
        assert_eq!(tasks[0].text, "real");
        assert_eq!(tasks[1].text, "also real");
    }

    #[test]
    fn done_tasks_are_excluded_unless_asked_for() {
        let nb = notebook_with(&[("Doc", "- [ ] open\n- [x] closed\n")]);
        assert_eq!(collect(&nb, false).len(), 1);
        assert_eq!(collect(&nb, true).len(), 2);
    }

    #[test]
    fn tasks_carry_the_line_they_came_from() {
        let nb = notebook_with(&[("Doc", "intro\n\n- [ ] first\ntext\n- [ ] second\n")]);
        let tasks = collect(&nb, false);
        assert_eq!(tasks[0].line, 2);
        assert_eq!(tasks[1].line, 4);
    }

    /// `notes` is a HashMap, so without an explicit sort the panel's rows would
    /// reshuffle every time it opened.
    #[test]
    fn the_order_is_stable_across_calls() {
        let nb = notebook_with(&[
            ("Zebra", "- [ ] z1\n- [ ] z2\n"),
            ("Apple", "- [ ] a1\n"),
            ("Mango", "- [ ] m1\n"),
        ]);
        let first = collect(&nb, false);
        for _ in 0..20 {
            assert_eq!(collect(&nb, false), first, "task order changed between calls");
        }
        let titles: Vec<&str> = first.iter().map(|t| t.note_title.as_str()).collect();
        assert_eq!(titles, vec!["Apple", "Mango", "Zebra", "Zebra"]);
    }

    #[test]
    fn notes_covered_counts_distinct_notes() {
        let nb = notebook_with(&[("A", "- [ ] one\n- [ ] two\n"), ("B", "- [ ] three\n")]);
        let tasks = collect(&nb, false);
        assert_eq!(tasks.len(), 3);
        assert_eq!(notes_covered(&tasks), 2);
    }
}
