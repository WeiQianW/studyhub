# Writes manifest.json: version + sha256 of every file the app needs (used by the desktop updater)
import hashlib, json, os, sys
root = sys.argv[1]
ver = json.load(open(os.path.join(root, 'content.json'), encoding='utf-8')).get('version', '0')
files = []
for dp, _, fs in os.walk(root):
    for f in sorted(fs):
        p = os.path.join(dp, f); rel = os.path.relpath(p, root).replace(os.sep, '/')
        if rel == 'manifest.json': continue
        files.append({'path': rel, 'sha256': hashlib.sha256(open(p, 'rb').read()).hexdigest(), 'size': os.path.getsize(p)})
import re
desk = re.search(r'^version\s*=\s*"([^"]+)"', open(os.path.join(os.path.dirname(__file__), '..', 'crates', 'desktop', 'Cargo.toml')).read(), re.M).group(1)
json.dump({'app': 'studyhub', 'version': ver, 'desktop': desk, 'files': sorted(files, key=lambda x: x['path'])}, open(os.path.join(root, 'manifest.json'), 'w'), indent=1)
print('manifest', ver, len(files), 'files')
