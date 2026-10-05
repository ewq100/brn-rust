#!/usr/bin/env python3
"""Check local files/fragments in current Markdown, without network access."""
import argparse
import html
from pathlib import Path
import re
import subprocess
import sys
import unicodedata
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent
# Outgoing links in retained historical evidence are not current requirements.
# Incoming links from current docs to these files/fragments are still checked.
HISTORICAL = (
    'docs/superpowers/', 'docs/audits/', 'docs/architecture/decisions/',
    'docs/work/completed/', 'experiments/',
    'docs/work/active/rig-first-reset/', 'docs/work/active/simple-rig-notes/',
    'docs/work/active/markdown-note-editing/', 'docs/work/active/ui-slice-2-chat-polish/',
)


def without_fences(text):
    lines, fence = [], None
    for line in text.splitlines():
        match = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line)
        if fence:
            if match and match[1][0] == fence[0] and len(match[1]) >= len(fence) and not match[2].strip():
                fence = None
            lines.append('')
        elif match:
            fence = match[1]
            lines.append('')
        else:
            lines.append(line)
    return '\n'.join(lines)


def without_inline_code(text):
    return re.sub(r'(`+)(?!`)([\s\S]*?)(?<!`)\1(?!`)', lambda m: ' ' * len(m[0]), text)


def slug(heading):
    heading = re.sub(r'<[^>]*>', '', heading)
    heading = re.sub(r'!?\[([^]]*)\]\([^)]*\)', r'\1', heading)
    heading = re.sub(r'[\\`*~]', '', html.unescape(heading)).lower()
    return ''.join('-' if c == ' ' else c for c in heading
                   if c == ' ' or c in '-_' or unicodedata.category(c)[0] in 'LMN')


def anchors(text):
    text = without_fences(text)
    result = set(re.findall(r'<(?:a|[a-z][\w-]*)\b[^>]*\b(?:id|name)=[\'"]([^\'"]+)[\'"]', text, re.I))
    occupied = set()
    lines = text.splitlines()
    for i, line in enumerate(lines):
        atx = re.match(r'^ {0,3}#{1,6}\s+(.+?)(?:\s+#+\s*)?$', line)
        heading = atx[1] if atx else None
        if not atx and i + 1 < len(lines) and line.strip() and re.fullmatch(r' {0,3}(?:=+|-+)\s*', lines[i + 1]):
            heading = line.strip()
        if heading is not None:
            base = slug(heading)
            candidate, duplicate = base, 0
            while candidate in occupied:
                duplicate += 1
                candidate = f'{base}-{duplicate}'
            occupied.add(candidate)
            result.add(candidate)
    return result


def destinations(text):
    text = without_inline_code(without_fences(text))
    definitions = {}
    for match in re.finditer(r'^ {0,3}\[([^]]+)\]:\s*(<[^>]*>|\S+)', text, re.M):
        definitions[' '.join(match[1].casefold().split())] = match[2].strip('<>')
    # Parse balanced parentheses rather than dropping a link after its first ')'.
    for match in re.finditer(r'(?<!!)\[([^]\n]+)\]|!\[([^]\n]*)\]', text):
        start = match.end()
        if start < len(text) and text[start] == '(':
            index, depth, escaped, angled = start + 1, 1, False, False
            while index < len(text) and depth:
                c = text[index]
                if escaped:
                    escaped = False
                elif c == '\\':
                    escaped = True
                elif c == '<':
                    angled = True
                elif c == '>':
                    angled = False
                elif not angled and c == '(':
                    depth += 1
                elif not angled and c == ')':
                    depth -= 1
                index += 1
            if depth == 0:
                body = text[start + 1:index - 1].strip()
                target = body[1:body.find('>')] if body.startswith('<') else re.split(r'\s+[\'"]', body, maxsplit=1)[0]
                yield text.count('\n', 0, match.start()) + 1, re.sub(r'\\([() ])', r'\1', target)
        else:
            ref = re.match(r'\[([^]]*)\]', text[start:])
            key = (ref[1] if ref and ref[1] else match[1] or match[2])
            target = definitions.get(' '.join(key.casefold().split()))
            if target is not None:
                yield text.count('\n', 0, match.start()) + 1, target


def check(root, paths):
    errors, count, cache = [], 0, {}
    for path in sorted(paths):
        for line, link in destinations(path.read_text()):
            location = f'{path.relative_to(root)}:{line}: {link}'
            try:
                parsed = urlsplit(link)
            except ValueError:
                errors.append(f'{location}: invalid destination')
                continue
            if parsed.scheme or parsed.netloc:
                continue
            count += 1
            base = root if parsed.path.startswith('/') else path.parent
            target = (base / unquote(parsed.path.lstrip('/'))).resolve() if parsed.path else path.resolve()
            if not target.exists():
                errors.append(f'{location}: missing file')
            elif parsed.fragment and target.suffix.lower() == '.md':
                if target not in cache:
                    cache[target] = anchors(target.read_text())
                if unquote(parsed.fragment) not in cache[target]:
                    errors.append(f'{location}: missing fragment')
    return errors, count


def current_paths(root):
    names = subprocess.check_output(['git', '-C', str(root), 'ls-files', '--cached', '--others', '--exclude-standard', '-z']).split(b'\0')
    return {root / name.decode() for name in names if name.decode().endswith('.md')
            and not name.decode().startswith(HISTORICAL)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--all', action='store_true', help='also audit outgoing historical links (never silently suppressed)')
    opts = parser.parse_args()
    paths = current_paths(ROOT) if not opts.all else {ROOT / name.decode() for name in subprocess.check_output(
        ['git', '-C', str(ROOT), 'ls-files', '--cached', '--others', '--exclude-standard', '-z']).split(b'\0') if name.endswith(b'.md')}
    errors, count = check(ROOT, paths)
    for error in errors:
        print(error, file=sys.stderr)
    print(f'Markdown links: {len(paths)} files, {count} local links, {len(errors)} failures; historical outgoing policy={"audit all" if opts.all else "explicit excluded prefixes"}')
    return int(bool(errors))


if __name__ == '__main__':
    sys.exit(main())
