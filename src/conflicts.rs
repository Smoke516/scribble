//! Finding and resolving conflict files.
//!
//! Storage refuses to overwrite work it did not write, preserving whatever it
//! found as a sibling file. That keeps both versions safe, and leaves you with two
//! copies and no way to reconcile them from inside the app. This is the other half.
//!
//! Nothing here merges anything. Which version wins is the user's call, the same
//! way it is in the storage policy; the job is to show the difference clearly and
//! then carry out whichever answer they give.

use uuid::Uuid;

use crate::models::NotebookData;

/// Who wrote the conflict file. Only used to explain the pairing to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Producer {
    Scribble,
    Nextcloud,
    Syncthing,
}

impl Producer {
    pub fn label(self) -> &'static str {
        match self {
            Producer::Scribble => "scribble",
            Producer::Nextcloud => "Nextcloud",
            Producer::Syncthing => "Syncthing",
        }
    }
}

/// A conflict artefact and, where it can be found, the note it forked from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The note's name with the marker stripped — `Meeting` for
    /// `Meeting (scribble conflict 2026-08-16 131829).md`.
    pub base_title: String,
    /// The note the artefact forked from, if it is still in the vault.
    pub original: Option<Uuid>,
    pub artefact: Uuid,
    pub producer: Producer,
}

/// Strip a conflict marker from a file stem, returning the original name.
///
/// Pairing is by filename rather than by `scribble_id`, for two reasons: the id is
/// re-minted in memory when two files claim the same one, so it cannot be relied on
/// to point back; and Nextcloud and Syncthing artefacts have to pair up the same
/// way even though nothing scribble wrote is involved.
pub fn strip_marker(stem: &str) -> Option<(String, Producer)> {
    // `Meeting (scribble conflict 2026-08-16 131829)` — ours.
    if let Some(at) = stem.find(" (scribble conflict") {
        return Some((stem[..at].to_string(), Producer::Scribble));
    }
    // `Meeting (conflicted copy 2026-08-16 120000)` — Nextcloud. The space before
    // the bracket is optional in the wild, so both spellings are accepted.
    for pattern in [" (conflicted copy", "(conflicted copy"] {
        if let Some(at) = stem.find(pattern) {
            return Some((stem[..at].trim_end().to_string(), Producer::Nextcloud));
        }
    }
    // `Meeting.sync-conflict-20260816-120000-ABCDEFG` — Syncthing.
    if let Some(at) = stem.find(".sync-conflict-") {
        return Some((stem[..at].to_string(), Producer::Syncthing));
    }
    None
}


fn file_stem(note: &crate::models::Note) -> Option<String> {
    note.file_path
        .as_ref()?
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
}

/// Every unresolved conflict in the vault, oldest name first.
///
/// Sorted so the list is stable between openings — `notes` is a HashMap with no
/// order of its own.
pub fn detect(notebook: &NotebookData) -> Vec<Conflict> {
    let mut found: Vec<Conflict> = notebook
        .notes
        .values()
        .filter_map(|note| {
            let stem = file_stem(note)?;
            let (base_title, producer) = strip_marker(&stem)?;

            // The original is the note whose file is named exactly the base name.
            // Matching on the title instead would pair the wrong note whenever a
            // title and its filename have diverged.
            let original = notebook
                .notes
                .values()
                .find(|n| n.id != note.id && file_stem(n).as_deref() == Some(base_title.as_str()))
                .map(|n| n.id);

            Some(Conflict {
                base_title,
                original,
                artefact: note.id,
                producer,
            })
        })
        .collect();

    found.sort_by(|a, b| {
        a.base_title
            .to_lowercase()
            .cmp(&b.base_title.to_lowercase())
            .then(a.artefact.cmp(&b.artefact))
    });
    found
}

/// One row of a side-by-side comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// Present and identical in both.
    Same(String),
    /// Only in the note you have.
    Mine(String),
    /// Only in the version that was preserved.
    Theirs(String),
}

/// Compare two versions line by line.
///
/// A longest-common-subsequence diff, so unchanged text lines up and the edit
/// stands out — a naive line-by-line pairing reports everything after a single
/// inserted line as different, which is exactly the case you most need to read.
pub fn diff(mine: &str, theirs: &str) -> Vec<Row> {
    let a: Vec<&str> = mine.lines().collect();
    let b: Vec<&str> = theirs.lines().collect();

    // lcs[i][j] = length of the longest common subsequence of a[i..] and b[j..].
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut rows = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            rows.push(Row::Same(a[i].to_string()));
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            rows.push(Row::Mine(a[i].to_string()));
            i += 1;
        } else {
            rows.push(Row::Theirs(b[j].to_string()));
            j += 1;
        }
    }
    rows.extend(a[i..].iter().map(|l| Row::Mine(l.to_string())));
    rows.extend(b[j..].iter().map(|l| Row::Theirs(l.to_string())));
    rows
}

/// How many lines differ, for a one-line summary.
pub fn changed_lines(rows: &[Row]) -> usize {
    rows.iter().filter(|r| !matches!(r, Row::Same(_))).count()
}

/// What to do with a conflict.
///
/// The shared `Keep` prefix is deliberate: these are the three answers to one
/// question, and `Resolution::Mine` reads as a fact about ownership rather than
/// as the instruction it is.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Keep the note as it stands; discard the preserved version.
    KeepMine,
    /// Take the preserved version's content into the note; discard the artefact.
    KeepTheirs,
    /// Keep both, with the artefact promoted to an ordinary note.
    KeepBoth,
}

