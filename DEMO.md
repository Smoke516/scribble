# Scribble Demo

This directory contains demo materials to showcase Scribble's features.

## Quick Demo

To see Scribble in action, you can:

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Run the demo:**
   ```bash
   ./target/release/scribble demo_files/
   ```

3. **Try these features:**
   - Navigate through files with arrow keys
   - Press `Enter` to open a file and see syntax highlighting
   - Use `Ctrl+F` to search within files
   - Press `q` to quit

## Recording a Demo

We've included scripts to create demo recordings:

### Create an asciinema recording:
```bash
./create_demo.sh
```

This will create a `demo.cast` file that can be:
- Played back locally: `asciinema play demo.cast`
- Uploaded to asciinema.org for web sharing
- Embedded in documentation

## Demo Files

The `demo_files/` directory contains sample files that showcase:

- **`hello.rs`** - Rust syntax highlighting with functions and tests
- **`example.py`** - Python syntax highlighting with type hints and imports  
- **`notes.md`** - Markdown formatting with lists, headers, and checkboxes

## Features Demonstrated

✨ **Syntax Highlighting** - Beautiful Tokyo Night theme with support for multiple languages

🔍 **Search Functionality** - Fast text search with highlighting

📁 **File Browser** - Clean interface for navigating directories and files

⚡ **Performance** - Smooth scrolling and responsive UI

🎨 **Terminal UI** - Modern terminal interface with clean design

## Recording Your Own Demo

1. Make sure asciinema is installed:
   ```bash
   sudo apt install asciinema  # Ubuntu/Debian
   brew install asciinema      # macOS
   ```

2. Use our demo script:
   ```bash
   ./create_demo.sh
   ```

3. Or record manually:
   ```bash
   asciinema rec my_demo.cast --title "My Scribble Demo" \
     --command "./target/release/scribble demo_files/"
   ```

## Sharing Your Demo

- **GitHub**: Commit the `.cast` file to your repository
- **asciinema.org**: Upload with `asciinema upload demo.cast`
- **Convert to GIF**: Use tools like `asciicast2gif` for animated GIFs

---

*For more information, visit the [main repository](https://github.com/Smoke516/scribble)*
