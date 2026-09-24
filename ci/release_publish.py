#!/usr/bin/env python3
import hashlib, json, os, re, sys, time, tomllib, urllib.error, urllib.parse, urllib.request
from datetime import datetime, timedelta
API_ROOT = "https://git.home.arpa/api/v1"
OWNER, REPO = "HOMESERVERSLTD", "arcadia"
PROJECT = f"{OWNER}/{REPO}"
RELEASES = f"{API_ROOT}/repos/{OWNER}/{REPO}/releases"
RELEASE_FLAG = "release.flag"
def fail(message):
    print(f"release_publish: {message}", file=sys.stderr); raise SystemExit(1)
def release_tag(sha):
    return f"sha-{sha}"
def request(method, url, token, body=None, content_type=None, accept=None):
    headers = {"Authorization": f"token {token}", "User-Agent": "arcadia-woodpecker-release"}
    if content_type: headers["Content-Type"] = content_type
    if accept: headers["Accept"] = accept
    if isinstance(body, (dict, list)):
        body = json.dumps(body, separators=(",", ":")).encode(); headers["Content-Type"] = "application/json"
    try:
        req = urllib.request.Request(url, data=body, headers=headers, method=method)
        with urllib.request.urlopen(req, timeout=180) as response: return response.status, response.read()
    except urllib.error.HTTPError as exc: return exc.code, exc.read()
    except (urllib.error.URLError, TimeoutError, OSError) as exc: fail(f"{method} {url} transport failure: {exc}")
def decode(raw, description):
    try: return json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError): fail(f"{description} returned invalid JSON")
def assets_of(release):
    assets = release.get("assets")
    if not isinstance(assets, list): fail("release response has no asset list")
    result = {}
    for asset in assets:
        name = asset.get("name") if isinstance(asset, dict) else None
        if not isinstance(name, str) or name in result: fail("release contains an invalid or duplicate asset name")
        result[name] = asset
    return result
def download(asset, token, name):
    url = asset.get("browser_download_url") if isinstance(asset, dict) else None
    if not isinstance(url, str) or not url: fail(f"asset {name} has no browser download URL")
    status, raw = request("GET", url, token, accept="application/octet-stream")
    if status != 200: fail(f"download of {name} returned HTTP {status}")
    return raw
def expected_assets(release, binary_name, sidecar_name):
    assets = assets_of(release)
    for name in (binary_name, sidecar_name):
        if name not in assets: fail(f"release is missing expected asset {name}")
    return assets

def verify_release_identity(release, sha, tag):
    if release.get("target_commitish") != sha: fail("existing release target_commitish conflicts with source SHA")
    if release.get("tag_name") != tag: fail("existing release tag_name conflicts with derived tag")

def verify_existing(release, token, binary_name, sidecar_name, digest, sidecar, sha, tag):
    verify_release_identity(release, sha, tag)
    assets = expected_assets(release, binary_name, sidecar_name)
    binary = download(assets[binary_name], token, binary_name)
    if hashlib.sha256(binary).hexdigest() != digest: fail(f"downloaded {binary_name} has a conflicting digest")
    if download(assets[sidecar_name], token, sidecar_name) != sidecar: fail(f"downloaded {sidecar_name} has conflicting contents")

def verify_fresh(release, token, binary_name, sidecar_name, digest, sidecar, sha, tag):
    verify_release_identity(release, sha, tag)
    assets = expected_assets(release, binary_name, sidecar_name)
    if hashlib.sha256(download(assets[binary_name], token, binary_name)).hexdigest() != digest: fail(f"downloaded {binary_name} has a conflicting digest")
    if download(assets[sidecar_name], token, sidecar_name) != sidecar: fail(f"downloaded {sidecar_name} has conflicting contents")
def canonical_flag_bytes(sha, digest, flagged_at, pipeline_url):
    payload = {
        "schema": "estate.release-flag.v1",
        "component": REPO,
        "source_sha": sha,
        "sha256": digest,
        "flagged_at": flagged_at,
        "pipeline_url": pipeline_url,
    }
    return (json.dumps(payload, indent=2) + "\n").encode("utf-8")

