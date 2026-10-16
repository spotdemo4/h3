#!/usr/bin/env python3
"""Drives a browser over W3C WebDriver to run index.html and reports the result.

Uses only the standard library so it runs as-is on GitHub's runners with
safaridriver, chromedriver or geckodriver.

Usage: run.py --driver http://127.0.0.1:4444 --browser chrome --page 'http://localhost:8000/index.html?url=...'
Exits 0 if every step passed, 1 otherwise.
"""

import argparse
import json
import sys
import time
import urllib.error
import urllib.request


def capabilities(browser):
    if browser == "chrome":
        return {
            "browserName": "chrome",
            "goog:chromeOptions": {
                "args": ["--headless=new", "--no-sandbox", "--enable-logging=stderr", "--v=0"],
            },
        }
    if browser == "firefox":
        return {
            "browserName": "firefox",
            "acceptInsecureCerts": True,
            "moz:firefoxOptions": {"args": ["-headless"]},
        }
    if browser == "safari":
        return {"browserName": "safari"}
    raise SystemExit(f"unknown browser {browser}")


class Driver:
    def __init__(self, base):
        self.base = base.rstrip("/")

    def call(self, method, path, body=None):
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(
            self.base + path,
            data=data,
            method=method,
            headers={"Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                return json.load(resp).get("value")
        except urllib.error.HTTPError as err:
            raise RuntimeError(f"{method} {path}: {err.code} {err.read().decode(errors='replace')}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", required=True)
    parser.add_argument("--browser", required=True)
    parser.add_argument("--page", required=True)
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--output", help="write the JSON result to this file")
    args = parser.parse_args()

    driver = Driver(args.driver)
    session = driver.call("POST", "/session", {"capabilities": {"alwaysMatch": capabilities(args.browser)}})
    sid = session["sessionId"]
    print(f"browser: {session['capabilities'].get('browserName')} {session['capabilities'].get('browserVersion')}")

    results = None
    printed = 0
    try:
        driver.call("POST", f"/session/{sid}/url", {"url": args.page})
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            try:
                state = driver.call(
                    "POST",
                    f"/session/{sid}/execute/sync",
                    {
                        "script": "return [window.__results || null, document.getElementById('log').textContent]",
                        "args": [],
                    },
                )
            except RuntimeError as err:
                # For example, the browser crashed. Steps logged so far were printed already.
                results = {"ok": False, "steps": [], "error": f"lost the page: {err}"}
                break
            results, log = state
            # Print progress as it happens, so it survives the page being lost.
            lines = log.splitlines()
            for line in lines[printed:]:
                print(f"page: {line}", flush=True)
            printed = len(lines)
            if results:
                break
            time.sleep(0.5)
        if not results:
            results = {"ok": False, "steps": [], "error": "page did not finish"}
    finally:
        try:
            driver.call("DELETE", f"/session/{sid}")
        except Exception as err:  # noqa: BLE001 - best effort cleanup
            print(f"failed to close session: {err}", file=sys.stderr)

    print(json.dumps(results, indent=2))
    if args.output:
        with open(args.output, "w") as f:
            json.dump(results, f, indent=2)

    for step in results.get("steps", []):
        status = "PASS" if step.get("ok") else ("SKIP" if step.get("skipped") else "FAIL")
        print(f"{status} {step['name']}" + (f": {step['error']}" if step.get("error") else ""))
    sys.exit(0 if results.get("ok") else 1)


if __name__ == "__main__":
    main()
