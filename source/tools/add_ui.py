# Copies the long Extra-tab paragraphs from the original page source into content.json ("ui" block),
# so they can be updated with the content instead of with a new program build.
import json, re, sys
src = open(sys.argv[1], encoding='utf-8').read()
fn = src[src.index('function renderExtra(){'):]
fn = fn[:fn.index('\n/* ============ BOOT')]
def pair(start):
    i = fn.index(start)
    m = re.compile(r"L\(`(.*?)`,`(.*?)`\)|L\('(.*?)','(.*?)'\)", re.S).match(fn, i)
    g = m.groups()
    return (g[0], g[1]) if g[0] is not None else (g[2], g[3])
intro = pair("L(`A catalogue")
books = pair("L('Detailed notes on one book")
p = sys.argv[2]
d = json.load(open(p, encoding='utf-8'))
d['ui'] = {'extra_intro_en': intro[0].replace('${tot}', '{tot}'), 'extra_intro_zh': intro[1].replace('${tot}', '{tot}'),
           'extra_books_en': books[0], 'extra_books_zh': books[1]}
d['version'] = sys.argv[3]
json.dump(d, open(p, 'w', encoding='utf-8'), ensure_ascii=False, separators=(',', ':'))
print({k: v[:60] for k, v in d['ui'].items()})
