import subprocess, json, re

proc = subprocess.Popen(
    [r'C:\Program Files\Google\Chrome\Application\chrome.exe',
     '--headless=new', '--disable-gpu', '--no-sandbox',
     '--dump-dom', '--virtual-time-budget=15000',
     'https://www.wholefoodsmarket.com/stores/search?text=98052'],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    encoding='utf-8', errors='replace'
)
stdout, stderr = proc.communicate(timeout=45)
print(f'Got {len(stdout)} chars')

# Look for store links like /stores/redmond or store-id params
store_links = re.findall(r'href="(/stores/[a-z0-9-]+)"', stdout)
print(f'Store links: {sorted(set(store_links))}')

# Look for sales-flyer links with store-id
flyer_links = re.findall(r'sales-flyer\?store-id=(\d+)', stdout)
print(f'Flyer store IDs: {sorted(set(flyer_links))}')

# Look for store cards with names and IDs
# Search for store name + id patterns
for term in ['Redmond', 'Bellevue', 'Kirkland', 'Lynnwood', 'Bothell']:
    if term.lower() in stdout.lower():
        idx = stdout.lower().index(term.lower())
        context = stdout[max(0, idx-200):idx+300]
        # Find any numbers near it that could be store IDs
        nearby_ids = re.findall(r'(\d{4,6})', context)
        print(f'\n{term} found, nearby IDs: {nearby_ids}')
        print(f'  context: ...{context[:300]}...')