impl Resolution {
    pub fn label(self) -> &'static str {
        match self {
            Resolution::KeepMine => "Keep mine — discard the preserved copy",
            Resolution::KeepTheirs => "Keep theirs — replace this note's text",
            Resolution::KeepBoth => "Keep both — the copy becomes its own note",
        }
    }
}

/// A free name for a promoted artefact: `Meeting (conflict copy)`, then `2`, `3`…
///
/// The marker has to go, or the promoted note is still a conflict as far as
/// everything else is concerned and would reappear in this list forever.
pub fn promoted_title(base: &str, taken: &[String]) -> String {
    let first = format!("{} (conflict copy)", base);
    if !taken.iter().any(|t| t.eq_ignore_ascii_case(&first)) {
        return first;
    }
    for n in 2.. {
        let candidate = format!("{} (conflict copy {})", base, n);
        if !taken.iter().any(|t| t.eq_ignore_ascii_case(&candidate)) {
            return candidate;
        }
    }
    unreachable!("an unbounded search cannot run out of names")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;

    fn notebook_with(files: &[(&str, &str)]) -> NotebookData {
        let mut nb = NotebookData::new();
        for (path, content) in files {
            let stem = std::path::Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let mut note = Note::new(stem, None);
            note.content = content.to_string();
            note.file_path = Some(std::path::PathBuf::from(*path));
            nb.add_note(note);
        }
        nb
    }

    #[test]
    fn markers_from_all_three_producers_are_stripped() {
        assert_eq!(
            strip_marker("Meeting (scribble conflict 2026-08-16 131829)"),
            Some(("Meeting".to_string(), Producer::Scribble))
        );
        assert_eq!(
            strip_marker("Meeting (conflicted copy 2026-08-16 120000)"),
            Some(("Meeting".to_string(), Producer::Nextcloud))
        );
        assert_eq!(
            strip_marker("Meeting.sync-conflict-20260816-120000-ABCDEFG"),
            Some(("Meeting".to_string(), Producer::Syncthing))
        );
    }

    #[test]
    fn an_ordinary_name_is_not_a_conflict() {
        assert_eq!(strip_marker("Meeting"), None);
        assert_eq!(strip_marker("Meeting notes (draft)"), None);
    }

    #[test]
    fn a_conflict_is_paired_with_the_note_it_forked_from() {
        let nb = notebook_with(&[
            ("/v/Meeting.md", "the original\n"),
            ("/v/Meeting (scribble conflict 2026-08-16 131829).md", "theirs\n"),
        ]);
        let found = detect(&nb);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].base_title, "Meeting");
        assert_eq!(found[0].producer, Producer::Scribble);
        assert!(found[0].original.is_some(), "the original was not found");
    }

    /// The original may have been deleted, or may never have arrived on this
    /// machine. The artefact still has to be listed, or it is invisible forever.
    #[test]
    fn a_conflict_with_no_original_is_still_listed() {
        let nb = notebook_with(&[(
            "/v/Orphan (conflicted copy 2026-08-16 120000).md",
            "only copy\n",
        )]);
        let found = detect(&nb);
        assert_eq!(found.len(), 1);
        assert!(found[0].original.is_none());
    }

    #[test]
    fn a_vault_with_no_conflicts_reports_none() {
        let nb = notebook_with(&[("/v/A.md", "x\n"), ("/v/B.md", "y\n")]);
        assert!(detect(&nb).is_empty());
    }

    /// `notes` is a HashMap, so without an explicit sort the list would reshuffle
    /// every time the panel opened.
    #[test]
    fn the_order_is_stable() {
        let nb = notebook_with(&[
            ("/v/Zebra (scribble conflict 1).md", "z\n"),
            ("/v/Apple (scribble conflict 1).md", "a\n"),
            ("/v/Mango (scribble conflict 1).md", "m\n"),
        ]);
        let first: Vec<String> = detect(&nb).iter().map(|c| c.base_title.clone()).collect();
        for _ in 0..20 {
            let again: Vec<String> = detect(&nb).iter().map(|c| c.base_title.clone()).collect();
            assert_eq!(first, again, "conflict order changed between calls");
        }
        assert_eq!(first, vec!["Apple", "Mango", "Zebra"]);
    }

    #[test]
    fn identical_versions_produce_no_differences() {
        let rows = diff("one\ntwo\n", "one\ntwo\n");
        assert_eq!(changed_lines(&rows), 0);
        assert!(rows.iter().all(|r| matches!(r, Row::Same(_))));
    }

    #[test]
    fn a_changed_line_shows_on_both_sides() {
        let rows = diff("one\nmine\nthree\n", "one\ntheirs\nthree\n");
        assert_eq!(
            rows,
            vec![
                Row::Same("one".into()),
                Row::Mine("mine".into()),
                Row::Theirs("theirs".into()),
                Row::Same("three".into()),
            ]
        );
    }

    /// The reason for a real diff rather than pairing line by line: one inserted
    /// line must not make everything after it look changed.
    #[test]
    fn an_inserted_line_does_not_desynchronise_the_rest() {
        let rows = diff("a\nb\nc\n", "a\nNEW\nb\nc\n");
        assert_eq!(changed_lines(&rows), 1, "{:?}", rows);
        assert_eq!(rows[1], Row::Theirs("NEW".into()));
    }

    #[test]
    fn an_appended_line_is_reported_once() {
        let rows = diff("a\nb\n", "a\nb\nextra\n");
        assert_eq!(changed_lines(&rows), 1);
        assert_eq!(rows.last(), Some(&Row::Theirs("extra".into())));
    }

    #[test]
    fn an_empty_version_is_all_one_sided() {
        let rows = diff("", "a\nb\n");
        assert_eq!(rows, vec![Row::Theirs("a".into()), Row::Theirs("b".into())]);
    }
}
