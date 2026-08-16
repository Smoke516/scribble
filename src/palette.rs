//! One door in front of the six finders.
//!
//! There were six ways to find a note — quick jump, search, recent, tag browser,
//! explorer, outline — which is not six features, it is one feature with six
//! doors. This is the single door: you press one key and start typing, and it
//! works out whether you meant a note, a tag, a heading or an action.
//!
//! The existing finders stay exactly as they were. They are the implementation
//! and the shortcuts for the ones already in your fingers; this sits in front.
//!
//! Resolution is a pure function of `(query, context)`, so ranking and the mode
//! prefixes can be tested without standing up an editor.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use uuid::Uuid;

use crate::models::NotebookData;
use crate::tags::TagManager;

/// What the palette will do if you press Enter on a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    OpenNote(Uuid),
    /// Open a note and put the cursor on a line — a heading or a task.
    JumpTo(Uuid, usize),
    /// Narrow the palette to notes carrying a tag, rather than leaving it.
    FilterTag(String),
    Run(Command),
}

/// Actions reachable by name rather than by chord.
///
/// Deliberately only things worth *finding*: a command you would have to remember
/// a chord for. Motions and editing keys are not here — nobody opens a palette to
/// press `w`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    DailyNote,
    Outline,
    Explorer,
    RecentFiles,
    TagBrowser,
    ThemeBrowser,
    VaultSwitcher,
    TogglePreview,
    NewNote,
    SaveNote,
    Help,
    Quit,
}

impl Command {
    /// Every command, in the order they appear when the palette is opened with `>`
    /// and nothing typed.
    pub const ALL: [Command; 12] = [
        Command::DailyNote,
        Command::Outline,
        Command::Explorer,
        Command::RecentFiles,
        Command::TagBrowser,
        Command::ThemeBrowser,
        Command::VaultSwitcher,
        Command::TogglePreview,
        Command::NewNote,
        Command::SaveNote,
        Command::Help,
        Command::Quit,
    ];

    /// What the row says. Phrased as an instruction so typing a verb finds it.
    pub fn label(self) -> &'static str {
        match self {
            Command::DailyNote => "Open today's daily note",
            Command::Outline => "Jump to a heading in this note",
            Command::Explorer => "Browse the vault",
            Command::RecentFiles => "Open a recent note",
            Command::TagBrowser => "Browse tags",
            Command::ThemeBrowser => "Change theme",
            Command::VaultSwitcher => "Switch vault",
            Command::TogglePreview => "Toggle live preview",
            Command::NewNote => "New note",
            Command::SaveNote => "Save this note",
            Command::Help => "Help",
            Command::Quit => "Quit scribble",
        }
    }

    /// The chord this command already has, shown on the row so the palette teaches
    /// the shortcut rather than replacing it.
    pub fn chord(self) -> &'static str {
        match self {
            Command::DailyNote => "F4",
            Command::Outline => "Ctrl+G",
            Command::Explorer => "Ctrl+E",
            Command::RecentFiles => "Ctrl+O",
            Command::TagBrowser => "Ctrl+T",
            Command::ThemeBrowser => "F3",
            Command::VaultSwitcher => "Ctrl+V",
            Command::TogglePreview => "Ctrl+P",
            Command::NewNote => "n",
            Command::SaveNote => "Ctrl+S",
            Command::Help => "?",
            Command::Quit => "q",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Note,
    Tag,
    Heading,
    Command,
}

impl Kind {
    /// A one-word badge, so a mixed list still reads as sorted by relevance rather
    /// than looking like four lists stapled together.
    pub fn badge(self) -> &'static str {
        match self {
            Kind::Note => "note",
            Kind::Tag => "tag",
            Kind::Heading => "head",
            Kind::Command => "cmd",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: Kind,
    pub label: String,
    /// Secondary text: the folder for a note, the note for a heading, the chord
    /// for a command.
    pub detail: String,
    pub action: PaletteAction,
    pub score: i64,
}

/// What the palette needs to know about the world, gathered by the caller so that
/// resolution stays a pure function.
pub struct Context<'a> {
    pub notebook: &'a NotebookData,
    /// The same extractor the tag browser uses. Tags live in two places — the
    /// frontmatter `tags:` field and inline `#hashtags` — and reading only the
    /// former showed an empty tag list against a vault whose tags are all inline.
    pub tags: &'a TagManager,
    /// Most-recent-first, used when nothing has been typed yet.
    pub recent: &'a [Uuid],
    pub current_note: Option<Uuid>,
    pub current_content: &'a str,
}

