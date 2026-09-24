#!/usr/bin/env python3
import hashlib, json, os, re, sys, time, tomllib, urllib.error, urllib.parse, urllib.request
from datetime import datetime, timedelta, timezone
API_ROOT = "https://git.home.arpa/api/v1"
OWNER, REPO = "HOMESERVERSLTD", "arcadia"
PROJECT = f"{OWNER}/{REPO}"
RELEASES = f"{API_ROOT}/repos/{OWNER}/{REPO}/releases"
RELEASE_FLAG = "release.flag"
RELEASE_RETENTION = 20
def fail(message):
    print(f"release_publish: {message}", file=sys.stderr); raise SystemExit(1)
def release_tag(sha):
    return f"sha-{sha}"
def request(method, url, token, body=None, content_type=None, accept=None):
    headers = {"User-Agent": "arcadia-woodpecker-release"}
    if token: headers["Authorization"] = f"token {token}"
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

def retention_releases(token):
    releases = []
    release_ids = set()
    first_page_ids = []
    page = 1
    while True:
        url = f"{RELEASES}?{urllib.parse.urlencode({'limit':50, 'page':page})}"
        status, raw = request("GET", url, token)
        if status != 200: raise RuntimeError(f"GET releases page {page} returned HTTP {status}")
        batch = decode(raw, f"releases page {page}")
        if not isinstance(batch, list): raise RuntimeError(f"releases page {page} is not a JSON array")
        batch_ids = []
        for release in batch:
            if not isinstance(release, dict):
                raise RuntimeError(f"releases page {page} contains a non-object entry")
            release_id = release.get("id")
            if not isinstance(release_id, int) or isinstance(release_id, bool):
                raise RuntimeError(f"releases page {page} contains a malformed release id")
            if release_id in release_ids:
                raise RuntimeError(f"release listing contains duplicate id {release_id}")
            release_ids.add(release_id)
            batch_ids.append(release_id)
        if page == 1: first_page_ids = batch_ids
        releases.extend(batch)
        if len(batch) < 50: break
        page += 1
    status, raw = request("GET", f"{RELEASES}?{urllib.parse.urlencode({'limit':50, 'page':1})}", token)
    if status != 200: raise RuntimeError(f"GET releases page 1 consistency read returned HTTP {status}")
    first_page = decode(raw, "releases page 1 consistency read")
    if not isinstance(first_page, list):
        raise RuntimeError("releases page 1 consistency read is not a JSON array")
    reread_ids = []
    for release in first_page:
        if not isinstance(release, dict):
            raise RuntimeError("releases page 1 consistency read contains a non-object entry")
        release_id = release.get("id")
        if not isinstance(release_id, int) or isinstance(release_id, bool):
            raise RuntimeError("releases page 1 consistency read contains a malformed release id")
        reread_ids.append(release_id)
    if reread_ids != first_page_ids:
        raise RuntimeError("release listing changed during pagination (first-page ids differ)")
    return releases

def strict_release_tag(release):
    tag = release.get("tag_name")
    target = release.get("target_commitish")
    if not isinstance(tag, str) or not re.fullmatch(r"sha-[0-9a-f]{40}", tag): return None
    sha = tag[4:]
    if target != sha: return None
    return tag

def retention_plan(token, current_sha):
    releases = retention_releases(token)
    eligible = []
    for release in releases:
        if not isinstance(release, dict): raise RuntimeError("release listing contains a non-object entry")
        if release.get("draft") is True: continue
        tag = strict_release_tag(release)
        if tag is None: continue
        release_id = release.get("id")
        if not isinstance(release_id, int) or isinstance(release_id, bool):
            raise RuntimeError(f"eligible release {tag} has no numeric id")
        created = release.get("created_at")
        if not isinstance(created, str) or not created:
            raise RuntimeError(f"eligible release {tag} has no created_at")
        try: created_key = datetime.fromisoformat(created.replace("Z", "+00:00"))
        except ValueError: raise RuntimeError(f"eligible release {tag} has invalid created_at")
        if created_key.tzinfo is None: raise RuntimeError(f"eligible release {tag} created_at is not timezone-aware")
        created_key = created_key.astimezone(timezone.utc)
        eligible.append({"id":release_id, "tag":tag, "created_at":created, "sort_time":created_key})
    eligible.sort(key=lambda release: (release["sort_time"], release["id"]), reverse=True)
    keep = eligible[:RELEASE_RETENTION]
    older = eligible[RELEASE_RETENTION:]
    current_tag = release_tag(current_sha) if current_sha else None
    protected = next((r for r in eligible if r["tag"] == current_tag), None)
    if current_tag and protected is None:
        raise RuntimeError(f"current published release {current_tag} is absent from eligible releases")
    boundary_ties = []
    if len(eligible) > RELEASE_RETENTION:
        boundary_time = keep[-1]["sort_time"]
        boundary_ties = [r["id"] for r in eligible if r["sort_time"] == boundary_time]
    return {"all_count":len(releases), "eligible_count":len(eligible), "kept":keep,
            "older":older, "current":protected, "boundary_tie_ids":boundary_ties}