def valid_utc_flagged_at(value):
    if not isinstance(value, str) or not value:
        fail("existing release.flag has no valid flagged_at")
    parsed_value = value[:-1] + "+00:00" if value.endswith("Z") else value
    try:
        parsed = datetime.fromisoformat(parsed_value)
    except ValueError:
        fail("existing release.flag has an invalid flagged_at")
    if parsed.tzinfo is None or parsed.utcoffset() != timedelta(0):
        fail("existing release.flag flagged_at must be UTC")
    return value

def flag_from_release(release, token, expected, description, sha, tag):
    verify_release_identity(release, sha, tag)
    assets = assets_of(release)
    asset = assets.get(RELEASE_FLAG)
    if asset is None: fail(f"{description} has no {RELEASE_FLAG} asset")
    actual = download(asset, token, RELEASE_FLAG)
    if actual != expected: fail(f"{description} has conflicting contents")

def flag_release(token, sha, pipeline_url):
    target_directory = os.environ.get("CARGO_TARGET_DIR", "target")
    binary_name = "arcadia-x86_64"; sidecar_name = "arcadia-x86_64.sha256"
    binary_path = os.path.join(target_directory, "release", REPO)
    if not os.path.isfile(binary_path): fail(f"release binary does not exist: {binary_path}")
    with open(binary_path, "rb") as binary_file: binary = binary_file.read()
    digest = hashlib.sha256(binary).hexdigest()
    sidecar = f"{digest}  {binary_name}\n".encode("ascii")
    tag = release_tag(sha)
    tag_url = f"{RELEASES}/tags/{urllib.parse.quote(tag, safe='')}"
    status, raw = request("GET", tag_url, token)
    if status != 200: fail(f"GET release tag returned HTTP {status}")
    release = decode(raw, "existing release")
    verify_existing(release, token, binary_name, sidecar_name, digest, sidecar, sha, tag)
    assets = assets_of(release)
    if RELEASE_FLAG in assets:
        existing = download(assets[RELEASE_FLAG], token, RELEASE_FLAG)
        existing_obj = decode(existing, "existing release.flag")
        if not isinstance(existing_obj, dict): fail("existing release.flag is not a JSON object")
        flagged_at = valid_utc_flagged_at(existing_obj.get("flagged_at"))
        expected = canonical_flag_bytes(sha, digest, flagged_at, pipeline_url)
        if existing != expected: fail("existing release.flag has conflicting contents")
        print(json.dumps({"status":"noop-flag-identical", "tag":tag, "asset":RELEASE_FLAG, "sha256":digest}, separators=(",", ":"))); return
    flagged_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    expected = canonical_flag_bytes(sha, digest, flagged_at, pipeline_url)
    release_id = release.get("id")
    if not isinstance(release_id, int) or isinstance(release_id, bool): fail("release has no numeric id")
    upload_url = f"{RELEASES}/{release_id}/assets?{urllib.parse.urlencode({'name':RELEASE_FLAG})}"
    upload_status, _ = request("POST", upload_url, token, expected, content_type="application/json")
    if upload_status == 409:
        reread_status, reread_raw = request("GET", tag_url, token)
        if reread_status != 200: fail(f"release flag collision reread returned HTTP {reread_status}")
        reread = decode(reread_raw, "release flag collision reread")
        flag_from_release(reread, token, expected, "release flag collision reread", sha, tag)
        print(json.dumps({"status":"noop-flag-identical", "tag":tag, "asset":RELEASE_FLAG, "sha256":digest}, separators=(",", ":"))); return
    if upload_status not in (200, 201): fail(f"upload of {RELEASE_FLAG} returned HTTP {upload_status}")
    reread_status, reread_raw = request("GET", tag_url, token)
    if reread_status != 200: fail(f"reread of release returned HTTP {reread_status}")
    reread = decode(reread_raw, "release reread")
    flag_from_release(reread, token, expected, "uploaded release.flag", sha, tag)
    print(json.dumps({"status":"flagged", "tag":tag, "asset":RELEASE_FLAG, "sha256":digest}, separators=(",", ":")))