/// Which pool a query is searching, decided by its first character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Notes by title, plus commands. The default.
    Mixed,
    /// `>` — commands only.
    Commands,
    /// `#` — tags.
    Tags,
    /// `?` — full text of every note, not just titles.
    Content,
    /// `@` — headings in the note that is open.
    Headings,
}

impl Mode {
    /// Split a raw query into its mode and the text after the prefix.
    pub fn split(query: &str) -> (Mode, &str) {
        match query.as_bytes().first() {
            Some(b'>') => (Mode::Commands, &query[1..]),
            Some(b'#') => (Mode::Tags, &query[1..]),
            Some(b'?') => (Mode::Content, &query[1..]),
            Some(b'@') => (Mode::Headings, &query[1..]),
            _ => (Mode::Mixed, query),
        }
    }

    /// The hint shown under the input, so the prefixes are discoverable without
    /// having to already know them.
    pub fn hint(self) -> &'static str {
        match self {
            Mode::Mixed => "> commands   # tags   ? full text   @ headings",
            Mode::Commands => "Commands",
            Mode::Tags => "Tags",
            Mode::Content => "Searching note text",
            Mode::Headings => "Headings in this note",
        }
    }
}

/// How many rows any one pool may contribute, so a vault with hundreds of notes
/// cannot crowd commands out of a mixed query.
const PER_POOL: usize = 40;
/// How much longer a heading extract may run before it is trimmed.
const SNIPPET: usize = 60;

/// Rank everything matching `query` into a single list, best first.
pub fn resolve(query: &str, ctx: &Context) -> Vec<Item> {
    let (mode, raw) = Mode::split(query);
    // Tags keep their raw text: a trailing space is what turns `#tag` into
    // "within this tag", so trimming it here would silently undo the narrowing.
    let raw = raw.trim_start();
    let text = raw.trim_end();
    let matcher = SkimMatcherV2::default();

    let mut items: Vec<Item> = match mode {
        Mode::Commands => commands(&matcher, text),
        Mode::Tags => tags(&matcher, raw, ctx),
        Mode::Content => content(&matcher, text, ctx),
        Mode::Headings => headings(&matcher, text, ctx),
        Mode::Mixed => {
            if text.is_empty() {
                // Nothing typed: the most useful thing to offer is what you were
                // just working on, which is what Ctrl+O existed for.
                return recent(ctx);
            }
            let mut mixed = notes(&matcher, text, ctx);
            mixed.extend(commands(&matcher, text));
            mixed
        }
    };

    items.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    items.truncate(PER_POOL * 2);
    items
}

/// What to show before anything is typed.
///
/// Recently-opened notes, which is what the recent-files finder was for. A fresh
/// session has no recency yet, so it falls back to listing the vault — an empty
/// palette on first open would be a dead end rather than a starting point.
fn recent(ctx: &Context) -> Vec<Item> {
    let mut ordered: Vec<&crate::models::Note> = ctx
        .recent
        .iter()
        .filter_map(|id| ctx.notebook.notes.get(id))
        .collect();

    if ordered.is_empty() {
        ordered = ctx.notebook.notes.values().collect();
        ordered.sort_by_key(|n| n.title.to_lowercase());
    }

    ordered
        .into_iter()
        .take(PER_POOL)
        .enumerate()
        .map(|(i, note)| Item {
            kind: Kind::Note,
            label: note.title.clone(),
            detail: folder_of(ctx.notebook, note.folder_id),
            action: PaletteAction::OpenNote(note.id),
            // Descending, so the existing recency order survives the sort.
            score: (PER_POOL - i) as i64,
        })
        .collect()
}

