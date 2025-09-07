#!/bin/bash

# Scribble Demo Script
# This script demonstrates the key features of the Scribble text editor

set -e

echo "🎬 Starting Scribble Demo..."
sleep 2

# Clear the terminal
clear

echo "Welcome to Scribble - A Terminal Text Editor"
echo "=============================================="
echo ""
echo "This demo will showcase:"
echo "• File browsing and navigation"
echo "• Syntax highlighting for different languages"
echo "• Search functionality"
echo "• File editing capabilities"
echo ""
echo "Press any key to continue..."
read -n 1 -s
clear

echo "Let's start by opening Scribble..."
sleep 1

# Start Scribble (this will be interactive)
./target/release/scribble demo_files/

echo ""
echo "Demo completed! ✨"
echo ""
echo "Key features demonstrated:"
echo "• Beautiful Tokyo Night theme"
echo "• Syntax highlighting for Rust, Python, and Markdown"
echo "• File browser with directory navigation"
echo "• Search capabilities"
echo "• Real-time file preview"
echo ""
echo "Visit https://github.com/Smoke516/scribble for more information!"
