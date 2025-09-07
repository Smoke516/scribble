#!/usr/bin/env python3
"""
A simple example Python file to demonstrate syntax highlighting
in the Scribble text editor.
"""

import json
import sys
from typing import List, Dict, Any

def fibonacci(n: int) -> int:
    """Calculate the nth Fibonacci number."""
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

def process_data(data: List[Dict[str, Any]]) -> Dict[str, int]:
    """Process a list of dictionaries and return summary statistics."""
    result = {
        'count': len(data),
        'total_value': 0,
        'avg_value': 0
    }
    
    values = [item.get('value', 0) for item in data if 'value' in item]
    if values:
        result['total_value'] = sum(values)
        result['avg_value'] = result['total_value'] // len(values)
    
    return result

if __name__ == "__main__":
    # Sample data
    sample_data = [
        {"name": "Alice", "value": 100},
        {"name": "Bob", "value": 150},
        {"name": "Charlie", "value": 75},
    ]
    
    print("Fibonacci sequence:")
    for i in range(10):
        print(f"F({i}) = {fibonacci(i)}")
    
    print("\nData processing:")
    stats = process_data(sample_data)
    print(json.dumps(stats, indent=2))
