import subprocess
import json
import re

proc = subprocess.Popen(
    [r'C:\Program Files\Google\Chrome\Application\chrome.exe',
     '--headless=new', '--disable-gpu', '--no-sandbox',
     '--dump-dom', '--virtual-time-budget=15000',
     'https://www.wholefoodsmarket.com/sales-flyer'],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    encoding='utf-8', errors='replace'
)
stdout, stderr = proc.communicate(timeout=45)

# Extract __NEXT_DATA__
scripts = re.findall(r'<script[^>]*>([\s\S]*?)</script>', stdout)
for s in scripts:
    if len(s) > 5000:
        try:
            data = json.loads(s)
            if isinstance(data, dict) and 'props' in data:
                props = data.get('props', {}).get('pageProps', {})
                print(f'pageProps keys: {list(props.keys())}')
                for key in props:
                    val = props[key]
                    if isinstance(val, list):
                        print(f'\n  {key}: list of {len(val)}')
                        if len(val) > 0 and isinstance(val[0], dict):
                            print(f'    first item keys: {list(val[0].keys())}')
                            print(f'    first item: {json.dumps(val[0], indent=2)[:800]}')
                    elif isinstance(val, dict):
                        print(f'\n  {key}: dict keys: {list(val.keys())[:20]}')
                        # Go one level deeper
                        for k2, v2 in val.items():
                            if isinstance(v2, list) and len(v2) > 0:
                                print(f'    {k2}: list of {len(v2)}')
                                if isinstance(v2[0], dict):
                                    print(f'      keys: {list(v2[0].keys())}')
                                    print(f'      first: {json.dumps(v2[0], indent=2)[:500]}')
                            elif isinstance(v2, dict):
                                print(f'    {k2}: dict keys: {list(v2.keys())[:10]}')
                            else:
                                print(f'    {k2}: {type(v2).__name__} = {str(v2)[:100]}')
                    else:
                        print(f'\n  {key}: {type(val).__name__} = {str(val)[:200]}')
        except json.JSONDecodeError:
            pass
