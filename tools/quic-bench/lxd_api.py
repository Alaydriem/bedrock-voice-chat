"""LXD REST calls shared by the quic-bench drivers. Standard library only."""
import json
import ssl
import time
import urllib.request


class LxdClient:
    def __init__(self, host, certs_dir):
        self.host = host
        self.ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        self.ctx.check_hostname = False
        self.ctx.verify_mode = ssl.CERT_NONE
        self.ctx.load_cert_chain(f"{certs_dir}/lxd-client.crt", f"{certs_dir}/lxd-client.key")

    def req(self, method, path, body=None, raw=None, headers=None, timeout=300, raw_response=False):
        data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
        r = urllib.request.Request(self.host + path, data=data, method=method)
        if body is not None and raw is None:
            r.add_header("Content-Type", "application/json")
        for k, v in (headers or {}).items():
            r.add_header(k, v)
        with urllib.request.urlopen(r, context=self.ctx, timeout=timeout) as resp:
            payload = resp.read()
        if raw_response:
            return payload
        try:
            return json.loads(payload)
        except Exception:
            return payload

    def wait(self, resp, timeout=900):
        """Block on an async LXD operation and return its metadata."""
        op = resp.get("operation") if isinstance(resp, dict) else None
        if not op:
            return resp
        out = self.req("GET", f"{op}/wait?timeout={timeout}", timeout=timeout + 30)
        md = out.get("metadata", {})
        if md.get("status") == "Failure":
            raise RuntimeError(f"operation failed: {md.get('err')}")
        return md


class LxdInstance:
    def __init__(self, client, name):
        self.client = client
        self.name = name

    def create(self, config, image):
        self.client.wait(
            self.client.req(
                "POST",
                "/1.0/instances",
                {
                    "name": self.name,
                    "type": "container",
                    "ephemeral": False,
                    "config": config,
                    "source": {
                        "type": "image",
                        "protocol": "simplestreams",
                        "server": "https://cloud-images.ubuntu.com/releases",
                        "alias": image,
                    },
                },
            )
        )

    def start(self):
        self.client.wait(
            self.client.req("PUT", f"/1.0/instances/{self.name}/state", {"action": "start", "timeout": 60})
        )

    def destroy(self):
        for method, path, body in (
            ("PUT", f"/1.0/instances/{self.name}/state", {"action": "stop", "timeout": 60, "force": True}),
            ("DELETE", f"/1.0/instances/{self.name}", None),
        ):
            try:
                self.client.wait(self.client.req(method, path, body))
            except Exception:
                pass

    def push(self, local, remote, mode=0o755, strip_cr=False):
        with open(local, "rb") as f:
            data = f.read()
        # Shell scripts are checked out with CRLF on Windows, and bash in the
        # container treats the carriage return as part of the command.
        if strip_cr:
            data = data.replace(b"\r\n", b"\n")
        self.push_bytes(data, remote, mode)
        print(f"  pushed {remote} to {self.name} ({len(data) / 1e6:.1f} MB)", flush=True)

    def push_bytes(self, data, remote, mode=0o644):
        self.client.req(
            "POST",
            f"/1.0/instances/{self.name}/files?path={remote}",
            raw=data,
            headers={
                "X-LXD-type": "file",
                "X-LXD-mode": oct(mode)[2:],
                "X-LXD-uid": "0",
                "X-LXD-gid": "0",
                "Content-Type": "application/octet-stream",
            },
            timeout=600,
        )

    def pull(self, remote):
        return self.client.req(
            "GET", f"/1.0/instances/{self.name}/files?path={remote}", raw_response=True
        )

    def run(self, cmd, env, timeout=3600, quiet=False):
        # LXD nests exec results one level deeper than other operations: the return
        # code and the output paths are under metadata.metadata, not metadata.
        md = self.client.wait(
            self.client.req(
                "POST",
                f"/1.0/instances/{self.name}/exec",
                {
                    "command": cmd,
                    "wait-for-websocket": False,
                    "record-output": True,
                    "interactive": False,
                    "environment": env,
                },
            ),
            timeout=timeout,
        )
        inner = md.get("metadata") or {}
        rc = inner.get("return", -1)
        out = ""
        for stream in ("1", "2"):
            p = (inner.get("output") or {}).get(stream)
            if p:
                b = self.client.req("GET", p, timeout=120, raw_response=True)
                out += b.decode("utf-8", "replace")
        if not quiet:
            print(out.rstrip(), flush=True)
        return rc, out

    def ipv4(self):
        state = self.client.req("GET", f"/1.0/instances/{self.name}/state")
        for address in state["metadata"]["network"]["eth0"]["addresses"]:
            if address["family"] == "inet":
                return address["address"]
        raise RuntimeError(f"{self.name} has no IPv4 address on eth0")

    def wait_for_network(self, env):
        for _ in range(60):
            time.sleep(2)
            rc, _ = self.run(["sh", "-c", "getent hosts archive.ubuntu.com >/dev/null 2>&1"], env, quiet=True)
            if rc == 0:
                return True
        return False

    def apt_install(self, packages, env):
        joined = " ".join(packages)
        self.run(
            ["sh", "-c", f"apt-get update -qq && apt-get install -y -qq {joined} >/dev/null 2>&1"],
            env,
        )
