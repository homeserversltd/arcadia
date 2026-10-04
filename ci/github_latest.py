#!/usr/bin/env python3
"""Publish Arcadia's Forgejo release as the mirrored GitHub ``latest`` release."""

import hashlib
import json
import os
import re
import subprocess
import sys
import time
import typing
import urllib.error
import urllib.parse
import urllib.request
from urllib.parse import urljoin, urlsplit

FORGEJO_ROOT = "https://git.home.arpa/api/v1"
FORGEJO_HOST = "git.home.arpa"
FORGEJO_GIT_REMOTE = "https://git.home.arpa/HOMESERVERSLTD/arcadia.git"
GITHUB_ROOT = "https://api.github.com"
GITHUB_HOST = "api.github.com"
GITHUB_UPLOAD_HOST = "uploads.github.com"
OWNER = "HOMESERVERSLTD"
REPO = "arcadia"
PROJECT = f"{OWNER}/{REPO}"
BINARY_NAME = "arcadia-x86_64"
SIDECAR_NAME = f"{BINARY_NAME}.sha256"
FLAG_NAME = "release.flag"
LATEST_REF = "refs/tags/latest"
LATEST_TAG = "latest"
EXPECTED_ASSETS = (BINARY_NAME, SIDECAR_NAME, FLAG_NAME)
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(rb"^([0-9a-f]{64})  arcadia-x86_64\n$")
REDIRECT_STATUSES = {301, 302, 303, 307, 308}
MIRROR_WAIT_ATTEMPTS = 37
MIRROR_WAIT_SECONDS = 5
PLAN_MODE = False


class PublishError(Exception):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def redact(value):
    text = str(value)
    for key in ("FORGEJO_TOKEN", "GITHUB_TOKEN"):
        secret = os.environ.get(key, "")
        if secret:
            text = text.replace(secret, "[REDACTED]")
    return text


def fail(message) -> typing.NoReturn:
    raise PublishError(redact(message))


def emit(payload):
    print(redact(json.dumps(payload, sort_keys=True, separators=(",", ":"))))


def require_sha(value):
    if not isinstance(value, str) or not SHA_RE.fullmatch(value):
        fail("CI_COMMIT_SHA must be exactly 40 lowercase hexadecimal characters")
    return value


def safe_url(url):
    parts = urlsplit(url)
    if parts.scheme != "https" or not parts.hostname or parts.username or parts.password:
        fail("refusing a non-HTTPS or credential-bearing request URL")
    return parts


def http_request(method, url, token="", auth_hosts=(), body=None, content_type=None, accept=None):
    method = method.upper()
    if PLAN_MODE and method != "GET":
        fail("--plan blocked a non-GET HTTP request")
    current_url = url
    for redirect_count in range(6):
        parts = safe_url(current_url)
        headers = {"User-Agent": "arcadia-woodpecker-github-latest", "Accept": accept or "application/json"}
        if token and parts.hostname.lower() in auth_hosts:
            headers["Authorization"] = f"token {token}" if parts.hostname.lower() == FORGEJO_HOST else f"Bearer {token}"
        if content_type:
            headers["Content-Type"] = content_type
        request = urllib.request.Request(current_url, data=body, headers=headers, method=method)
        opener = urllib.request.build_opener(NoRedirect())
        try:
            with opener.open(request, timeout=45) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as exc:
            if method == "GET" and exc.code in REDIRECT_STATUSES:
                location = exc.headers.get("Location") if exc.headers else None
                exc.close()
                if location and redirect_count < 5:
                    current_url = urljoin(current_url, location)
                    safe_url(current_url)
                    continue
            status = exc.code
            exc.close()
            return status, b""
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            reason = getattr(exc, "reason", exc)
            fail(f"{method} HTTPS request failed ({type(reason).__name__})")
    fail("HTTP redirect limit exceeded")


