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
        content, quotes = line, 0
        while match := re.match(r'^ {0,3}>[ \t]?', content):
            content = content[match.end():]
            quotes += 1
        # A quoted block's fence cannot hide real links after that quote ends.
        if fence and quotes < fence[1]:
            fence = None
        match = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', content)
        if fence:
            token, depth = fence
            if match and quotes == depth and match[1][0] == token[0] and len(match[1]) >= len(token) and not match[2].strip():
                fence = None
            lines.append('')
        elif match:
            fence = (match[1], quotes)
            lines.append('')
        else:
            lines.append(line)
    return '\n'.join(lines)


def without_inline_code(text):
    return re.sub(r'(`+)(?!`)([\s\S]*?)(?<!`)\1(?!`)', lambda m: ' ' * len(m[0]), text)


def slug(heading):
    heading = re.sub(r'<[^>]*>', '', rendered_links(heading))
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


def balanced_end(text, start, opening, closing):
    """End after matching delimiter, preserving nested/escaped link syntax."""
    depth, escaped, angled = 1, False, False
    for index in range(start + 1, len(text)):
        char = text[index]
        if escaped:
            escaped = False
        elif char == '\\':
            escaped = True
        elif opening == '(' and char == '<':
            angled = True
        elif opening == '(' and char == '>':
            angled = False
        elif not angled and char == opening:
            depth += 1
        elif not angled and char == closing:
            depth -= 1
            if depth == 0:
                return index + 1
    return None


def inline_links(text, definitions=None):
    definitions = definitions or {}
    suffixes = []
    for index, char in enumerate(text):
        if char != '[' or any(start <= index < end for start, end in suffixes):
            continue
        # Escaped brackets are text, not links; an even backslash run is literal.
        preceding = len(text[:index]) - len(text[:index].rstrip('\\'))
        if preceding % 2:
            continue
        label_end = balanced_end(text, index, '[', ']')
        if label_end is None:
            continue
        label = text[index + 1:label_end - 1]
        start = index - 1 if index and text[index - 1] == '!' else index
        if text[label_end:label_end + 1] == '(':
            end = balanced_end(text, label_end, '(', ')')
            if end is None:
                continue
            body = text[label_end + 1:end - 1].strip()
            target = body[1:body.find('>')] if body.startswith('<') else re.split(r'\s+[\'"]', body, maxsplit=1)[0]
            suffixes.append((label_end, end))
            yield start, end, label, re.sub(r'\\([() ])', r'\1', target)
        else:
            ref = re.match(r'\[([^]]*)\]', text[label_end:])
            key = ref[1] if ref and ref[1] else label
            target = definitions.get(' '.join(key.casefold().split()))
            if target is not None:
                end = label_end + len(ref[0]) if ref else label_end
                suffixes.append((label_end, end))
                yield start, end, label, target


def rendered_links(text):
    """Heading text excludes destinations, including nested parentheses."""
    result, cursor = [], 0
    for start, end, label, _target in inline_links(text):
        if start < cursor:
            continue
        result.extend((text[cursor:start], rendered_links(label)))
        cursor = end
    result.append(text[cursor:])
    return ''.join(result)


def destinations(text):
    text = without_inline_code(without_fences(text))
    definitions = {}
    for match in re.finditer(r'^ {0,3}\[([^]]+)\]:\s*(<[^>]*>|\S+)', text, re.M):
        definitions[' '.join(match[1].casefold().split())] = match[2].strip('<>')
    # Visit both outer links and inner images, but never parse URL/title contents.
    for start, _end, _label, target in inline_links(text, definitions):
        yield text.count('\n', 0, start) + 1, target


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
