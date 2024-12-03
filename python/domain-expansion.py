
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
from typing import Iterable, Optional
import urllib.request
import argparse
import json

HEADER = '''▗▄▄▖▗▄▄▖     ▗▄▄▄   ▗▄▖ ▗▖  ▗▖ ▗▄▖ ▗▄▄▄▖▗▖  ▗▖    ▗▄▄▄▖▗▖  ▗▖▗▄▄▖  ▗▄▖ ▗▖  ▗▖ ▗▄▄▖▗▄▄▄▖ ▗▄▖ ▗▖  ▗▖    ▗▄▄▖▗▄▄▖ 
▐▌  ▐▌       ▐▌  █ ▐▌ ▐▌▐▛▚▞▜▌▐▌ ▐▌  █  ▐▛▚▖▐▌    ▐▌    ▝▚▞▘ ▐▌ ▐▌▐▌ ▐▌▐▛▚▖▐▌▐▌     █  ▐▌ ▐▌▐▛▚▖▐▌      ▐▌  ▐▌ 
▐▌  ▐▌       ▐▌  █ ▐▌ ▐▌▐▌  ▐▌▐▛▀▜▌  █  ▐▌ ▝▜▌    ▐▛▀▀▘  ▐▌  ▐▛▀▘ ▐▛▀▜▌▐▌ ▝▜▌ ▝▀▚▖  █  ▐▌ ▐▌▐▌ ▝▜▌      ▐▌  ▐▌ 
 ■   ■       ▐▙▄▄▀ ▝▚▄▞▘▐▌  ▐▌▐▌ ▐▌▗▄█▄▖▐▌  ▐▌    ▐▙▄▄▖▗▞▘▝▚▖▐▌   ▐▌ ▐▌▐▌  ▐▌▗▄▄▞▘▗▄█▄▖▝▚▄▞▘▐▌  ▐▌       ■   ■ 
 ■■■ ■■■                                                                                               ■■■ ■■■'''

@dataclass
class Node:
    name: str
    children: Optional[list[Node]] = None

@dataclass
class Style:
    indent_prefix: str = ''
    t_prefix: str = ''
    last_prefix: Optional[str] = None
    leaf_prefix: Optional[str] = None
    include_root: bool = False

@dataclass
class Options:
    style: Style
    colored: bool
    spaces: int
    include_root: bool

class NodeKind(Enum):
    DEFAULT = 0
    LAST = 1
    ROOT = 2

def generate_node(node: Node, opts: Options, kind: NodeKind=NodeKind.DEFAULT, depth: int=0) -> Iterable[str]:
    # Compute prefix
    if kind == NodeKind.ROOT:
        prefix = ''
    else:
        if node.children is None and opts.style.leaf_prefix:
            prefix = opts.style.leaf_prefix
        elif kind == NodeKind.DEFAULT:
            prefix = opts.style.t_prefix
        elif kind == NodeKind.LAST:
            prefix = opts.style.last_prefix if opts.style.last_prefix is not None else opts.style.t_prefix
        else:
            raise ValueError(f'Unimplemented node kind: {kind}')

    # Compute indent
    if kind == NodeKind.ROOT:
        indent = ''
    elif kind == NodeKind.LAST:
        indent = ' ' * len(prefix)
    else:
        indent = opts.style.indent_prefix + (' ' * (len(prefix) - len(opts.style.indent_prefix)))

    # Assemble lines
    color = PREFIX_TO_COLORS[depth % len(PREFIX_TO_COLORS)] if opts.colored else ''
    reset = '\033[0m' if opts.colored else ''

    yield color + prefix + node.name + reset

    if node.children is None:
        return
    
    for i, child in enumerate(node.children):
        child_kind = NodeKind.LAST if i == len(node.children) - 1 else NodeKind.DEFAULT
            
        for line in list(generate_node(child, opts, child_kind, depth+1)):
            yield color + indent + line + reset

def generate(node: Node, opts: Options) -> str:
    return '\n'.join(generate_node(node, opts, NodeKind.LAST if opts.include_root else NodeKind.ROOT))

STYLES = {
    'ascii': Style(indent_prefix='|', t_prefix='+-', last_prefix='\\-'),
    'ascii2': Style(indent_prefix='|', t_prefix='+-', last_prefix='`-'),
    'ascii-compact': Style(indent_prefix='|', t_prefix='+', last_prefix='\\'),
    'ascii2-compact': Style(indent_prefix='|', t_prefix='+', last_prefix='`'),
    'arrows': Style(indent_prefix='|', t_prefix='->'),
    'harrows': Style(indent_prefix='|', t_prefix='#>'),
    'bars': Style(indent_prefix='|', t_prefix='|'),
    'yaml': Style(t_prefix='-', last_prefix='-'),
    'empty': Style(),
    'compact': Style(indent_prefix='│', t_prefix='├', last_prefix='└'),
    'unicode': Style(indent_prefix='│', t_prefix='├─', last_prefix='└─'),
}

PREFIX_TO_COLORS = [
    '\033[91m',
    '\033[92m',
    '\033[93m',
    '\033[94m',
    '\033[95m',
    '\033[96m',
]

def crtsh(domain: str, colored: bool =True):
    url = f"https://crt.sh/?q=%.{domain}&output=json"
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3'
    }

    common_names = set()

    request = urllib.request.Request(url, headers=headers)

    with urllib.request.urlopen(request) as response:
        data = json.loads(response.read().decode())

    for res in data:
        common_names.add(res['common_name'])

    return create_tree(common_names, colored)

def create_tree(common_names: Iterable, colored: bool = True):
    split_domains = [list(reversed(name.split('.'))) for name in common_names]
    split_domains.sort(key=lambda x: (len(x), x))

    name_to_node = {}

    for domain in split_domains:
        for i in range(len(domain)):
            parent = '.'.join(reversed(domain[:i]))
            current = '.'.join(reversed(domain[:i + 1]))

            if current not in name_to_node:
                node = Node(name=current, children=[])
                name_to_node[current] = node
                actual_parent = name_to_node.get(parent, None)

                if actual_parent:
                    if node not in actual_parent.children:
                        actual_parent.children.append(node)
                elif parent:
                    raise Exception(f"Parent {parent} not found")

    options = Options(style=STYLES['unicode'], colored=colored, spaces=2, include_root=False)
    lines = []

    for node in name_to_node.values():
        if node.name.count('.') == 1:
            lines.append(generate(node, options))

    return '\n'.join(lines)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-d", "--domain", help="Domain name to search")
    parser.add_argument('-n', '--no-color', action='store_true', help='Disable header and colored output')
    args = parser.parse_args()

    if args.domain:
        if not args.no_color:
            for i, line in enumerate(HEADER.split('\n')):
                print(PREFIX_TO_COLORS[(i + 1) % len(PREFIX_TO_COLORS)] + line + '\033[0m')

            print()

        try:
            line = crtsh(args.domain, colored=not args.no_color)

            if line:
                print(line)
            else:
                print('No data found')
        except:
            print('Error: Unable to fetch data from crt.sh')
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