def api_json(method, url, token, auth_hosts, payload=None, accept="application/vnd.github+json"):
    body = None if payload is None else json.dumps(payload, separators=(",", ":")).encode("utf-8")
    status, raw = http_request(
        method,
        url,
        token=token,
        auth_hosts=auth_hosts,
        body=body,
        content_type="application/json" if body is not None else None,
        accept=accept,
    )
    if status < 200 or status >= 300:
        return status, None
    if not raw:
        return status, None
    try:
        return status, json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"{method} API response was invalid JSON ({type(exc).__name__})")


def forgejo_api(method, path, token, payload=None, accept="application/json"):
    return api_json(
        method,
        f"{FORGEJO_ROOT}{path}",
        token,
        {FORGEJO_HOST},
        payload=payload,
        accept=accept,
    )


def github_api(method, path, token, payload=None, accept="application/vnd.github+json"):
    return api_json(
        method,
        f"{GITHUB_ROOT}{path}",
        token,
        {GITHUB_HOST, GITHUB_UPLOAD_HOST},
        payload=payload,
        accept=accept,
    )


def download_asset(url, token, auth_hosts):
    status, raw = http_request(
        "GET",
        url,
        token=token,
        auth_hosts=auth_hosts,
        accept="application/octet-stream",
    )
    if status != 200:
        fail(f"release asset download returned HTTP {status}")
    return raw


def parse_assets(release, description):
    if not isinstance(release, dict) or not isinstance(release.get("assets"), list):
        fail(f"{description} has no valid asset list")
    result = {}
    for asset in release["assets"]:
        if not isinstance(asset, dict):
            fail(f"{description} contains a malformed asset")
        name = asset.get("name")
        asset_id = asset.get("id")
        if not isinstance(name, str) or not name or name in result:
            fail(f"{description} contains an invalid or duplicate asset name")
        if not isinstance(asset_id, int) or isinstance(asset_id, bool) or asset_id <= 0:
            fail(f"{description} contains an invalid asset id")
        result[name] = asset
    return result


def forgejo_asset_bytes(release, token, asset):
    tag = release.get("tag_name")
    name = asset.get("name")
    download_url = asset.get("browser_download_url")
    if not isinstance(tag, str) or not re.fullmatch(r"sha-[0-9a-f]{40}", tag):
        fail("Forgejo release has no valid SHA tag for asset download")
    if name not in EXPECTED_ASSETS:
        fail("Forgejo release asset name is not an expected Arcadia asset")
    if not isinstance(download_url, str):
        fail(f"Forgejo {name} asset has no browser_download_url")
    parts = urlsplit(download_url)
    expected_path = f"/{OWNER}/{REPO}/releases/download/{tag}/{name}"
    if (parts.scheme != "https" or parts.netloc.lower() != FORGEJO_HOST
            or parts.path != expected_path or parts.query or parts.fragment):
        fail(f"Forgejo {name} browser_download_url is outside its exact expected host and release path")
    return download_asset(download_url, token, {FORGEJO_HOST})


def github_asset_bytes(asset, token):
    asset_id = asset["id"]
    path = f"/repos/{OWNER}/{REPO}/releases/assets/{asset_id}"
    return download_asset(f"{GITHUB_ROOT}{path}", token, {GITHUB_HOST})


def validate_source_assets(release, sha, forgejo_token):
    expected_tag = f"sha-{sha}"
    if release.get("tag_name") != expected_tag or release.get("target_commitish") != sha:
        fail("Forgejo release identity does not match the requested source SHA")
    assets = parse_assets(release, "Forgejo release")
    missing = [name for name in EXPECTED_ASSETS if name not in assets]
    if missing:
        fail("Forgejo release is missing required assets: " + ", ".join(missing))
    contents = {
        name: forgejo_asset_bytes(release, forgejo_token, assets[name])
        for name in EXPECTED_ASSETS
    }
    digest = hashlib.sha256(contents[BINARY_NAME]).hexdigest()
    sidecar = DIGEST_RE.fullmatch(contents[SIDECAR_NAME])
    if sidecar is None or sidecar.group(1).decode("ascii") != digest:
        fail("Forgejo binary and checksum sidecar do not match exact expected syntax")
    try:
        flag = json.loads(contents[FLAG_NAME])
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"Forgejo release.flag is invalid JSON ({type(exc).__name__})")
    if not isinstance(flag, dict):
        fail("Forgejo release.flag is not a JSON object")
    if flag.get("schema") != "estate.release-flag.v1":
        fail("Forgejo release.flag schema is not estate.release-flag.v1")
    if flag.get("component") != REPO:
        fail("Forgejo release.flag component does not identify Arcadia")
    if flag.get("source_sha") != sha:
        fail("Forgejo release.flag source_sha does not match the requested SHA")
    if flag.get("sha256") != digest:
        fail("Forgejo release.flag sha256 does not match the Forgejo binary")
    return contents, digest


