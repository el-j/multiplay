#!/usr/bin/env python3
"""Writes DSD64 fixtures (TASK-231): dsd_stereo.dsf and dsd_stereo.dff, 0.25 s each.

Left 440 Hz, right 660 Hz at amplitude 0.5, made with a second-order sigma-delta modulator.
ffmpeg cannot encode DSD, so this offline script does; ffmpeg's DSD decoder checks the result.
Usage: python3 scripts/make_dsd_fixtures.py <output dir>
"""
import math
import struct
import sys

RATE = 2_822_400  # DSD64
SECONDS = 0.25
N = int(RATE * SECONDS)


def modulate(freq):
    """Second-order sigma-delta: one +1/-1 decision per DSD sample, packed MSB first."""
    i1 = i2 = 0.0
    out = bytearray()
    byte = 0
    for n in range(N):
        x = 0.5 * math.sin(2 * math.pi * freq * n / RATE)
        y = 1.0 if i2 >= 0 else -1.0
        i1 += x - y
        i2 += i1 - y
        byte = (byte << 1) | (1 if y > 0 else 0)
        if n % 8 == 7:
            out.append(byte)
            byte = 0
    return bytes(out)


def reverse_bits(data):
    table = bytes(int(f"{b:08b}"[::-1], 2) for b in range(256))
    return data.translate(table)


def write_dsf(path, channels):
    block = 4096
    blocks = math.ceil(len(channels[0]) / block)
    payload = bytearray()
    for b in range(blocks):
        for ch in channels:  # DSF: per-channel blocks, LSB first
            chunk = reverse_bits(ch[b * block:(b + 1) * block])
            payload += chunk + bytes(block - len(chunk))
    data = b"data" + struct.pack("<Q", 12 + len(payload)) + payload
    fmt = b"fmt " + struct.pack("<QIIIIIIQI4x", 52, 1, 0, 2, 2, RATE, 1, N, block)
    total = 28 + len(fmt) + len(data)
    header = b"DSD " + struct.pack("<QQQ", 28, total, 0)
    open(path, "wb").write(header + fmt + data)


def write_dff(path, channels):
    def chunk(cid, body):
        pad = b"\0" if len(body) % 2 else b""
        return cid + struct.pack(">Q", len(body)) + body + pad

    interleaved = bytes(b for pair in zip(*channels) for b in pair)  # DFF: byte-interleaved, MSB first
    prop = b"SND " + chunk(b"FS  ", struct.pack(">I", RATE))
    prop += chunk(b"CHNL", struct.pack(">H", 2) + b"SLFTSRGT")
    prop += chunk(b"CMPR", b"DSD " + bytes([14]) + b"not compressed" + b"\0")
    body = b"DSD " + chunk(b"FVER", struct.pack(">I", 0x01050000)) + chunk(b"PROP", prop)
    body += chunk(b"DSD ", interleaved)
    open(path, "wb").write(b"FRM8" + struct.pack(">Q", len(body)) + body)


if __name__ == "__main__":
    out = sys.argv[1]
    channels = [modulate(440), modulate(660)]
    write_dsf(f"{out}/dsd_stereo.dsf", channels)
    write_dff(f"{out}/dsd_stereo.dff", channels)
