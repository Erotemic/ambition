"""Find RON field boundaries without reformatting the document."""

from __future__ import annotations

import json
import re

_TOKEN = re.compile(
    r'\s+|//[^\n]*|/\*|r(?P<hashes>#{0,255})".*?"(?P=hashes)|"(?:\\.|[^"\\])*"'
    r"|'(?:\\.|[^'\\])*'|[A-Za-z_][A-Za-z0-9_]*|[^\s]",
    re.S,
)
_COMMENT = re.compile(r"/\*|\*/")
_CLOSE = {"(": ")", "{": "}", "[": "]"}


class Document:
    def __init__(self, text):
        self.text = text
        self.tokens = []
        self.ends = {}
        openings = []
        position = 0
        while position < len(text):
            match = _TOKEN.match(text, position)
            token = match[0]
            position = match.end()
            if token == "/*":
                depth = 1
                for comment in _COMMENT.finditer(text, position):
                    depth += 1 if comment[0] == "/*" else -1
                    if depth == 0:
                        position = comment.end()
                        break
                if depth:
                    raise ValueError("A RON comment is incomplete.")
                continue
            if token.isspace() or token.startswith("//"):
                continue
            index = len(self.tokens)
            self.tokens.append(match)
            if token in _CLOSE:
                openings.append(index)
            elif token in _CLOSE.values():
                if not openings or _CLOSE[self[openings[-1]]] != token:
                    raise ValueError("The RON delimiters do not match.")
                self.ends[openings.pop()] = index
        if openings:
            raise ValueError("A RON value is incomplete.")

    def __getitem__(self, at):
        return self.tokens[at][0]

    def root(self):
        for index in self.children(0, len(self.tokens)):
            if self[index] == "(":
                return index
        raise ValueError("The RON document has no root record.")

    def children(self, start, end):
        while start < end:
            yield start
            start = self.ends.get(start, start) + 1

    def entries(self, opening):
        """Return the keys and the first and last tokens of their values."""
        end = self.ends.get(opening)
        if end is None:
            raise ValueError("Expected a RON record or map.")
        entries = {}
        start = opening + 1
        for at in [*self.children(start, end), end]:
            if at != end and self[at] != ",":
                continue
            if start < at:
                if at - start < 3 or self[start + 1] != ":":
                    raise ValueError("Expected a named RON field.")
                name = self[start] if self[start].isidentifier() else self.string(start)
                if name in entries:
                    raise ValueError(f"The RON field {name!r} occurs more than once.")
                entries[name] = (start + 2, at - 1)
            start = at + 1
        return entries

    def string(self, at):
        match = self.tokens[at]
        hashes = match.group("hashes")
        if hashes is not None:
            return match[0][len(hashes) + 2:-(len(hashes) + 1)]
        if match[0].startswith('"'):
            return json.loads(match[0])
        raise ValueError("Expected a RON string.")
