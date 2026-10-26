#!/usr/bin/env python3
"""Read-only Rust test-corpus audit. Stdlib only; no Cargo, no source changes.

Conservative heuristics: exact normalized line windows, similar test bodies,
ignored attributes, and coarse assertion inventory. Inspect before deletion.
"""
from __future__ import annotations
import argparse
from collections import Counter, defaultdict
from difflib import SequenceMatcher
import hashlib
import json
from pathlib import Path
import re

TEST_ATTR = re.compile(r'#\s*\[\s*(?:test|tokio::test)(?:\s*\([^\]]*\))?\s*\]')
FUNC = re.compile(r'\b(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\s*\(')
IGNORED = re.compile(r'#\s*\[\s*ignore\b[^\]]*\]')
ASSERT = re.compile(r'\b(?:assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne|panic|unreachable)!\s*\(')
STEP = re.compile(r'\.\s*(?:update|step)\s*\(')
BUILD = re.compile(r'\b(?:App::new|build_visible_app|Platformer2dSimHarness::new[_A-Za-z0-9]*)\s*\(')
PROBE = re.compile(r'^(?:probe_|measure_|which_|name_)')
TOKEN = re.compile(r'[A-Za-z_][A-Za-z_0-9]*|\d+(?:\.\d+)?|"(?:\\.|[^"\\])*"|\S')


def mask_comments(source: str) -> str:
    """Preserve newlines, strings and positions while removing Rust comments."""
    out = list(source)
    i, n = 0, len(source)
    while i < n:
        if source[i:i+2] == '//':
            j = source.find('\n', i)
            if j < 0: j = n
            for k in range(i,j): out[k] = ' '
            i = j
        elif source[i:i+2] == '/*':
            depth, j = 1, i+2
            while j < n and depth:
                if source[j:j+2] == '/*': depth += 1; j += 2
                elif source[j:j+2] == '*/': depth -= 1; j += 2
                else: j += 1
            for k in range(i,j):
                if source[k] != '\n': out[k] = ' '
            i = j
        elif source[i] in ('"', "'") and (source[i] == '"' or i+2<n and source[i+2]=="'"):
            q=source[i];j=i+1
            while j<n:
                if source[j]=='\\': j+=2; continue
                if source[j]==q: j+=1;break
                j+=1
            i=j
        else: i+=1
    return ''.join(out)


def closing_brace(s: str, start: int) -> int:
    depth=0
    for i in range(start,len(s)):
        if s[i]=='{': depth+=1
        elif s[i]=='}':
            depth-=1
            if depth==0:return i+1
    return len(s)


def get_tests(source: str, path: str) -> list[dict]:
    masked=mask_comments(source)
    result=[]
    for attr in TEST_ATTR.finditer(masked):
        m=FUNC.search(masked,attr.end())
        if not m or m.start()-attr.end()>550: continue
        between=masked[attr.end():m.start()]
        # Only intervening attributes, attributes with simple arguments, and whitespace.
        if re.sub(r'#\s*\[[^\]]*\]|\s+', '', between): continue
        brace=masked.find('{',m.end())
        if brace<0:continue
        end=closing_brace(masked,brace)
        raw=source[brace:end]
        clean=masked[brace:end]
        line=source.count('\n',0,m.start())+1
        attrs=masked[max(0,attr.start()-260):m.start()]
        ignored=bool(IGNORED.search(between))
        tokens=TOKEN.findall(clean)
        # Similarity on meaningful symbols, strip punctuation/noise but retain assertion/operation names.
        symbols=[x for x in tokens if re.match(r'^[A-Za-z_]', x) and x not in {'let','mut','use','as','if','else','for','in','self','true','false','Some','None','Ok','Err'}]
        result.append(dict(path=path,name=m.group(1),line=line,lines=raw.count('\n')+1,
            ignored=ignored,probe=bool(PROBE.search(m.group(1))),
            assertions=len(ASSERT.findall(clean)),steps=len(STEP.findall(clean)),
            builds=len(BUILD.findall(clean)),tokens=len(tokens),symbols=symbols,
            fingerprint=hashlib.sha256(' '.join(tokens).encode()).hexdigest()[:16]))
    return result