def ref_record(payload, expected_ref, description):
    refs = payload if isinstance(payload, list) else [payload]
    matching = [item for item in refs if isinstance(item, dict) and item.get("ref") == expected_ref]
    if len(matching) != 1:
        fail(f"{description} did not return exactly one {expected_ref} ref")
    obj = matching[0].get("object")
    if not isinstance(obj, dict) or not isinstance(obj.get("sha"), str):
        fail(f"{description} returned a malformed ref object")
    if not SHA_RE.fullmatch(obj["sha"]):
        fail(f"{description} returned a malformed object SHA")
    kind = obj.get("type")
    if kind not in ("commit", "tag"):
        fail(f"{description} returned an unsupported ref object type")
    return obj["sha"], kind


def resolve_tag_target(provider, object_sha, object_type, token):
    current_sha = object_sha
    current_type = object_type
    for _ in range(6):
        if current_type == "commit":
            return current_sha
        if current_type != "tag":
            fail("latest tag does not resolve to a commit")
        if provider == "forgejo":
            status, tag_obj = forgejo_api("GET", f"/repos/{OWNER}/{REPO}/git/tags/{current_sha}", token)
        else:
            status, tag_obj = github_api("GET", f"/repos/{OWNER}/{REPO}/git/tags/{current_sha}", token)
        if status != 200 or not isinstance(tag_obj, dict):
            fail(f"{provider} annotated tag read returned HTTP {status}")
        obj = tag_obj.get("object")
        if not isinstance(obj, dict) or not isinstance(obj.get("sha"), str):
            fail(f"{provider} annotated tag object is malformed")
        current_sha = obj["sha"]
        current_type = obj.get("type")
        if not SHA_RE.fullmatch(current_sha):
            fail(f"{provider} annotated tag contains a malformed object SHA")
    fail(f"{provider} annotated tag nesting exceeds the safety limit")


def forgejo_latest_ref(token):
    path = f"/repos/{OWNER}/{REPO}/git/refs/tags/latest"
    status, payload = forgejo_api("GET", path, token)
    if status == 404:
        return None
    if status != 200:
        fail(f"Forgejo latest-tag GET returned HTTP {status}")
    object_sha, object_type = ref_record(payload, LATEST_REF, "Forgejo latest-tag GET")
    target_sha = resolve_tag_target("forgejo", object_sha, object_type, token)
    return {"object_sha": object_sha, "object_type": object_type, "target_sha": target_sha}


def forgejo_main_sha(token):
    path = f"/repos/{OWNER}/{REPO}/branches/main"
    status, branch = forgejo_api("GET", path, token)
    if status != 200 or not isinstance(branch, dict):
        fail(f"Forgejo main branch lookup returned HTTP {status}")
    commit = branch.get("commit")
    sha = commit.get("id", commit.get("sha")) if isinstance(commit, dict) else None
    if not isinstance(sha, str) or not SHA_RE.fullmatch(sha):
        fail("Forgejo main branch response has no exact commit SHA")
    return sha


def github_latest_ref(token):
    path = f"/repos/{OWNER}/{REPO}/git/ref/tags/latest"
    status, payload = github_api("GET", path, token)
    if status == 404:
        return None
    if status != 200:
        fail(f"GitHub latest-tag GET returned HTTP {status}")
    object_sha, object_type = ref_record(payload, LATEST_REF, "GitHub latest-tag GET")
    target_sha = resolve_tag_target("github", object_sha, object_type, token)
    return {"object_sha": object_sha, "object_type": object_type, "target_sha": target_sha}


