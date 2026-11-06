"""Turns the word vectors of fastText into the file of the Flair game (see `flair.rs` for its format).

Usage: build.py <cc.xx.300.vec.gz, or its URL> <output> [--limit N]

The vectors are rounded to one byte per dimension, which keeps their direction, the only thing the
game reads. A player types in lowercase, so the first and most frequent form of a word stands for
all of them, "Paris" for "paris". Words that cannot be typed, with digits or punctuation, are left out.
"""
import argparse
import gzip
import hashlib
import math
import mmap
import os
import struct
import tempfile
import urllib.request

MAGIC = b'FLR1'
MAX_WORD_BYTES = 64
REPORT_EVERY = 100_000
# A normalised vector of 300 dimensions rarely goes past a third of 1: this uses the whole byte.
SCALE = 127 * 3


def typed_by_a_player(word: str) -> bool:
    return word.replace('-', '').replace("'", '').replace('’', '').isalpha()


def open_source(source: str):
    if source.startswith('https://'):
        request = urllib.request.Request(source, headers={'User-Agent': 'arthex-flair-data'})
        return gzip.open(urllib.request.urlopen(request, timeout=120), 'rt', encoding='utf-8', errors='replace')
    return gzip.open(source, 'rt', encoding='utf-8', errors='replace')


def quantized(values: list[float]) -> bytes:
    size = math.sqrt(sum(value * value for value in values)) or 1.0
    return struct.pack(f'{len(values)}b', *(max(-127, min(127, round(value / size * SCALE))) for value in values))


def build(source: str, output: str, limit: int | None) -> int:
    words = []
    seen = set()
    folder = os.path.dirname(os.path.abspath(output))
    with tempfile.NamedTemporaryFile(dir=folder, delete=False) as raw:
        with open_source(source) as lines:
            dimensions = int(next(lines).split()[1])
            for number, line in enumerate(lines):
                if limit is not None and number >= limit:
                    break
                word, _, numbers = line.rstrip('\n').partition(' ')
                word = word.lower()
                encoded = word.encode('utf-8')
                if word in seen or len(encoded) > MAX_WORD_BYTES or not typed_by_a_player(word):
                    continue
                values = numbers.split(' ')
                if len(values) != dimensions:
                    continue
                seen.add(word)
                raw.write(quantized([float(value) for value in values]))
                words.append(encoded)
                if len(words) % REPORT_EVERY == 0:
                    print(f'{len(words)} words', flush=True)
        temporary = raw.name
    del seen
    order = sorted(range(len(words)), key=words.__getitem__)
    with open(output, 'wb') as out, open(temporary, 'rb') as vectors:
        out.write(MAGIC + struct.pack('<II', len(words), dimensions))
        offset = 0
        for index in order:
            out.write(struct.pack('<I', offset))
            offset += len(words[index])
        out.write(struct.pack('<I', offset))
        for index in order:
            out.write(words[index])
        with mmap.mmap(vectors.fileno(), 0, access=mmap.ACCESS_READ) as view:
            for index in order:
                start = index * dimensions
                out.write(view[start:start + dimensions])
    os.remove(temporary)
    return len(words)


def sha256_of(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, 'rb') as file:
        while chunk := file.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('source')
    parser.add_argument('output')
    parser.add_argument('--limit', type=int)
    arguments = parser.parse_args()
    count = build(arguments.source, arguments.output, arguments.limit)
    print(f'{count} words, {os.path.getsize(arguments.output)} bytes, sha256 {sha256_of(arguments.output)}')


if __name__ == '__main__':
    main()
