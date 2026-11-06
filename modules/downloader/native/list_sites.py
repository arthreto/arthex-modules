"""Writes the sites a yt-dlp knows, as the search proposes them: list_sites.py <its folder> <output.json>.

They come from its own test links and link patterns, as it keeps no list of them.
"""

import json
import re
import sys
from urllib.parse import urlsplit

sys.path.insert(0, sys.argv[1])

from yt_dlp.extractor import gen_extractor_classes  # noqa: E402

# Starts where a host can start, so that the end of "uni-(?:a|b)-berlin\.de" is not taken for one.
LITERAL = re.compile(r"(?<![a-z0-9\-)\]?*+}])(?:[a-z0-9][a-z0-9-]*\\\.)+[a-z]{2,}(?![a-z0-9-])")
HOST = re.compile(r"[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*\.[a-z]{2,}")
PREFIX = re.compile(r"^(?:www\d?|m|mobile|web|embed|player|video|videos|play|en|fr|de|es)\.")
# Examples and the servers behind the sites, where nobody finds a video to download.
IGNORED = {
    "example.com", "example.org", "example.net", "github.com", "w3.org", "cloudflare.com", "amazonaws.com",
    "googleapis.com", "akamaihd.net", "cloudfront.net", "google.com", "youtube-nocookie.com", "gstatic.com",
    "googlevideo.com", "schema.org",
}
FILE_ENDINGS = (
    ".php", ".html", ".htm", ".json", ".m3u8", ".mp4", ".mp3", ".js", ".aspx", ".xml", ".f4m", ".mpd", ".cgi",
    ".asp", ".jsp", ".swf", ".smil", ".ism", ".png", ".jpg",
)


def clean(host):
    host = host.lower().strip(".")
    while PREFIX.match(host) and host.count(".") > 1:
        host = PREFIX.sub("", host)
    return host


def is_site(host):
    return host not in IGNORED and not host.endswith(FILE_ENDINGS) and HOST.fullmatch(host) is not None


def hosts_of(extractor):
    found = set()
    try:
        tests = list(extractor.get_testcases(include_onlymatching=True))
    except Exception:
        tests = []
    for test in tests:
        url = test.get("url") if isinstance(test, dict) else None
        if url and url.startswith("http"):
            try:
                host = urlsplit(url).hostname
            except ValueError:
                host = None
            if host:
                found.add(clean(host))
    patterns = getattr(extractor, "_VALID_URL", None)
    for pattern in patterns if isinstance(patterns, (list, tuple)) else [patterns]:
        if isinstance(pattern, str):
            for literal in LITERAL.findall(pattern):
                found.add(clean(literal.replace("\\.", ".")))
    return {host for host in found if is_site(host)}


sites = set()
for extractor in gen_extractor_classes():
    if extractor.__name__ != "GenericIE" and extractor.working():
        sites |= hosts_of(extractor)

if len(sites) < 1000:
    sys.exit(f"only {len(sites)} sites found: the extractors have changed")
with open(sys.argv[2], "w", encoding="utf-8", newline="\n") as out:
    json.dump(sorted(sites), out, indent=0)
    out.write("\n")
print(len(sites), "sites")