def github_repository(token):
    status, payload = github_api("GET", f"/repos/{PROJECT}", token)
    if status == 404:
        return status, None
    if status != 200:
        return status, None
    if not isinstance(payload, dict) or str(payload.get("full_name", "")).casefold() != PROJECT.casefold():
        fail("GitHub repository readback did not identify the fixed Arcadia target")
    return status, payload


def github_latest_release(token):
    path = f"/repos/{OWNER}/{REPO}/releases/tags/{urllib.parse.quote(LATEST_TAG, safe='')}"
    status, payload = github_api("GET", path, token)
    if status == 404:
        return status, None
    if status != 200:
        return status, None
    if not isinstance(payload, dict) or payload.get("tag_name") != LATEST_TAG:
        fail("GitHub latest-release readback did not identify the exact latest tag")
    release_id = payload.get("id")
    if not isinstance(release_id, int) or isinstance(release_id, bool) or release_id <= 0:
        fail("GitHub latest release has no valid numeric id")
    parse_assets(payload, "GitHub latest release")
    return status, payload


def github_404_state(token):
    return "missing" if token else "not visible or missing"


def github_release_assets_match(release, expected, token):
    assets = parse_assets(release, "GitHub latest release")
    if set(assets) != set(EXPECTED_ASSETS):
        return False
    for name in EXPECTED_ASSETS:
        actual = github_asset_bytes(assets[name], token)
        if name == BINARY_NAME:
            if hashlib.sha256(actual).digest() != hashlib.sha256(expected[name]).digest():
                return False
        elif actual != expected[name]:
            return False
    return True


def plan_observation(token, kind):
    try:
        if kind == "ref":
            value = github_latest_ref(token)
            return {"state": github_404_state(token) if value is None else "present",
                    "target_sha": value["target_sha"] if value else None}
        status, value = github_latest_release(token)
        if status == 404:
            return {"state": github_404_state(token), "http_status": status,
                    "release_id": None, "asset_names": []}
        if status != 200 or value is None:
            return {"state": "unavailable", "http_status": status, "release_id": None, "asset_names": []}
        return {"state": "present", "http_status": status, "release_id": value.get("id"),
                "asset_names": sorted(asset["name"] for asset in value.get("assets", []))}
    except PublishError as exc:
        return {"state": "unavailable", "error": redact(exc)}


def observe_github(token):
    repo_status, repo = github_repository(token)
    ref_info = plan_observation(token, "ref")
    release_info = plan_observation(token, "release")
    if repo_status == 200:
        repo_info = {"state": "present", "full_name": repo["full_name"]}
    elif repo_status == 404:
        repo_info = {"state": github_404_state(token), "http_status": 404}
    else:
        repo_info = {"state": "unavailable", "http_status": repo_status}
    return repo_info, ref_info, release_info


def current_forgejo_ref_plan(token):
    try:
        value = forgejo_latest_ref(token)
        return {"state": "missing" if value is None else "present",
                "target_sha": value["target_sha"] if value else None}
    except PublishError as exc:
        return {"state": "unavailable", "error": redact(exc)}