fn notes(matcher: &SkimMatcherV2, text: &str, ctx: &Context) -> Vec<Item> {
    let mut out: Vec<Item> = ctx
        .notebook
        .notes
        .values()
        .filter_map(|note| {
            let score = matcher.fuzzy_match(&note.title, text)?;
            Some(Item {
                kind: Kind::Note,
                label: note.title.clone(),
                detail: folder_of(ctx.notebook, note.folder_id),
                action: PaletteAction::OpenNote(note.id),
                // Titles are what people mean by default, so they outrank a command
                // that merely happens to contain the same letters.
                score: score * 2,
            })
        })
        .collect();
    out.sort_by(|a, b| b.score.cmp(&a.score));
    out.truncate(PER_POOL);
    out
}

fn commands(matcher: &SkimMatcherV2, text: &str) -> Vec<Item> {
    Command::ALL
        .iter()
        .filter_map(|&cmd| {
            let score = if text.is_empty() {
                0
            } else {
                matcher.fuzzy_match(cmd.label(), text)?
            };
            Some(Item {
                kind: Kind::Command,
                label: cmd.label().to_string(),
                detail: cmd.chord().to_string(),
                action: PaletteAction::Run(cmd),
                score,
            })
        })
        .collect()
}

fn tags(matcher: &SkimMatcherV2, text: &str, ctx: &Context) -> Vec<Item> {
    // `#tag ` — a tag followed by a space — stops being a search for tags and
    // becomes a search *within* that tag. This is what picking a tag off the list
    // does, so narrowing stays inside the same pure resolution rather than the UI
    // holding a second kind of state the query cannot describe.
    if let Some((tag, rest)) = text.split_once(' ') {
        return notes_with_tag(matcher, tag, rest.trim(), ctx);
    }

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for note in ctx.notebook.notes.values() {
        for tag in ctx.tags.extract_tags_from_note(note) {
            *counts.entry(tag).or_insert(0) += 1;
        }
    }

    let mut out: Vec<Item> = counts
        .into_iter()
        .filter_map(|(tag, count)| {
            let score = if text.is_empty() {
                count as i64
            } else {
                matcher.fuzzy_match(&tag, text)?
            };
            Some(Item {
                kind: Kind::Tag,
                label: format!("#{}", tag),
                detail: format!("{} note{}", count, if count == 1 { "" } else { "s" }),
                action: PaletteAction::FilterTag(tag.clone()),
                score,
            })
        })
        .collect();
    out.sort_by(|a, b| b.score.cmp(&a.score));
    out.truncate(PER_POOL);
    out
}

/// Notes carrying `tag`, optionally narrowed further by title.
fn notes_with_tag(matcher: &SkimMatcherV2, tag: &str, rest: &str, ctx: &Context) -> Vec<Item> {
    let mut out: Vec<Item> = ctx
        .notebook
        .notes
        .values()
        .filter(|n| {
            ctx.tags
                .extract_tags_from_note(n)
                .iter()
                .any(|t| t.eq_ignore_ascii_case(tag))
        })
        .filter_map(|note| {
            let score = if rest.is_empty() {
                0
            } else {
                matcher.fuzzy_match(&note.title, rest)?
            };
            Some(Item {
                kind: Kind::Note,
                label: note.title.clone(),
                detail: folder_of(ctx.notebook, note.folder_id),
                action: PaletteAction::OpenNote(note.id),
                score,
            })
        })
        .collect();
    out.sort_by(|a, b| b.score.cmp(&a.score).then(a.label.cmp(&b.label)));
    out.truncate(PER_POOL);
    out
}