def main():
    flag_mode = "--flag" in sys.argv[1:]
    token = os.environ.get("FORGEJO_TOKEN", "")
    if not token: fail("FORGEJO_TOKEN is required")
    sha = os.environ.get("CI_COMMIT_SHA", "")
    if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha): fail("CI_COMMIT_SHA must be exactly 40 lowercase hexadecimal characters")
    if flag_mode:
        pipeline_url = os.environ.get("CI_PIPELINE_URL", "")
        if not pipeline_url or not pipeline_url.strip(): fail("CI_PIPELINE_URL is required")
        flag_release(token, sha, pipeline_url); return
    try:
        with open("Cargo.toml", "rb") as cargo_file: cargo = tomllib.load(cargo_file)
    except (OSError, tomllib.TOMLDecodeError) as exc: fail(f"cannot read Cargo metadata: {exc}")
    package = cargo.get("package", {})
    if package.get("name") != REPO: fail(f"Cargo package name must be {REPO}")
    version = package.get("version")
    declarations = cargo.get("bin", [])
    if declarations:
        if len(declarations) != 1 or not isinstance(declarations[0].get("name"), str): fail("Arcadia must declare exactly one binary target")
        binary_decl = declarations[0]["name"]
    else:
        binary_decl = package.get("name")
    if not isinstance(version, str) or not version or not isinstance(binary_decl, str) or not binary_decl: fail("Cargo package version or binary declaration is missing")
    target_directory = os.environ.get("CARGO_TARGET_DIR", "target")
    binary_name = "arcadia-x86_64"; sidecar_name = "arcadia-x86_64.sha256"
    binary_path = os.path.join(target_directory, "release", binary_decl)
    if not os.path.isfile(binary_path): fail(f"release binary does not exist: {binary_path}")
    with open(binary_path, "rb") as binary_file: binary = binary_file.read()
    digest = hashlib.sha256(binary).hexdigest(); sidecar = f"{digest}  arcadia-x86_64\n".encode("ascii")
    name = f"arcadia {sha[:8]}"
    tag = release_tag(sha)
    tag_url = f"{RELEASES}/tags/{urllib.parse.quote(tag, safe='')}"; status, raw = request("GET", tag_url, token)
    if status == 200:
        verify_existing(decode(raw, "existing release"), token, binary_name, sidecar_name, digest, sidecar, sha, tag)
        print(json.dumps({"status":"noop-identical", "tag":tag, "name":name, "assets":[binary_name, sidecar_name], "sha256":digest, "cargo_version":version}, separators=(",", ":"))); return
    if status != 404: fail(f"GET release tag returned HTTP {status}")
    payload = {"tag_name":tag, "name":name, "target_commitish":sha, "draft":False, "prerelease":False}; status, raw = request("POST", RELEASES, token, payload)
    if status == 409:
        status, raw = request("GET", tag_url, token)
        if status != 200: fail(f"release collision reread returned HTTP {status}")
        verify_existing(decode(raw, "existing release"), token, binary_name, sidecar_name, digest, sidecar, sha, tag)
        print(json.dumps({"status":"noop-identical", "tag":tag, "name":name, "assets":[binary_name, sidecar_name], "sha256":digest, "cargo_version":version}, separators=(",", ":"))); return
    if status not in (200, 201): fail(f"release creation returned HTTP {status}")
    release = decode(raw, "release creation"); verify_release_identity(release, sha, tag); release_id = release.get("id")
    if not isinstance(release_id, int): fail("created release has no numeric id")
    if assets_of(release): fail("new release unexpectedly contains assets")
    upload_url = f"{RELEASES}/{release_id}/assets"
    for asset_name, content, content_type in ((binary_name, binary, "application/octet-stream"), (sidecar_name, sidecar, "text/plain; charset=utf-8")):
        url = f"{upload_url}?{urllib.parse.urlencode({'name':asset_name})}"; status, _ = request("POST", url, token, content, content_type=content_type)
        if status not in (200, 201): fail(f"upload of {asset_name} returned HTTP {status}")
    status, raw = request("GET", tag_url, token)
    if status != 200: fail(f"reread of release returned HTTP {status}")
    verify_fresh(decode(raw, "release reread"), token, binary_name, sidecar_name, digest, sidecar, sha, tag)
    print(json.dumps({"status":"published", "tag":tag, "name":name, "assets":[binary_name, sidecar_name], "sha256":digest, "cargo_version":version}, separators=(",", ":")))
if __name__ == "__main__": main()
