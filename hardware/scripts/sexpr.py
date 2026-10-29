"""Minimal S-expression reader/writer for KiCad files."""

import re

TOKEN = re.compile(r'\s*(?:(\()|(\))|("(?:[^"\\]|\\.)*")|([^\s()"]+))')


class Sym(str):
    """Unquoted atom (keyword or number)."""


def parse(text):
    stack, cur, pos = [], [], 0
    while True:
        m = TOKEN.match(text, pos)
        if not m or m.end() == pos:
            break
        pos = m.end()
        if m.group(1):
            stack.append(cur)
            cur = []
        elif m.group(2):
            done, cur = cur, stack.pop()
            cur.append(done)
        elif m.group(3) is not None:
            cur.append(m.group(3)[1:-1].replace('\\"', '"').replace("\\\\", "\\"))
        else:
            cur.append(Sym(m.group(4)))
    return cur[0] if len(cur) == 1 else cur


def dump(node, indent=0):
    """Serialise keeping the original item order; nested lists go on their own lines."""
    if isinstance(node, list):
        if not node:
            return "()"
        if all(not isinstance(x, list) for x in node):
            return "(" + " ".join(dump(x) for x in node) + ")"
        out, pad = "(", "\t" * (indent + 1)
        for i, x in enumerate(node):
            if isinstance(x, list):
                out += "\n" + pad + dump(x, indent + 1)
            else:
                out += ("" if i == 0 else " ") + dump(x)
        return out + "\n" + "\t" * indent + ")"
    if isinstance(node, Sym):
        return str(node)
    if isinstance(node, bool):
        return "yes" if node else "no"
    if isinstance(node, (int, float)):
        return fmt_num(node)
    return '"' + str(node).replace("\\", "\\\\").replace('"', '\\"') + '"'


def fmt_num(v):
    if isinstance(v, int):
        return str(v)
    s = f"{v:.4f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def find(node, key):
    return [x for x in node if isinstance(x, list) and x and x[0] == key]


def first(node, key):
    r = find(node, key)
    return r[0] if r else None
