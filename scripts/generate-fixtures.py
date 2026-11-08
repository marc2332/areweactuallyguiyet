#!/usr/bin/env python3
"""Generate the shared deterministic dashboard PNG fixtures."""

from pathlib import Path
import struct
import zlib


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


directory = Path(__file__).resolve().parent.parent / "assets" / "images"
directory.mkdir(parents=True, exist_ok=True)
for index in range(10):
    scanlines = bytearray()
    for vertical in range(64):
        scanlines.append(0)
        for horizontal in range(64):
            scanlines.extend(((horizontal * 4 + index * 13) % 256, (vertical * 4 + index * 19) % 256, ((horizontal + vertical) * 2 + index * 23) % 256, 255))
    header = struct.pack(">IIBBBBB", 64, 64, 8, 6, 0, 0, 0)
    image = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(bytes(scanlines), 9)) + chunk(b"IEND", b"")
    (directory / f"dashboard-{index}.png").write_bytes(image)