def run_plan(sha, forgejo_token, github_token):
    source_path = f"/repos/{OWNER}/{REPO}/releases/tags/{urllib.parse.quote(f'sha-{sha}', safe='')}"
    source_status, source_release = forgejo_api("GET", source_path, forgejo_token)
    if source_status not in (200, 404):
        fail(f"Forgejo source-release GET returned HTTP {source_status}")
    contents = None
    binary_digest = None
    if source_status == 200:
        if not isinstance(source_release, dict):
            fail("Forgejo source-release response is not an object")
        contents, binary_digest = validate_source_assets(source_release, sha, forgejo_token)
    repo_info, github_ref_info, github_release_info = observe_github(github_token)
    forgejo_ref_info = current_forgejo_ref_plan(forgejo_token)
    assets = []
    if contents is not None:
        assets = [
            {"name": name, "sha256": hashlib.sha256(contents[name]).hexdigest()}
            for name in EXPECTED_ASSETS
        ]
    if source_status == 404:
        ref_action = "no-op-no-source-release"
        actions = []
        status = "noop-forgejo-release-absent"
    else:
        current_sha = forgejo_ref_info.get("target_sha") if forgejo_ref_info.get("state") == "present" else None
        if forgejo_ref_info.get("state") == "unavailable":
            ref_action = "unknown"
            actions = ["could not inspect Forgejo latest ref; no mutation is planned"]
            status = "plan-incomplete"
        else:
            ref_action = "no-op" if current_sha == sha else ("force-update" if current_sha else "create")
            actions = [
                f"set Forgejo {LATEST_REF} to {sha} (forceful update when it exists)",
                f"POST /repos/{OWNER}/{REPO}/push_mirrors-sync to synchronize Forgejo push mirrors",
                f"wait for mirrored GitHub {LATEST_REF} to resolve to {sha}",
                "create or update only the GitHub latest release with the three listed assets",
            ]
            status = "plan-only"
    emit({
        "status": status,
        "mutations": False,
        "source_sha": sha,
        "forgejo_source_release": {"state": "present" if source_status == 200 else "missing",
                                   "tag": f"sha-{sha}"},
        "github_repository": repo_info,
        "github_token_configured": bool(github_token),
        "github_latest_ref": github_ref_info,
        "github_latest_release": github_release_info,
        "github_publish_target_found": repo_info.get("state") == "present",
        "github_publish_blocker": None if repo_info.get("state") == "present" else repo_info,
        "forgejo_latest_ref": {"state": forgejo_ref_info.get("state"),
                               "current_target_sha": forgejo_ref_info.get("target_sha"),
                               "read_error": forgejo_ref_info.get("error")},
        "forgejo_latest_ref_move": {"ref": LATEST_REF, "action": ref_action,
                                    "target_sha": sha, "force": True},
        "planned_upload_assets": assets,
        "binary_sha256": binary_digest,
        "expected_actions": actions,
    })


def ensure_github_target(token):
    status, repo = github_repository(token)
    if status == 404:
        fail(f"GitHub target repository {PROJECT} is missing (HTTP 404); refusing publication")
    if status != 200:
        fail(f"GitHub target repository GET returned HTTP {status}; refusing publication")
    return repo


def ensure_current_forgejo_main(token, sha, context):
    if forgejo_main_sha(token) != sha:
        fail(f"CI_COMMIT_SHA is no longer Forgejo main; stopped before {context}")


