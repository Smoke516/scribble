#!/bin/bash

# Script to create a demo recording of Scribble

echo "🎬 Creating Scribble Demo Recording..."
echo ""
echo "This will record a terminal session demonstrating Scribble's features."
echo "The recording will be saved as 'demo.cast' which can be:"
echo "• Played back with: asciinema play demo.cast"
echo "• Uploaded to asciinema.org for web playback"
echo "• Converted to GIF for GitHub README"
echo ""
echo "During the recording, please:"
echo "1. Navigate through the demo files"
echo "2. Show syntax highlighting (hello.rs, example.py, notes.md)"
echo "3. Use the search functionality (Ctrl+F)"
echo "4. Navigate between files using arrow keys"
echo "5. Exit with 'q' when done"
echo ""
read -p "Ready to start recording? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Starting recording in 3 seconds..."
    sleep 3
    
    # Record the demo
    asciinema rec demo.cast --title "Scribble - Terminal Text Editor Demo" \
        --command "./target/release/scribble demo_files/" \
        --idle-time-limit 3
    
    echo ""
    echo "✅ Demo recorded successfully!"
    echo ""
    echo "To play back the demo:"
    echo "  asciinema play demo.cast"
    echo ""
    echo "To upload to asciinema.org (optional):"
    echo "  asciinema upload demo.cast"
    echo ""
    echo "The demo.cast file can be committed to your repository."
else
    echo "Recording cancelled."
fi