fn content(matcher: &SkimMatcherV2, text: &str, ctx: &Context) -> Vec<Item> {
    if text.is_empty() {
        return Vec::new();
    }

    let needle = text.to_lowercase();
    let mut out = Vec::new();
    for note in ctx.notebook.notes.values() {
        // A literal line match is what someone typing into a search box means, and
        // it gives an honest snippet to show. Fuzzy matching whole note bodies
        // matches almost everything and can point at no particular line.
        let hit = note
            .content
            .lines()
            .enumerate()
            .find(|(_, line)| line.to_lowercase().contains(&needle));

        let Some((line_no, line)) = hit else { continue };
        let score = matcher
            .fuzzy_match(&note.title, text)
            .unwrap_or(0)
            .max(10);
        out.push(Item {
            kind: Kind::Note,
            label: trim_to(line.trim(), SNIPPET),
            detail: note.title.clone(),
            action: PaletteAction::JumpTo(note.id, line_no),
            score,
        });
    }
    out.sort_by(|a, b| b.score.cmp(&a.score));
    out.truncate(PER_POOL);
    out
}

fn headings(matcher: &SkimMatcherV2, text: &str, ctx: &Context) -> Vec<Item> {
    let Some(note_id) = ctx.current_note else {
        return Vec::new();
    };


    let mut out = Vec::new();
    let mut in_code = false;
    for (line_no, line) in ctx.current_content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let level = trimmed.bytes().take_while(|&b| b == b'#').count();
        if !(1..=6).contains(&level) || !trimmed[level..].starts_with(' ') {
            continue;
        }
        let title = trimmed[level..].trim();

        let score = if text.is_empty() {
            // Keep document order when nothing is typed, and favour top-level
            // headings so the shape of the note comes through.
            (1000 - line_no as i64) * 10 - level as i64
        } else {
            match matcher.fuzzy_match(title, text) {
                Some(s) => s,
                None => continue,
            }
        };

        out.push(Item {
            kind: Kind::Heading,
            label: format!("{} {}", "#".repeat(level), title),
            detail: String::new(),
            action: PaletteAction::JumpTo(note_id, line_no),
            score,
        });
    }
    out.truncate(PER_POOL);
    out
}

fn folder_of(notebook: &NotebookData, folder_id: Option<Uuid>) -> String {
    folder_id
        .and_then(|id| notebook.folders.get(&id))
        .map(|f| f.name.clone())
        .unwrap_or_default()
}