def grouped_duplicates(file_data: list[tuple[str,str]], window: int, min_repeats: int) -> list[dict]:
    occurrences=defaultdict(list)
    for path,source in file_data:
        masked=mask_comments(source)
        lines=masked.splitlines()
        meaningful=[(i+1,re.sub(r'\s+',' ',x).strip()) for i,x in enumerate(lines) if x.strip()]
        for i in range(len(meaningful)-window+1):
            block=meaningful[i:i+window]
            fingerprint='\n'.join(x[1] for x in block)
            # Ignore very low-information windows: punctuation and braces only.
            if sum(c.isalpha() for c in fingerprint)<45:continue
            h=hashlib.blake2b(fingerprint.encode(),digest_size=12).hexdigest()
            occurrences[h].append((path,block[0][0],block[-1][0],fingerprint))
    found=[]
    for group in occurrences.values():
        # Remove overlapping windows in the same file; repeated snippets should be distinct.
        kept=[]
        for occurrence in sorted(group,key=lambda x:(x[0],x[1])):
            if kept and kept[-1][0]==occurrence[0] and occurrence[1]<=kept[-1][2]:continue
            kept.append(occurrence)
        if len(kept)<min_repeats:continue
        found.append(dict(repeats=len(kept),locations=[dict(path=p,line=start,end_line=end) for p,start,end,_ in kept[:30]],
                          example=kept[0][3][:700],cross_file=len({x[0] for x in kept})>1))
    found.sort(key=lambda x:(x['repeats'],x['cross_file'],len(x['example'])),reverse=True)
    return found


