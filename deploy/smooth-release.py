#!/usr/bin/env python3
"""Single-host, two-port release switch. Run as an authorized deploy operator."""
import argparse
import fcntl
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import time
import urllib.request
from pathlib import Path


def run(*args):
    subprocess.run(args, check=True)


def save(path, value):
    tmp = path.with_suffix(path.suffix + '.tmp')
    tmp.write_text(json.dumps(value, indent=2) + '\n')
    os.replace(tmp, path)


def digest(path):
    h = hashlib.sha256()
    with open(path, 'rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def verify_files(release, binary):
    manifest = release / 'FILE_SHA256SUMS'
    if not manifest.is_file():
        raise RuntimeError('package has no internal file checksums')
    checked = set()
    for line in manifest.read_text().splitlines():
        value, marker, name = line.partition('  ')
        path = Path(name)
        if len(value) != 64 or not marker or path.is_absolute() or '..' in path.parts:
            raise RuntimeError('invalid internal checksum manifest')
        file = release / path
        if not file.is_file() or digest(file) != value:
            raise RuntimeError('internal checksum mismatch: %s' % name)
        checked.add(file.resolve())
    actual = {p.resolve() for p in release.rglob('*') if p.is_file() and p != manifest}
    if checked != actual:
        raise RuntimeError('internal checksum file list mismatch')
    executable = release / binary
    with executable.open('rb') as stream:
        header = stream.read(20)
    if header[:5] != b'\x7fELF\x02' or int.from_bytes(header[18:20], 'little') != 62:
        raise RuntimeError('binary is not Linux x86_64 ELF')


def probe(port, expected):
    url = 'http://127.0.0.1:%d/internal/ready' % port
    with urllib.request.urlopen(url, timeout=3) as response:
        result = json.load(response)
    if result.get('status') != 'ok' or result.get('version') != expected:
        raise RuntimeError('candidate readiness/version mismatch: %r' % result)


def snapshot_workers(pid_file):
    master = int(Path(pid_file).read_text().strip())
    result = []
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            data = (entry / 'stat').read_text().split()
            cmd = (entry / 'cmdline').read_bytes()
            if int(data[3]) == master and b'nginx: worker process' in cmd:
                result.append([int(entry.name), data[21]])
        except (OSError, ValueError, IndexError):
            continue
    if not result:
        raise RuntimeError('cannot establish Nginx worker drain boundary')
    return result


def alive(worker):
    try:
        data = Path('/proc/%s/stat' % worker[0]).read_text().split()
        return data[21] == worker[1]
    except (OSError, IndexError):
        return False


def snippet(cfg, port):
    assets = str(Path(cfg['assets_dir']).resolve())
    if ' ' in assets or '\n' in assets:
        raise RuntimeError('asset directory cannot contain whitespace')
    return '''# Managed by smooth-release.py; include once inside the existing server block.
location = /__release_port { if ($remote_addr !~ "^(127\\.0\\.0\\.1|::1)$") { return 404; } default_type text/plain; return 200 "%d"; }
location ^~ /internal/ { return 404; }
location ^~ /admin/assets/ { alias %s/; add_header Cache-Control "public, max-age=31536000, immutable"; }
location / {
    proxy_pass http://127.0.0.1:%d;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_next_upstream off;
    proxy_read_timeout 120s;
    proxy_send_timeout 120s;
    add_header Cache-Control "no-store" always;
    add_header X-RuoYi-Release-Port "%d" always;
}
''' % (port, assets, port, port)


def nginx(cfg, port):
    target = Path(cfg['nginx_include'])
    old = target.read_bytes() if target.exists() else None
    tmp = target.with_suffix(target.suffix + '.tmp')
    tmp.write_text(snippet(cfg, port))
    os.replace(tmp, target)
    try:
        run(cfg['nginx_bin'], '-t', '-c', cfg['nginx_config'])
        run(cfg['nginx_bin'], '-s', 'reload', '-c', cfg['nginx_config'])
    except Exception:
        if old is None:
            target.unlink(missing_ok=True)
        else:
            target.write_bytes(old)
        run(cfg['nginx_bin'], '-t', '-c', cfg['nginx_config'])
        run(cfg['nginx_bin'], '-s', 'reload', '-c', cfg['nginx_config'])
        raise


def verify_routing(cfg, port):
    origin = cfg.get('probe_origin', 'http://127.0.0.1').rstrip('/')
    request = urllib.request.Request(origin + '/__release_port', headers={'Host': cfg['host']})
    with urllib.request.urlopen(request, timeout=5) as response:
        if response.read().decode().strip() != str(port):
            raise RuntimeError('Nginx is not routing to candidate')
    for path in cfg['smoke_paths']:
        request = urllib.request.Request(origin + path, headers={'Host': cfg['host']})
        with urllib.request.urlopen(request, timeout=5) as response:
            if response.status >= 400 or response.headers.get('X-RuoYi-Release-Port') != str(port):
                raise RuntimeError('smoke response did not use candidate: %s' % path)


def assets(cfg, release):
    source = release / cfg['web_assets']
    target = Path(cfg['assets_dir'])
    target.mkdir(parents=True, exist_ok=True)
    if not source.is_dir():
        raise RuntimeError('package has no hashed frontend assets')
    for file in source.rglob('*'):
        if file.is_symlink():
            raise RuntimeError('asset symlink rejected')
        if file.is_file():
            dst = target / file.relative_to(source)
            dst.parent.mkdir(parents=True, exist_ok=True)
            if dst.exists():
                if digest(file) != digest(dst):
                    raise RuntimeError('asset path changed content: %s' % dst)
            else:
                shutil.copy2(file, dst)


def unit(cfg, name, release, port):
    command = str(release / cfg['binary'])
    if cfg.get('serve_arg'):
        command += ' ' + cfg['serve_arg']
    env = 'RUOYI_PORT=%d' % port if cfg.get('serve_arg') else 'RUOYI_PORT=%d RUOYI_AUTO_SETUP=false' % port
    text = '''[Unit]
Description=%s release %s
After=network.target
[Service]
Type=simple
User=%s
Group=%s
WorkingDirectory=%s
EnvironmentFile=%s
Environment=%s
ExecStart=%s
Restart=on-failure
RestartSec=3
TimeoutStopSec=90
KillSignal=SIGTERM
SendSIGKILL=no
NoNewPrivileges=true
UMask=0027
[Install]
WantedBy=multi-user.target
''' % (cfg['service_prefix'], name, cfg['user'], cfg['group'], release,
       cfg['env_file'], env, command)
    target = Path(cfg['systemd_dir']) / ('%s-%s.service' % (cfg['service_prefix'], name))
    target.write_text(text)
    run('systemctl', 'daemon-reload')
    return target.stem


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', required=True)
    parser.add_argument('action', choices=['adopt', 'prepare', 'start', 'switch', 'rollback', 'retire', 'status'])
    parser.add_argument('name', nargs='?')
    parser.add_argument('archive', nargs='?')
    parser.add_argument('sha256', nargs='?')
    parser.add_argument('version', nargs='?')
    args = parser.parse_args()
    cfg = json.loads(Path(args.config).read_text())
    root = Path(cfg['state_dir'])
    root.mkdir(parents=True, exist_ok=True)
    with (root / '.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state_file = root / 'state.json'
        pending_file = root / 'pending.json'
        state = json.loads(state_file.read_text()) if state_file.exists() else {'instances': {}, 'active': None, 'previous': None}
        if pending_file.exists():
            pending = json.loads(pending_file.read_text())
            nginx(cfg, pending['old_port'])
            state = pending['state']
            save(state_file, state)
            pending_file.unlink()
        if args.action == 'status':
            print(json.dumps(state, indent=2))
            return
        if args.action in ('adopt', 'prepare', 'start', 'switch', 'retire') and not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9._-]{0,60}', args.name or ''):
            raise RuntimeError('invalid release name')
        if args.action == 'adopt':
            if state['active']:
                raise RuntimeError('already adopted')
            # Existing process remains untouched; operator must confirm its port.
            port = cfg['ports'][0]
            workers = snapshot_workers(cfg['nginx_pid'])
            assets(cfg, Path(args.archive).resolve())
            nginx(cfg, port)
            verify_routing(cfg, port)
            state['instances'][args.name] = {'port': port, 'legacy': True, 'drain': workers}
            state['active'] = args.name
        elif args.action == 'prepare':
            if args.name in state['instances']:
                raise RuntimeError('release already exists')
            if not args.version:
                raise RuntimeError('expected version required')
            archive = Path(args.archive).resolve()
            if digest(archive) != args.sha256:
                raise RuntimeError('archive SHA256 mismatch')
            release = Path(cfg['releases_dir']) / args.name
            if release.exists():
                raise RuntimeError('release path exists')
            release.mkdir(parents=True)
            try:
                with tarfile.open(archive, 'r:gz') as tar:
                    for member in tar.getmembers():
                        p = Path(member.name)
                        if p.is_absolute() or '..' in p.parts or not (member.isfile() or member.isdir()):
                            raise RuntimeError('unsafe archive entry')
                    tar.extractall(release)
                verify_files(release, cfg['binary'])
                upload = release / cfg['upload_path']
                if upload.exists() and (not upload.is_dir() or any(upload.iterdir())):
                    raise RuntimeError('package contains upload data')
                upload.rmdir() if upload.exists() else None
                upload.parent.mkdir(parents=True, exist_ok=True)
                upload.symlink_to(Path(cfg['shared_uploads']).resolve(), target_is_directory=True)
                assets(cfg, release)
            except Exception:
                shutil.rmtree(release)
                raise
            state['instances'][args.name] = {'path': str(release), 'sha256': args.sha256, 'port': None, 'version': args.version}
        elif args.action == 'start':
            item = state['instances'][args.name]
            if item.get('legacy'):
                raise RuntimeError('legacy cannot be started by release tool')
            if not item['port']:
                used = {v['port'] for v in state['instances'].values() if v['port']}
                free = [p for p in cfg['ports'] if p not in used]
                if not free:
                    raise RuntimeError('no free port; drain and retire old release first')
                item['port'] = free[0]
                item['unit'] = unit(cfg, args.name, Path(item['path']), item['port'])
                save(state_file, state)
            run('systemctl', 'start', item['unit'])
            for _ in range(60):
                try:
                    probe(item['port'], item['version'])
                    break
                except Exception:
                    time.sleep(2)
            else:
                raise RuntimeError('candidate did not become ready; old release still active')
        elif args.action in ('switch', 'rollback'):
            name = args.name if args.action == 'switch' else state['previous']
            if not name or name == state['active']:
                raise RuntimeError('no different target')
            item = state['instances'][name]
            if not item['port']:
                raise RuntimeError('target must be running')
            if item.get('legacy'):
                with urllib.request.urlopen('http://127.0.0.1:%d/admin/' % item['port'], timeout=5) as response:
                    if response.status != 200:
                        raise RuntimeError('legacy health check failed')
            else:
                probe(item['port'], item['version'])
            old = state['active']
            workers = snapshot_workers(cfg['nginx_pid'])
            save(pending_file, {'old_port': state['instances'][old]['port'], 'state': json.loads(json.dumps(state)), 'target': name})
            nginx(cfg, item['port'])
            try:
                verify_routing(cfg, item['port'])
            except Exception:
                nginx(cfg, state['instances'][old]['port'])
                raise
            state['instances'][old]['drain'] = workers
            state['previous'], state['active'] = old, name
        elif args.action == 'retire':
            if args.name == state['active']:
                raise RuntimeError('cannot retire active release')
            item = state['instances'][args.name]
            if any(alive(w) for w in item.get('drain', [])):
                raise RuntimeError('old Nginx workers still serving requests')
            if item.get('legacy'):
                raise RuntimeError('legacy requires supervised retirement')
            run('systemctl', 'stop', item['unit'])
            item['port'] = None
        save(state_file, state)
        if args.action in ('switch', 'rollback'):
            pending_file.unlink()


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print('release failed: %s' % error, file=sys.stderr)
        sys.exit(1)
