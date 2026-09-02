#!/usr/bin/env python3
import hashlib, json, os, re, sys, tomllib, urllib.error, urllib.parse, urllib.request
API_ROOT = "https://git.home.arpa/api/v1"
OWNER, REPO = "HOMESERVERSLTD", "arcadia"
PROJECT = f"{OWNER}/{REPO}"
RELEASES = f"{API_ROOT}/repos/{OWNER}/{REPO}/releases"
def fail(message):
    print(f"release_publish: {message}", file=sys.stderr); raise SystemExit(1)
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

def verify_existing(release, token, binary_name, sidecar_name, digest, sidecar):
    assets = expected_assets(release, binary_name, sidecar_name)
    binary = download(assets[binary_name], token, binary_name)
    if hashlib.sha256(binary).hexdigest() != digest: fail(f"downloaded {binary_name} has a conflicting digest")
    if download(assets[sidecar_name], token, sidecar_name) != sidecar: fail(f"downloaded {sidecar_name} has conflicting contents")

def verify_fresh(release, token, binary_name, sidecar_name, digest, sidecar):
    assets = expected_assets(release, binary_name, sidecar_name)
    if hashlib.sha256(download(assets[binary_name], token, binary_name)).hexdigest() != digest: fail(f"downloaded {binary_name} has a conflicting digest")
    if download(assets[sidecar_name], token, sidecar_name) != sidecar: fail(f"downloaded {sidecar_name} has conflicting contents")
def main():
    token = os.environ.get("FORGEJO_TOKEN", "")
    if not token: fail("FORGEJO_TOKEN is required")
    sha = os.environ.get("CI_COMMIT_SHA", "")
    if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha): fail("CI_COMMIT_SHA must be exactly 40 lowercase hexadecimal characters")
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
    tag_url = f"{RELEASES}/tags/{urllib.parse.quote(sha, safe='')}"; status, raw = request("GET", tag_url, token)
    if status == 200:
        verify_existing(decode(raw, "existing release"), token, binary_name, sidecar_name, digest, sidecar)
        print(json.dumps({"status":"noop-identical", "tag":sha, "name":name, "assets":[binary_name, sidecar_name], "sha256":digest, "cargo_version":version}, separators=(",", ":"))); return
    if status != 404: fail(f"GET release tag returned HTTP {status}")
    payload = {"tag_name":sha, "name":name, "target_commitish":sha, "draft":False, "prerelease":False}; status, raw = request("POST", RELEASES, token, payload)
    if status == 409:
        status, raw = request("GET", tag_url, token)
        if status != 200: fail(f"release collision reread returned HTTP {status}")
        verify_existing(decode(raw, "existing release"), token, binary_name, sidecar_name, digest, sidecar)
        print(json.dumps({"status":"noop-identical", "tag":sha, "name":name, "assets":[binary_name, sidecar_name], "sha256":digest, "cargo_version":version}, separators=(",", ":"))); return
    if status not in (200, 201): fail(f"release creation returned HTTP {status}")
    release = decode(raw, "release creation"); release_id = release.get("id")
    if not isinstance(release_id, int): fail("created release has no numeric id")
    if assets_of(release): fail("new release unexpectedly contains assets")
    upload_url = f"{RELEASES}/{release_id}/assets"
    for asset_name, content, content_type in ((binary_name, binary, "application/octet-stream"), (sidecar_name, sidecar, "text/plain; charset=utf-8")):
        url = f"{upload_url}?{urllib.parse.urlencode({'name':asset_name})}"; status, _ = request("POST", url, token, content, content_type=content_type)
        if status not in (200, 201): fail(f"upload of {asset_name} returned HTTP {status}")
    status, raw = request("GET", tag_url, token)
    if status != 200: fail(f"reread of release returned HTTP {status}")
    verify_fresh(decode(raw, "release reread"), token, binary_name, sidecar_name, digest, sidecar)
    print(json.dumps({"status":"published", "tag":sha, "name":name, "assets":[binary_name, sidecar_name], "sha256":digest, "cargo_version":version}, separators=(",", ":")))
if __name__ == "__main__": main()
