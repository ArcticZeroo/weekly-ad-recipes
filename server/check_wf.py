import subprocess
import json
import re
import sys

proc = subprocess.Popen(
    [r'C:\Program Files\Google\Chrome\Application\chrome.exe',
     '--headless=new', '--disable-gpu', '--no-sandbox',
     '--dump-dom', '--virtual-time-budget=15000',
     'https://www.wholefoodsmarket.com/sales-flyer'],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    encoding='utf-8', errors='replace'
)
stdout, stderr = proc.communicate(timeout=45)
print(f'Got {len(stdout)} chars of DOM')

# Check for chobani
if 'chobani' in stdout.lower():
    idx = stdout.lower().index('chobani')
    print(f'\nFound "chobani" at index {idx}')
    start = max(0, idx - 300)
    end = min(len(stdout), idx + 300)
    print('Context around chobani:')
    print(stdout[start:end])
    print('\n---')
else:
    print('No "chobani" found in rendered DOM')

# Look for large JSON-like structures
# Check for script tags with type=application/json
scripts = re.findall(r'<script[^>]*>([\s\S]*?)</script>', stdout)
print(f'\nFound {len(scripts)} script tags total')
for i, s in enumerate(scripts):
    if len(s) > 1000:
        print(f'\nLarge script {i}: {len(s)} chars')
        # Try to parse as JSON
        try:
            data = json.loads(s)
            if isinstance(data, dict):
                print(f'  Keys: {list(data.keys())[:15]}')
            elif isinstance(data, list):
                print(f'  Array of {len(data)} items')
        except:
            print(f'  Not JSON. First 200 chars: {s[:200]}')

# Also search for any JSON arrays with item/product/deal-like keys
for pattern in [r'"product_name"', r'"item_name"', r'"sale_price"', r'"regular_price"', r'"savings"']:
    if pattern.strip('"') in stdout.lower():
        idx = stdout.lower().index(pattern.strip('"'))
        print(f'\nFound {pattern} at index {idx}')
        start = max(0, idx - 200)
        end = min(len(stdout), idx + 200)
        print(stdout[start:end])
        break