fn trim_to(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Folder, Note};

    struct World {
        notebook: NotebookData,
        recent: Vec<Uuid>,
        tags: TagManager,
    }

    fn world() -> World {
        let mut notebook = NotebookData::new();
        let folder = Folder::new("Projects".to_string(), None);
        let folder_id = folder.id;
        notebook.add_folder(folder);

        let mut alpha = Note::new("Meeting Notes".to_string(), Some(folder_id));
        alpha.content = "# Agenda\n\nDiscuss the budget forecast\n\n## Actions\n- follow up\n"
            .to_string();
        alpha.tags = vec!["work".to_string(), "meetings".to_string()];

        let mut beta = Note::new("Grocery List".to_string(), None);
        beta.content = "milk\neggs\n".to_string();
        beta.tags = vec!["home".to_string()];

        let mut gamma = Note::new("Rust Notes".to_string(), None);
        gamma.content = "ownership and borrowing\n".to_string();
        gamma.tags = vec!["work".to_string()];

        let recent = vec![beta.id, gamma.id, alpha.id];
        for n in [alpha, beta, gamma] {
            notebook.add_note(n);
        }
        World { notebook, recent, tags: TagManager::new() }
    }

    fn ctx(w: &World) -> Context<'_> {
        Context {
            notebook: &w.notebook,
            tags: &w.tags,
            recent: &w.recent,
            current_note: None,
            current_content: "",
        }
    }

    fn labels(items: &[Item]) -> Vec<&str> {
        items.iter().map(|i| i.label.as_str()).collect()
    }

    #[test]
    fn prefixes_choose_the_pool() {
        assert_eq!(Mode::split(">theme"), (Mode::Commands, "theme"));
        assert_eq!(Mode::split("#work"), (Mode::Tags, "work"));
        assert_eq!(Mode::split("?budget"), (Mode::Content, "budget"));
        assert_eq!(Mode::split("@agenda"), (Mode::Headings, "agenda"));
        assert_eq!(Mode::split("meeting"), (Mode::Mixed, "meeting"));
    }

    /// Nothing typed should offer what you were just doing — that is what the
    /// recent-files finder was for, and an empty palette is a dead end.
    #[test]
    fn an_empty_query_offers_recent_notes_in_order() {
        let w = world();
        let items = resolve("", &ctx(&w));
        assert_eq!(labels(&items), vec!["Grocery List", "Rust Notes", "Meeting Notes"]);
    }

    #[test]
    fn typing_finds_notes_by_title() {
        let w = world();
        let items = resolve("grocery", &ctx(&w));
        assert_eq!(items[0].label, "Grocery List");
        assert_eq!(items[0].kind, Kind::Note);
    }

    #[test]
    fn a_note_carries_its_folder_as_detail() {
        let w = world();
        let items = resolve("meeting", &ctx(&w));
        let note = items.iter().find(|i| i.label == "Meeting Notes").unwrap();
        assert_eq!(note.detail, "Projects");
    }

    /// A mixed query returns both pools, and a title outranks a command that
    /// merely shares letters — searching "note" should not lead with "New note".
    #[test]
    fn titles_outrank_commands_in_a_mixed_query() {
        let w = world();
        // "note" matches the "… Notes" titles and the "New note" command alike.
        let items = resolve("note", &ctx(&w));
        assert_eq!(items[0].kind, Kind::Note, "a command outranked a note title");
        assert!(
            items.iter().any(|i| i.kind == Kind::Command),
            "commands vanished from a mixed query"
        );
    }

    #[test]
    fn commands_are_searchable_by_what_they_do() {
        let w = world();
        let items = resolve(">theme", &ctx(&w));
        assert_eq!(items[0].action, PaletteAction::Run(Command::ThemeBrowser));
        assert!(items.iter().all(|i| i.kind == Kind::Command));
    }

    /// The palette should teach the chord rather than replace it.
    #[test]
    fn a_command_row_shows_the_shortcut_it_already_has() {
        let w = world();
        let items = resolve(">daily", &ctx(&w));
        assert_eq!(items[0].detail, "F4");
    }

    #[test]
    fn bare_prefix_lists_the_whole_pool() {
        let w = world();
        assert_eq!(resolve(">", &ctx(&w)).len(), Command::ALL.len());
    }

    #[test]
    fn tags_are_listed_with_how_many_notes_carry_them() {
        let w = world();
        let items = resolve("#", &ctx(&w));
        let work = items.iter().find(|i| i.label == "#work").unwrap();
        assert_eq!(work.detail, "2 notes");
        assert_eq!(work.action, PaletteAction::FilterTag("work".to_string()));
        let home = items.iter().find(|i| i.label == "#home").unwrap();
        assert_eq!(home.detail, "1 note", "singular should not be pluralised");
    }

    /// Tags live in two places, and the vault this was built for uses only the
    /// second: a frontmatter `tags:` field, and inline `#hashtags` in the body.
    /// Reading only `note.tags` showed an empty tag list against a real vault.
    #[test]
    fn inline_hashtags_count_as_tags() {
        let mut w = world();
        let id = w.notebook.find_note_by_title("Grocery List").unwrap();
        w.notebook.notes.get_mut(&id).unwrap().content = "milk #errands\n".to_string();

        let items = resolve("#errands", &ctx(&w));
        assert_eq!(items[0].label, "#errands", "an inline hashtag was not seen as a tag");

        let narrowed = resolve("#errands ", &ctx(&w));
        assert_eq!(labels(&narrowed), vec!["Grocery List"]);
    }

    #[test]
    fn tags_can_be_narrowed_by_typing() {
        let w = world();
        let items = resolve("#wo", &ctx(&w));
        assert_eq!(labels(&items), vec!["#work"]);
    }

    /// Full-text search points at the line it found, not just the note, and shows
    /// that line so you can tell the hits apart.
    #[test]
    fn content_search_finds_a_line_and_points_at_it() {
        let w = world();
        let items = resolve("?budget", &ctx(&w));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "Discuss the budget forecast");
        assert_eq!(items[0].detail, "Meeting Notes");
        let note_id = w.notebook.find_note_by_title("Meeting Notes").unwrap();
        assert_eq!(items[0].action, PaletteAction::JumpTo(note_id, 2));
    }

    #[test]
    fn an_empty_content_search_returns_nothing_rather_than_everything() {
        let w = world();
        assert!(resolve("?", &ctx(&w)).is_empty());
    }

    #[test]
    fn headings_come_from_the_open_note_in_document_order() {
        let w = world();
        let note_id = w.notebook.find_note_by_title("Meeting Notes").unwrap();
        let content = w.notebook.notes.get(&note_id).unwrap().content.clone();
        let c = Context {
            notebook: &w.notebook,
            tags: &w.tags,
            recent: &w.recent,
            current_note: Some(note_id),
            current_content: &content,
        };
        let items = resolve("@", &c);
        assert_eq!(labels(&items), vec!["# Agenda", "## Actions"]);
        assert_eq!(items[0].action, PaletteAction::JumpTo(note_id, 0));
    }

    /// A `#` inside a fence is not a heading, the same rule the outline uses.
    #[test]
    fn headings_skip_fenced_code() {
        let w = world();
        let note_id = w.notebook.find_note_by_title("Meeting Notes").unwrap();
        let content = "# Real\n```\n# Not a heading\n```\n## Also real\n".to_string();
        let c = Context {
            notebook: &w.notebook,
            tags: &w.tags,
            recent: &w.recent,
            current_note: Some(note_id),
            current_content: &content,
        };
        assert_eq!(labels(&resolve("@", &c)), vec!["# Real", "## Also real"]);
    }

    #[test]
    fn headings_are_empty_with_no_note_open() {
        let w = world();
        assert!(resolve("@anything", &ctx(&w)).is_empty());
    }

    /// `#tag ` narrows to that tag's notes. The trailing space carries the meaning,
    /// so the resolver must not trim it away — it did, and picking a tag silently
    /// re-listed the tags instead of the notes.
    #[test]
    fn a_trailing_space_narrows_a_tag_to_its_notes() {
        let w = world();
        let items = resolve("#work ", &ctx(&w));
        assert_eq!(labels(&items), vec!["Meeting Notes", "Rust Notes"]);
        assert!(items.iter().all(|i| i.kind == Kind::Note));
    }

    #[test]
    fn a_narrowed_tag_can_be_filtered_further_by_title() {
        let w = world();
        let items = resolve("#work rust", &ctx(&w));
        assert_eq!(labels(&items), vec!["Rust Notes"]);
    }

    /// A fresh session has no recency yet. Showing nothing would make the palette a
    /// dead end on first open, so it falls back to the vault.
    #[test]
    fn an_empty_query_falls_back_to_the_vault_when_there_is_no_recency() {
        let w = world();
        let c = Context {
            notebook: &w.notebook,
            tags: &w.tags,
            recent: &[],
            current_note: None,
            current_content: "",
        };
        let items = resolve("", &c);
        assert_eq!(labels(&items), vec!["Grocery List", "Meeting Notes", "Rust Notes"]);
    }

    #[test]
    fn a_query_matching_nothing_returns_nothing() {
        let w = world();
        assert!(resolve("zzzzqqqq", &ctx(&w)).is_empty());
    }

    /// Long lines must not blow out the row width.
    #[test]
    fn content_snippets_are_trimmed() {
        let mut w = world();
        let id = w.notebook.find_note_by_title("Grocery List").unwrap();
        w.notebook.notes.get_mut(&id).unwrap().content =
            format!("{} needle", "x".repeat(300));
        let items = resolve("?needle", &ctx(&w));
        assert!(
            items[0].label.chars().count() <= SNIPPET + 1,
            "snippet not trimmed: {} chars",
            items[0].label.chars().count()
        );
        assert!(items[0].label.ends_with('…'));
    }
}