def force_push_forgejo_latest_tag(token, sha):
    if PLAN_MODE:
        fail("--plan blocked the Forgejo tag-only git push")
    git_env = os.environ.copy()
    git_env.pop("FORGEJO_TOKEN", None)
    git_env["GIT_CONFIG_COUNT"] = "1"
    git_env["GIT_CONFIG_KEY_0"] = "http.https://git.home.arpa/.extraheader"
    git_env["GIT_CONFIG_VALUE_0"] = f"Authorization: token {token}"
    git_env["GIT_TERMINAL_PROMPT"] = "0"
    try:
        result = subprocess.run(
            ["git", "push", "--force", "--no-follow-tags",
             FORGEJO_GIT_REMOTE, f"{sha}:{LATEST_REF}"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
            env=git_env,
        )
    except subprocess.TimeoutExpired:
        fail("Forgejo tag-only git push timed out")
    except OSError as exc:
        fail(f"Forgejo tag-only git push could not run ({type(exc).__name__})")
    if result.returncode != 0:
        # Captured Git output may contain remote echoes; never relay it.
        fail(f"Forgejo tag-only git push failed (exit code {result.returncode})")


def move_forgejo_latest(token, sha):
    current = forgejo_latest_ref(token)
    if current is not None and current["target_sha"] == sha:
        ensure_current_forgejo_main(token, sha, "GitHub release publication")
        return "noop"
    ensure_current_forgejo_main(token, sha, "Forgejo latest-tag update")
    force_push_forgejo_latest_tag(token, sha)
    readback = forgejo_latest_ref(token)
    if readback is None or readback["target_sha"] != sha:
        fail("Forgejo latest-tag ref did not read back at the requested SHA")
    return "force-pushed"


def sync_forgejo_push_mirrors(token):
    path = f"/repos/{OWNER}/{REPO}/push_mirrors-sync"
    status, _ = forgejo_api("POST", path, token)
    print(f"Forgejo push-mirror sync returned HTTP {status}", file=sys.stderr)
    if status < 200 or status >= 300:
        fail(f"Forgejo push-mirror sync returned HTTP {status}")


def wait_for_mirrored_github_ref(token, sha):
    last = None
    for attempt in range(MIRROR_WAIT_ATTEMPTS):
        last = github_latest_ref(token)
        if last is not None and last["target_sha"] == sha:
            return last
        if attempt + 1 < MIRROR_WAIT_ATTEMPTS:
            time.sleep(MIRROR_WAIT_SECONDS)
    observed = last["target_sha"] if last is not None else "missing"
    fail(f"GitHub mirrored latest tag did not reach {sha} after bounded wait (observed {observed})")


def verify_release_metadata(release, sha):
    if release.get("tag_name") != LATEST_TAG:
        fail("GitHub release tag_name is not latest")
    if (release.get("name") != "Arcadia latest" or release.get("draft") is not False
            or release.get("prerelease") is not False):
        return False
    if release.get("body") != f"Arcadia release mirrored from Forgejo source sha-{sha}.":
        return False
    target_commitish = release.get("target_commitish")
    if target_commitish is not None:
        if not isinstance(target_commitish, str) or not target_commitish:
            return False
        # GitHub treats target_commitish as a tag-creation hint once the tag exists.
        # Check it when it is an actual SHA; the mirrored Git ref is authoritative
        # for resolving symbolic branch names after tag creation.
        if (re.fullmatch(r"[0-9a-fA-F]{40}", target_commitish)
                and target_commitish.lower() != sha):
            return False
    return True


def verify_github_assets(release, expected, sha, github_token):
    if release.get("tag_name") != LATEST_TAG:
        fail("GitHub release readback is not attached to latest")
    assets = parse_assets(release, "GitHub latest release readback")
    if set(assets) != set(EXPECTED_ASSETS):
        fail("GitHub latest release asset set is not the exact three Forgejo assets")
    downloaded = {name: github_asset_bytes(assets[name], github_token) for name in EXPECTED_ASSETS}
    actual_digest = hashlib.sha256(downloaded[BINARY_NAME]).hexdigest()
    sidecar = DIGEST_RE.fullmatch(downloaded[SIDECAR_NAME])
    expected_digest = hashlib.sha256(expected[BINARY_NAME]).hexdigest()
    if downloaded[BINARY_NAME] != expected[BINARY_NAME] or actual_digest != expected_digest:
        fail("GitHub uploaded binary does not match the Forgejo binary digest")
    if sidecar is None or sidecar.group(1).decode("ascii") != actual_digest:
        fail("GitHub binary digest does not match the exact GitHub sidecar")
    if downloaded[SIDECAR_NAME] != expected[SIDECAR_NAME]:
        fail("GitHub sidecar bytes do not match the Forgejo sidecar")
    if downloaded[FLAG_NAME] != expected[FLAG_NAME]:
        fail("GitHub release.flag bytes do not exactly match Forgejo")
    try:
        flag = json.loads(downloaded[FLAG_NAME])
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"GitHub release.flag is invalid JSON ({type(exc).__name__})")
    if (not isinstance(flag, dict) or flag.get("schema") != "estate.release-flag.v1"
            or flag.get("component") != REPO or flag.get("source_sha") != sha
            or flag.get("sha256") != actual_digest):
        fail("GitHub release.flag does not identify the mirrored source SHA and binary digest")
    if not verify_release_metadata(release, sha):
        fail("GitHub latest release metadata did not read back as Arcadia latest")
    return {"asset_names": list(EXPECTED_ASSETS), "binary_sha256": actual_digest,
            "release_flag_source_sha": flag["source_sha"]}


