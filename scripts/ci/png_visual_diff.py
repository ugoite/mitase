#!/usr/bin/env python3
"""Dependency-free RGB/RGBA PNG visual comparison for Workbench baselines."""

import struct
import sys
import zlib


def decode(path):
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    offset = 8
    compressed = bytearray()
    width = height = color = None
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        body = data[offset + 8 : offset + 8 + length]
        offset += 12 + length
        if kind == b"IHDR":
            width, height, depth, color = struct.unpack(">IIBB", body[:10])
            if depth != 8 or color not in (2, 6):
                raise ValueError(f"{path}: expected 8-bit RGB/RGBA PNG")
        elif kind == b"IDAT":
            compressed.extend(body)
        elif kind == b"IEND":
            break
    channels = 3 if color == 2 else 4
    raw = zlib.decompress(compressed)
    stride = width * channels
    rows = []
    previous = bytearray(stride)
    cursor = 0
    for _ in range(height):
        filter_type = raw[cursor]
        cursor += 1
        scanline = bytearray(raw[cursor : cursor + stride])
        cursor += stride
        for index in range(stride):
            left = scanline[index - channels] if index >= channels else 0
            up = previous[index]
            upper_left = previous[index - channels] if index >= channels else 0
            if filter_type == 1:
                scanline[index] = (scanline[index] + left) & 255
            elif filter_type == 2:
                scanline[index] = (scanline[index] + up) & 255
            elif filter_type == 3:
                scanline[index] = (scanline[index] + ((left + up) // 2)) & 255
            elif filter_type == 4:
                estimate = left + up - upper_left
                distances = (abs(estimate - left), abs(estimate - up), abs(estimate - upper_left))
                predictor = (left, up, upper_left)[distances.index(min(distances))]
                scanline[index] = (scanline[index] + predictor) & 255
            elif filter_type != 0:
                raise ValueError(f"{path}: unknown PNG filter {filter_type}")
        rows.append(bytes(scanline))
        previous = scanline
    rgb = bytearray()
    for row in rows:
        for index in range(0, len(row), channels):
            rgb.extend(row[index : index + 3])
    return width, height, rgb


def main(reference, actual):
    ref_width, ref_height, ref = decode(reference)
    width, height, image = decode(actual)
    if (ref_width, ref_height) != (width, height):
        raise SystemExit(f"visual size mismatch: expected {ref_width}x{ref_height}, got {width}x{height}")
    differences = [abs(left - right) for left, right in zip(ref, image)]
    mean_error = sum(differences) / len(differences)
    changed_pixels = sum(
        1
        for index in range(0, len(differences), 3)
        if max(differences[index : index + 3]) > 36
    )
    changed_ratio = changed_pixels / (width * height)
    print(f"{actual}: mean_error={mean_error:.2f}, changed_pixels={changed_ratio:.2%}")
    if mean_error > 6 or changed_ratio > 0.03:
        raise SystemExit("visual regression exceeds mean-error or changed-pixel threshold")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
