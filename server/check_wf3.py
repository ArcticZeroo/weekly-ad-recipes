import subprocess
import json
import re
import sys

# Use Chrome with network logging to see what API calls the page makes
proc = subprocess.Popen(
    [r'C:\Program Files\Google\Chrome\Application\chrome.exe',
     '--headless=new', '--disable-gpu', '--no-sandbox',
     '--dump-dom', '--virtual-time-budget=15000',
     # Use a store-specific URL if one exists
     'https://www.wholefoodsmarket.com/sales-flyer?storeId=10101'],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    encoding='utf-8', errors='replace'
)
stdout, stderr = proc.communicate(timeout=45)
print(f'Got {len(stdout)} chars')

# Check __NEXT_DATA__ again  
scripts = re.findall(r'<script[^>]*>([\s\S]*?)</script>', stdout)
for s in scripts:
    if len(s) > 5000:
        try:
            data = json.loads(s)
            if isinstance(data, dict) and 'props' in data:
                props = data.get('props', {}).get('pageProps', {})
                promotions = props.get('promotions', [])
                store_id = props.get('storeId')
                store_name = props.get('storeName')
                print(f'storeId: {store_id}')
                print(f'storeName: {store_name}')
                print(f'promotions count: {len(promotions)}')
                if promotions:
                    print(f'\nFirst promotion keys: {list(promotions[0].keys())}')
                    print(json.dumps(promotions[0], indent=2)[:1000])
                    if len(promotions) > 1:
                        print(f'\nSecond promotion:')
                        print(json.dumps(promotions[1], indent=2)[:1000])
                    print(f'\nTotal promotions: {len(promotions)}')
        except:
            pass

# Also look for deal data in the DOM itself
if 'chobani' in stdout.lower():
    # Find a JSON-like structure near chobani
    idx = stdout.lower().index('chobani')
    # Search backwards for a large data attribute or JSON structure
    chunk = stdout[max(0, idx-2000):idx+2000]
    # Look for data attributes
    data_attrs = re.findall(r'data-[a-z-]+="([^"]*chobani[^"]*)"', chunk, re.IGNORECASE)
    if data_attrs:
        print(f'\nData attrs with chobani: {data_attrs}')
    
    # Check for price/deal info near chobani
    for pattern in ['\\$', 'price', 'save', 'off', 'sale', 'buy']:
        nearby = chunk.lower()
        if pattern.lower().replace('\\', '') in nearby:
            matches = re.findall(f'.{{0,50}}{pattern}.{{0,50}}', nearby, re.IGNORECASE)
            if matches:
                print(f'\nNear chobani - "{pattern}": {matches[0]}')