def remove_github_release_assets(release, token, forgejo_token, sha):
    assets = parse_assets(release, "existing GitHub latest release")
    for name, asset in assets.items():
        ensure_current_forgejo_main(forgejo_token, sha, f"deletion of GitHub asset {name}")
        status, _ = github_api("DELETE", f"/repos/{OWNER}/{REPO}/releases/assets/{asset['id']}", token)
        if status not in (204, 404):
            fail(f"GitHub delete of old latest asset {name} returned HTTP {status}")


def release_payload(sha):
    return {
        "tag_name": LATEST_TAG,
        "target_commitish": sha,
        "name": "Arcadia latest",
        "body": f"Arcadia release mirrored from Forgejo source sha-{sha}.",
        "draft": False,
        "prerelease": False,
        "make_latest": "true",
    }


def create_or_update_release(token, release, sha, forgejo_token):
    if release is None:
        request_payload = release_payload(sha)
        ensure_current_forgejo_main(forgejo_token, sha, "GitHub latest-release creation")
        status, payload = github_api("POST", f"/repos/{OWNER}/{REPO}/releases", token, request_payload)
        if status in (200, 201) and isinstance(payload, dict):
            return payload
        if status in (409, 422):
            reread_status, reread = github_latest_release(token)
            if reread_status == 200 and reread is not None:
                remove_github_release_assets(reread, token, forgejo_token, sha)
                return create_or_update_release(token, reread, sha, forgejo_token)
        fail(f"GitHub latest release create returned HTTP {status}")
    release_id = release["id"]
    request_payload = release_payload(sha)
    ensure_current_forgejo_main(forgejo_token, sha, "GitHub latest-release update")
    status, payload = github_api("PATCH", f"/repos/{OWNER}/{REPO}/releases/{release_id}", token,
                                 request_payload)
    if status not in (200, 201) or not isinstance(payload, dict):
        fail(f"GitHub latest release update returned HTTP {status}")
    if payload.get("id") != release_id or payload.get("tag_name") != LATEST_TAG:
        fail("GitHub latest release update readback changed release identity")
    return payload


def upload_github_assets(release, expected, token, forgejo_token, sha):
    release_id = release.get("id")
    if not isinstance(release_id, int) or isinstance(release_id, bool) or release_id <= 0:
        fail("GitHub latest release has no valid numeric id for asset upload")
    upload_root = f"https://{GITHUB_UPLOAD_HOST}/repos/{OWNER}/{REPO}/releases/{release_id}/assets"
    content_types = {
        BINARY_NAME: "application/octet-stream",
        SIDECAR_NAME: "text/plain; charset=utf-8",
        FLAG_NAME: "application/json",
    }
    for name in EXPECTED_ASSETS:
        url = upload_root + "?" + urllib.parse.urlencode({"name": name})
        ensure_current_forgejo_main(forgejo_token, sha, f"upload of GitHub asset {name}")
        status, _ = http_request(
            "POST", url, token=token, auth_hosts={GITHUB_UPLOAD_HOST},
            body=expected[name], content_type=content_types[name],
            accept="application/vnd.github+json",
        )
        if status not in (200, 201):
            fail(f"GitHub upload of {name} returned HTTP {status}")