def similarity(tests: list[dict], threshold: float, limit: int) -> list[dict]:
    # Exact or nearly equal symbol sequences; block by first 16 symbols / length to bound work.
    blocks=defaultdict(list)
    for i,t in enumerate(tests):
        sy=t['symbols']
        if len(sy)<35:continue
        key=(len(sy)//60,tuple(sy[:5]))
        blocks[key].append(i)
    results=[]
    for group in blocks.values():
        for aidx,i in enumerate(group):
            for j in group[aidx+1:]:
                a,b=tests[i],tests[j]
                if abs(len(a['symbols'])-len(b['symbols']))/max(len(a['symbols']),len(b['symbols']))>0.23:continue
                ratio=SequenceMatcher(None,a['symbols'],b['symbols'],autojunk=True).ratio()
                if ratio>=threshold:
                    results.append(dict(score=round(ratio,3),left={'path':a['path'],'test':a['name'],'line':a['line']},right={'path':b['path'],'test':b['name'],'line':b['line']},same_file=a['path']==b['path']))
    return sorted(results,key=lambda x:x['score'],reverse=True)[:limit]


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--root',type=Path,default=Path('.'))
    ap.add_argument('--scope',default='game/ambition_app',help='relative path, or . for entire repo')
    ap.add_argument('--out',type=Path,default=Path('/tmp/ambition-test-audit'))
    ap.add_argument('--window',type=int,default=10)
    ap.add_argument('--threshold',type=float,default=.88)
    ap.add_argument('--pair-limit',type=int,default=250)
    args=ap.parse_args()
    root=args.root.resolve(); base=(root/args.scope).resolve()
    if not base.is_relative_to(root):ap.error('scope escapes root')
    files=sorted(p for p in base.rglob('*.rs') if p.is_file() and not any(part in ('target','.git') for part in p.relative_to(base).parts))
    data=[];tests=[]; inventory=[]
    for p in files:
        path=p.relative_to(root).as_posix()
        try:s=p.read_text(encoding='utf-8')
        except UnicodeError:continue
        if '/tests/' not in '/'+path and not re.search(r'(?:_tests?|/tests?)\.rs$',path) and '#[test]' not in s and '#[cfg(test)]' not in s:continue
        t=get_tests(s,path)
        tests.extend(t);data.append((path,s))
        lines=s.splitlines()
        inventory.append(dict(path=path,lines=len(lines),test_count=len(t),ignored=sum(z['ignored'] for z in t),
            comment_lines=sum(l.strip().startswith('//') for l in lines),
            assertions=sum(z['assertions'] for z in t),
            builds=sum(z['builds'] for z in t)))
    duplicates=grouped_duplicates(data,args.window,3)
    pairs=similarity(tests,args.threshold,args.pair_limit)
    safe_tests=[{k:v for k,v in t.items() if k!='symbols'} for t in tests]
    stats=dict(scope=args.scope,files=len(data),test_functions=len(tests),ignored=sum(t['ignored'] for t in tests),
        probe_named=sum(t['probe'] for t in tests),total_file_lines=sum(f['lines'] for f in inventory),
        duplicate_groups=len(duplicates),near_duplicate_pairs=len(pairs))
    report=dict(summary=stats,files=sorted(inventory,key=lambda x:x['lines'],reverse=True),tests=safe_tests,
        repeated_blocks=duplicates,near_duplicate_tests=pairs)
    args.out.mkdir(parents=True,exist_ok=True)
    (args.out/'audit.json').write_text(json.dumps(report,indent=2)+'\n')
    md=['# Rust test corpus static audit','',f"Scope: `{args.scope}`",'',
        '## Inventory','',f"- Rust test-bearing files: {stats['files']}",f"- Extracted test functions: {stats['test_functions']}",
        f"- Ignored tests (heuristic): {stats['ignored']}",f"- Probe-named tests: {stats['probe_named']}",
        f"- Repeated block groups: {len(duplicates)}",f"- Near-duplicate test pairs: {len(pairs)}",'',
        '## Largest files','', '| File | LOC | Tests | Ignored | Comment LOC |','|---|---:|---:|---:|---:|']
    for f in sorted(inventory,key=lambda x:x['lines'],reverse=True)[:45]:
        md.append(f"| `{f['path']}` | {f['lines']} | {f['test_count']} | {f['ignored']} | {f['comment_lines']} |")
    md.extend(['','## Repeated source blocks (top 45)','', 'Conservative whitespace-normalized matches; repeated initialization is not evidence of redundant behavioral coverage.',''])
    for d in duplicates[:45]:
        loc=', '.join(f"`{l['path']}:{l['line']}`" for l in d['locations'][:9])
        md.append(f"- **{d['repeats']} occurrences**: {loc}")
        md.append('  ```rust\n  '+d['example'][:300].replace('\n','\n  ')+'\n  ```')
    md.extend(['','## Near-duplicate test functions','', 'Candidate review only; preserve distinct failure modes.',''])
    for d in pairs[:75]:
        md.append(f"- {d['score']}: `{d['left']['path']}::{d['left']['test']}` / `{d['right']['path']}::{d['right']['test']}`")
    md.extend(['','## Ignored tests (first 100)',''])
    for t in [t for t in tests if t['ignored']][:100]:md.append(f"- `{t['path']}:{t['line']}` `{t['name']}`")
    md.extend(['','## Caveats','', '- This is static evidence, not mutation coverage or an execution-time profile.',
               '- Rust macros, `cfg_attr`, raw strings, and unusual attribute syntax can cause classification misses.',
               '- Similarity scores indicate repeated structure; do not delete tests based on score alone.',
               '- Preserve a test that uniquely covers a production boundary or historically observed regression.',''])
    (args.out/'audit.md').write_text('\n'.join(md)+'\n')
    print(json.dumps(stats,indent=2))
    print('Report:',args.out/'audit.md')

if __name__=='__main__':main()
