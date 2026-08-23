# Markdown snippets

Scribble offers a snippet when the text just before the cursor is one that has
something to add. Accepting it leaves the note different from what you typed —
that is the whole rule, and it is why there is no suggestion for `- ` or `# `.

## Using it

- **Accept**: `Tab`
- **Dismiss**: `Esc` (a second `Esc` leaves insert mode, as usual)
- **Navigate**: `↑` / `↓`
- `Enter` always breaks the line. It never accepts a snippet.

The popup goes away as soon as you type something the trigger no longer matches.

## The snippets

| Type | You get | Cursor lands |
|---|---|---|
| `[` | `[](url)` | between the brackets |
| `![` | `![](image.png)` | in the alt text |
| `**` | `****` | between them |
| `*` | `**` | between them |
| `` ` `` | ` `` ` | between them |
| ```` ``` ```` | a fenced block | on the empty line inside |
| `\|` | a three-column table skeleton | in the first header |

The fence and the table only fire with nothing but whitespace before them on the
line — one that began halfway through a sentence would not be one. The rest fire
anywhere, so `see the [` opens a link mid-sentence.

## Tab

Inside a list item, `Tab` indents it by one level and `Shift+Tab` outdents it.
Two spaces, matching the nesting the preview renders. Everywhere else `Tab`
inserts four spaces.

## History

Until 3.2 this table held seventeen entries, and it did not work:

- **Seven could never fire.** The scan demanded the text before the cursor end in
  a space, and `[`, `![`, `**`, `*`, `` ` ``, ```` ``` ```` and `---` do not.
- **Ten replaced the trigger with itself.** Typing `- ` offered "Bullet list
  item", and accepting it turned `- ` into `- `.
- Because a popup was up over every list item, `Tab` could not indent one and the
  first `Esc` did not leave insert mode.
- Every cursor offset in the unreachable half was wrong, which nothing noticed
  because nothing could reach them.
