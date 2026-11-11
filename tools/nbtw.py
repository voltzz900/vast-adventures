"""A small NBT reader and writer (gzip, big-endian) for structure templates.

Values are typed with the classes below; a plain dict is a compound, a str a string, a bool a byte.
"""
import gzip
import struct


class Byte(int): pass
class Short(int): pass
class Int(int): pass
class Long(int): pass
class Float(float): pass
class Double(float): pass


class List(list):
    """A list of one tag type (`kind`, a tag id; 0 for an empty list)."""

    def __init__(self, kind, items=()):
        super().__init__(items)
        self.kind = kind


class IntArray(list): pass


def tag_id(value):
    if isinstance(value, bool) or isinstance(value, Byte):
        return 1
    for cls, tid in ((Short, 2), (Int, 3), (Long, 4), (Float, 5), (Double, 6)):
        if isinstance(value, cls):
            return tid
    if isinstance(value, str):
        return 8
    if isinstance(value, IntArray):
        return 11
    if isinstance(value, List):
        return 9
    if isinstance(value, dict):
        return 10
    if isinstance(value, int):
        return 3
    if isinstance(value, float):
        return 6
    raise TypeError(f"no NBT type for {value!r}")


def _payload(value, out):
    t = tag_id(value)
    if t == 1:
        out += struct.pack(">b", int(value))
    elif t == 2:
        out += struct.pack(">h", value)
    elif t == 3:
        out += struct.pack(">i", value)
    elif t == 4:
        out += struct.pack(">q", value)
    elif t == 5:
        out += struct.pack(">f", value)
    elif t == 6:
        out += struct.pack(">d", value)
    elif t == 8:
        data = value.encode("utf-8")
        out += struct.pack(">H", len(data)) + data
    elif t == 9:
        out += struct.pack(">bi", value.kind if value else 0, len(value))
        for item in value:
            _payload(item, out)
    elif t == 10:
        for key, item in value.items():
            name = key.encode("utf-8")
            out += struct.pack(">bH", tag_id(item), len(name)) + name
            _payload(item, out)
        out += b"\0"
    elif t == 11:
        out += struct.pack(">i", len(value)) + struct.pack(f">{len(value)}i", *value)


def write(root, name=""):
    """The gzipped NBT file for compound `root`."""
    out = bytearray()
    data = name.encode("utf-8")
    out += struct.pack(">bH", 10, len(data)) + data
    _payload(root, out)
    return gzip.compress(bytes(out), mtime=0)


def _read(b, i, t):
    if t == 1:
        return Byte(struct.unpack_from(">b", b, i)[0]), i + 1
    if t == 2:
        return Short(struct.unpack_from(">h", b, i)[0]), i + 2
    if t == 3:
        return Int(struct.unpack_from(">i", b, i)[0]), i + 4
    if t == 4:
        return Long(struct.unpack_from(">q", b, i)[0]), i + 8
    if t == 5:
        return Float(struct.unpack_from(">f", b, i)[0]), i + 4
    if t == 6:
        return Double(struct.unpack_from(">d", b, i)[0]), i + 8
    if t == 8:
        n = struct.unpack_from(">H", b, i)[0]
        return b[i + 2:i + 2 + n].decode("utf-8"), i + 2 + n
    if t == 9:
        kind = b[i]
        n = struct.unpack_from(">i", b, i + 1)[0]
        i += 5
        out = List(kind)
        for _ in range(n):
            v, i = _read(b, i, kind)
            out.append(v)
        return out, i
    if t == 10:
        out = {}
        while True:
            tt = b[i]
            i += 1
            if tt == 0:
                return out, i
            n = struct.unpack_from(">H", b, i)[0]
            key = b[i + 2:i + 2 + n].decode("utf-8")
            out[key], i = _read(b, i + 2 + n, tt)
    if t == 11:
        n = struct.unpack_from(">i", b, i)[0]
        return IntArray(struct.unpack_from(f">{n}i", b, i + 4)), i + 4 + 4 * n
    raise ValueError(f"NBT tag {t}")


def read(data):
    """The root compound of a gzipped NBT file."""
    b = gzip.decompress(data)
    n = struct.unpack_from(">H", b, 1)[0]
    root, _ = _read(b, 3 + n, 10)
    return root