def require_tag_ref_absent(token, tag):
    url = f"{API_ROOT}/repos/{OWNER}/{REPO}/git/refs/tags/{urllib.parse.quote(tag, safe='')}"
    status, _ = request("GET", url, token)
    if status == 404: return True
    if status == 200: return False
    raise RuntimeError(f"GET tag ref {tag} returned HTTP {status}")

def retention_execute(token, current_sha):
    receipt = {"status":"failed", "keep_limit":RELEASE_RETENTION, "kept_count":0,
               "deleted_ids":[], "deleted_tags":[], "remaining_tag_refs":[],
               "protected_current_id":None}
    try:
        plan = retention_plan(token, current_sha)
        current = plan["current"]
        receipt.update({"all_count":plan["all_count"], "eligible_count":plan["eligible_count"],
                       "kept_count":len(plan["kept"]), "kept_ids":[r["id"] for r in plan["kept"]],
                       "protected_current_id":current["id"] if current else None,
                       "boundary_tie_ids":plan["boundary_tie_ids"]})
        if current is None: raise RuntimeError("cannot mutate retention without a current published SHA")
        # The current release must still carry the verified success flag before pruning.
        current_url = f"{RELEASES}/{current['id']}"
        status, raw = request("GET", current_url, token)
        if status != 200: raise RuntimeError(f"current release readback returned HTTP {status}")
        live = decode(raw, "current release")
        verify_release_identity(live, current_sha, release_tag(current_sha))
        assets = assets_of(live)
        if RELEASE_FLAG not in assets: raise RuntimeError("current release has no release.flag; retention refused")
        flag_obj = decode(download(assets[RELEASE_FLAG], token, RELEASE_FLAG), "current release.flag")
        if not isinstance(flag_obj, dict) or flag_obj.get("schema") != "estate.release-flag.v1" or flag_obj.get("component") != REPO or flag_obj.get("source_sha") != current_sha:
            raise RuntimeError("current release.flag does not identify the current SHA")
        for old in plan["older"]:
            if old["id"] == current["id"]: raise RuntimeError("refusing to delete current published release")
            url = f"{RELEASES}/{old['id']}"
            status, _ = request("DELETE", url, token)
            if status not in (200, 204, 404): raise RuntimeError(f"DELETE release {old['id']} returned HTTP {status}")
            check, _ = request("GET", url, token)
            if check != 404: raise RuntimeError(f"release {old['id']} still exists after DELETE (HTTP {check})")
            receipt["deleted_ids"].append(old["id"])
            tag_url = f"{API_ROOT}/repos/{OWNER}/{REPO}/tags/{urllib.parse.quote(old['tag'], safe='')}"
            tag_status, _ = request("DELETE", tag_url, token)
            if tag_status not in (204, 404): raise RuntimeError(f"DELETE tag {old['tag']} returned HTTP {tag_status}")
            if require_tag_ref_absent(token, old["tag"]):
                receipt["deleted_tags"].append(old["tag"])
            else:
                receipt["remaining_tag_refs"].append(old["tag"])
        receipt["status"] = "complete"
    except SystemExit as exc:
        # Shared fail() uses SystemExit for fatal API/JSON checks. Retention must
        # still emit its machine-readable partial-failure receipt.
        receipt["error"] = f"fail-closed check exited with status {exc.code}"
    except Exception as exc:
        receipt["error"] = str(exc)
    print(json.dumps(receipt, separators=(",", ":")))
    if receipt["status"] != "complete": raise SystemExit(1)

def plan_retention(token, current_sha):
    plan = retention_plan(token, current_sha)
    print(json.dumps({"status":"plan-only", "mutations":False, "keep_limit":RELEASE_RETENTION,
                      "all_count":plan["all_count"], "eligible_count":plan["eligible_count"],
                      "kept_ids":[r["id"] for r in plan["kept"]],
                      "older":[{"id":r["id"], "tag":r["tag"]} for r in plan["older"]],
                      "protected_current_id":plan["current"]["id"] if plan["current"] else None,
                      "boundary_tie_ids":plan["boundary_tie_ids"]}, separators=(",", ":")))

def main():
    args = sys.argv[1:]
    plan_only = args == ["--plan-retention"]
    retain = args == ["--retain"]
    if args and not (plan_only or retain or args == ["--flag"]): fail("usage: release_publish.py [--flag|--plan-retention|--retain]")
    if plan_only or retain:
        token = os.environ.get("FORGEJO_TOKEN", "")
        if retain and not token: fail("FORGEJO_TOKEN is required")
        sha = os.environ.get("CI_COMMIT_SHA", "")
        if plan_only:
            if sha and (len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha)):
                fail("CI_COMMIT_SHA must be exactly 40 lowercase hexadecimal characters when provided")
            plan_retention(token, sha); return
        if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha): fail("CI_COMMIT_SHA must be exactly 40 lowercase hexadecimal characters")
        if os.environ.get("CI_COMMIT_BRANCH") != "main" or os.environ.get("CI_PIPELINE_EVENT") != "push":
            fail("retention mutation is restricted to main push pipelines")
        retention_execute(token, sha); return
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
