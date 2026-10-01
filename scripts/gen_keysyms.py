#!/usr/bin/env python3
# Genere azure-foundation/src/ui/services/interact/keysyms.rs depuis
# /usr/include/X11/keysymdef.h : les noms de touches XKB (« eacute »,
# « dead_circumflex ») et le caractere de chaque valeur. Relancer apres une
# mise a jour de keysymdef.h : python3 scripts/gen_keysyms.py
import re, sys
src = sys.argv[1] if len(sys.argv) > 1 else "/usr/include/X11/keysymdef.h"
out = "azure-foundation/src/ui/services/interact/keysyms.rs"
names, chars = {}, {}
for line in open(src, encoding="utf-8", errors="replace"):
    m = re.match(r"#define XK_(\w+)\s+0x([0-9a-fA-F]+)\s*(?:/\*\s*\(?U\+([0-9A-Fa-f]{4,6}))?", line)
    if not m:
        continue
    name, value, uni = m.group(1), int(m.group(2), 16), m.group(3)
    keep = uni or name.startswith(("dead_", "KP_", "ISO_Level")) or name in ("NoSymbol", "VoidSymbol")
    if keep:
        names.setdefault(name, value)
    # Les plages triviales (Latin-1, 0x01xxxxxx) sont calculees, pas stockees.
    if uni and not (0x20 <= value <= 0x7E or 0xA0 <= value <= 0xFF or value >= 0x01000000):
        chars.setdefault(value, int(uni, 16))
with open(out, "w") as f:
    f.write("// GENERE par scripts/gen_keysyms.py depuis keysymdef.h - ne pas modifier a la main.\n")
    f.write("// Noms de touches XKB -> valeur, et valeur -> caractere hors des plages\n")
    f.write("// calculees par `keymap::keysym_char`.\n\n")
    f.write("/// `(nom, valeur)`, trie par nom.\n#[rustfmt::skip]\npub(super) const NAMES: &[(&str, u32)] = &[\n")
    for n in sorted(names):
        f.write(f'    ("{n}", 0x{names[n]:x}),\n')
    f.write("];\n\n/// `(valeur, caractere)`, trie par valeur.\n#[rustfmt::skip]\npub(super) const CHARS: &[(u32, u32)] = &[\n")
    for v in sorted(chars):
        f.write(f"    (0x{v:x}, 0x{chars[v]:x}),\n")
    f.write("];\n")
print(f"{len(names)} noms, {len(chars)} caracteres -> {out}")