def run_publish(sha, forgejo_token, github_token):
    source_path = f"/repos/{OWNER}/{REPO}/releases/tags/{urllib.parse.quote(f'sha-{sha}', safe='')}"
    source_status, source_release = forgejo_api("GET", source_path, forgejo_token)
    if source_status == 404:
        emit({"status": "noop-forgejo-release-absent", "mutations": False,
              "source_sha": sha, "forgejo_tag": f"sha-{sha}"})
        return
    if source_status != 200 or not isinstance(source_release, dict):
        fail(f"Forgejo source-release GET returned HTTP {source_status}")
    expected, binary_digest = validate_source_assets(source_release, sha, forgejo_token)

    # Prove the fixed GitHub target exists before moving the Forgejo mirror source ref.
    ensure_github_target(github_token)
    current_release_status, current_release = github_latest_release(github_token)
    if current_release_status not in (200, 404):
        fail(f"GitHub latest-release GET returned HTTP {current_release_status}")

    forgejo_action = move_forgejo_latest(forgejo_token, sha)
    sync_forgejo_push_mirrors(forgejo_token)
    wait_for_mirrored_github_ref(github_token, sha)
    ensure_current_forgejo_main(forgejo_token, sha, "GitHub latest-release inspection")

    if current_release is not None:
        assets_match = github_release_assets_match(current_release, expected, github_token)
        metadata_match = verify_release_metadata(current_release, sha)
        if assets_match and metadata_match:
            verified = verify_github_assets(current_release, expected, sha, github_token)
            final_ref = github_latest_ref(github_token)
            if final_ref is None or final_ref["target_sha"] != sha:
                fail("GitHub latest tag no longer resolves to the admitted SHA")
            ensure_current_forgejo_main(forgejo_token, sha, "GitHub latest-release verification")
            emit({"status": "noop-identical", "mutations": forgejo_action not in ("noop", "concurrent-noop"),
                  "source_sha": sha,
                  "forgejo_latest_ref_action": forgejo_action,
                  "github_latest_ref_sha": final_ref["target_sha"],
                  "release_id": current_release["id"], "assets": verified["asset_names"],
                  "binary_sha256": binary_digest})
            return
        remove_github_release_assets(current_release, github_token, forgejo_token, sha)
        release = create_or_update_release(github_token, current_release, sha, forgejo_token)
    else:
        release = create_or_update_release(github_token, None, sha, forgejo_token)

    # New/updated latest release has no old assets left before these exact Forgejo copies.
    upload_github_assets(release, expected, github_token, forgejo_token, sha)
    release_id = release.get("id")
    read_status, readback = github_api("GET", f"/repos/{OWNER}/{REPO}/releases/{release_id}", github_token)
    if read_status != 200 or not isinstance(readback, dict):
        fail(f"GitHub latest release readback returned HTTP {read_status}")
    verified = verify_github_assets(readback, expected, sha, github_token)
    final_ref = github_latest_ref(github_token)
    if final_ref is None or final_ref["target_sha"] != sha:
        fail("GitHub latest tag moved away from the admitted SHA during publication")
    emit({"status": "published", "mutations": True, "source_sha": sha,
          "forgejo_latest_ref_action": forgejo_action,
          "github_latest_ref_sha": final_ref["target_sha"],
          "release_id": readback["id"], "assets": verified["asset_names"],
          "binary_sha256": verified["binary_sha256"],
          "release_flag_source_sha": verified["release_flag_source_sha"]})


def main():
    global PLAN_MODE
    args = sys.argv[1:]
    if args not in ([], ["--plan"]):
        fail("usage: github_latest.py [--plan]")
    PLAN_MODE = args == ["--plan"]
    sha = require_sha(os.environ.get("CI_COMMIT_SHA", ""))
    forgejo_token = os.environ.get("FORGEJO_TOKEN", "")
    github_token = os.environ.get("GITHUB_TOKEN", "")
    if not forgejo_token:
        fail("FORGEJO_TOKEN is required for Forgejo release readback")
    if not PLAN_MODE:
        if not github_token:
            fail("GITHUB_TOKEN is required for GitHub publication")
        if os.environ.get("CI_COMMIT_BRANCH") != "main" or os.environ.get("CI_PIPELINE_EVENT") != "push":
            fail("publication is restricted to main push pipelines")
    if PLAN_MODE:
        run_plan(sha, forgejo_token, github_token)
    else:
        run_publish(sha, forgejo_token, github_token)


if __name__ == "__main__":
    try:
        main()
    except PublishError as exc:
        print(f"github-latest-arcadia: {redact(exc)}", file=sys.stderr)
        raise SystemExit(1)
    except Exception as exc:
        print(f"github-latest-arcadia: unexpected {type(exc).__name__}: {redact(exc)}", file=sys.stderr)
        raise SystemExit(1)
