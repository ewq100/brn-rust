"""POSIX session deadline for pinned BRN (helpers change groups, not sessions)."""
import contextlib
import fcntl
import datetime
import json
import os
from pathlib import Path
import signal
import subprocess
import threading
import time
import uuid

ORIGINAL_POPEN = subprocess.Popen


def session_members(sid, runnable_only=False):
    # PID-only inventory; never expose argv, environment, or account identifiers.
    proc = ORIGINAL_POPEN(['/bin/ps', '-axo', 'pid=,stat='], stdout=subprocess.PIPE,
                          stderr=subprocess.DEVNULL, text=True, start_new_session=True)
    try:
        output, _ = proc.communicate(timeout=2)
    except BaseException:
        proc.kill()
        proc.wait()
        raise
    if proc.returncode:
        raise RuntimeError('Process inventory unavailable')
    members = set()
    for line in output.splitlines():
        item, state = line.split()
        if runnable_only and state.startswith('Z'):
            continue
        pid = int(item)
        try:
            if os.getsid(pid) == sid:
                members.add(pid)
        except ProcessLookupError:
            pass
    return members


def stop_session(sid):
    if sid == os.getsid(0):
        raise RuntimeError('Refuse to stop launcher session')
    # Freeze owned session members before killing them, including helper groups.
    # Recheck session identity immediately before every signal.
    for _ in range(3):
        members = session_members(sid)
        for pid in members:
            try:
                if os.getsid(pid) == sid:
                    os.kill(pid, signal.SIGSTOP)
            except ProcessLookupError:
                pass
        if session_members(sid) <= members:
            break
    for pid in session_members(sid):
        try:
            if os.getsid(pid) == sid:
                os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    for _ in range(3):
        if not session_members(sid, runnable_only=True):
            return
        time.sleep(.02)
    raise RuntimeError('Owned session still contains live processes')


class SessionRegistry:
    """Shared only with nested offline supervisors; serialized spawn/stop admission."""
    def __init__(self, receipt):
        inherited = os.environ.get('AGENTBENCH_PILOT_SESSIONS')
        self.owner = inherited is None
        parent = os.environ.get('AGENTBENCH_PILOT_SCOPE', '') if inherited else ''
        self.scope = parent + '.' + uuid.uuid4().hex
        self.path = Path(inherited) if inherited else Path(str(receipt)+'.sessions.jsonl')
        if self.owner:
            with open(self.path,'x') as f:
                os.fchmod(f.fileno(),0o600); f.flush(); os.fsync(f.fileno())
            fd=os.open(self.path.parent,os.O_RDONLY|getattr(os,'O_DIRECTORY',0))
            try: os.fsync(fd)
            finally: os.close(fd)

    def active(self, rows):
        active={}
        for row in rows:
            if row['event']=='opened': active[row['sid']]=row['scope']
            elif row['event']=='closed': active.pop(row['sid'],None)
        return {sid for sid,scope in active.items() if scope == self.scope or scope.startswith(self.scope+'.')}

    def known_prefix(self):
        rows=[]
        with open(self.path,encoding='utf-8') as f:
            for line in f:
                try: rows.append(json.loads(line))
                except ValueError: break
        return self.active(rows)

    @contextlib.contextmanager
    def locked(self):
        with open(self.path,'a+',encoding='utf-8') as f:
            fcntl.flock(f,fcntl.LOCK_EX)
            f.seek(0)
            raw=f.read()
            if raw and not raw.endswith('\n'):
                raise RuntimeError('Incomplete session ledger')
            rows=[json.loads(x) for x in raw.splitlines()]
            def append(row):
                f.write(json.dumps(row)+'\n'); f.flush(); os.fsync(f.fileno())
            yield rows,append


class Guard:
    def __init__(self, receipt, seconds, identity):
        self.seconds = seconds
        self.dispatch_seconds = seconds-30 if seconds >= 60 else seconds * .6
        self.lock = threading.RLock()
        self.sessions = set()
        self.closed = False
        self.sequence = 0
        self.before_launch = lambda: None
        self.started = time.monotonic()
        self.stream = open(receipt, 'x', encoding='utf-8')
        os.fchmod(self.stream.fileno(), 0o600)
        self.record('claimed', identity=identity, deadline_seconds=seconds)
        self.registry = SessionRegistry(receipt)
        directory = os.open(Path(receipt).parent, os.O_RDONLY | getattr(os, 'O_DIRECTORY', 0))
        try:
            os.fsync(directory)
        finally:
            os.close(directory)

    def record(self, event, **fields):
        self.stream.write(json.dumps({'event': event, 'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                                      'elapsed_seconds': time.monotonic()-self.started, **fields})+'\n')
        self.stream.flush()
        os.fsync(self.stream.fileno())

    def completed(self, pid):
        with self.lock:
            if pid in self.sessions:
                # Retire session on completion, killing any orphan helper in it.
                stop_session(pid)
                self.sessions.remove(pid)
                with self.registry.locked() as (_,append):
                    append({'event':'closed','sid':pid})
                self.record('session_closed', pid=pid)

    def install(self):
        guard = self
        class TrackedPopen(ORIGINAL_POPEN):
            def __init__(self, args, **kwargs):
                with guard.lock:
                    if guard.closed or time.monotonic()-guard.started >= guard.dispatch_seconds:
                        raise RuntimeError('Experiment deadline expired; launch refused')
                    if guard.sessions:
                        raise RuntimeError('Unexpected concurrent child; sequential pilot only')
                    guard.before_launch()
                    kwargs['start_new_session'] = True
                    guard.sequence += 1
                    guard.record('launch_intent', launch=guard.sequence)
                    with guard.registry.locked() as (rows,append):
                        if any(row['event']=='stop' and (guard.registry.scope == row['scope'] or guard.registry.scope.startswith(row['scope']+'.')) for row in rows):
                            raise RuntimeError('Outer supervisor stopped; launch refused')
                        super().__init__(args, **kwargs)
                        guard.sessions.add(self.pid)
                        try:
                            append({'event':'opened','sid':self.pid,'scope':guard.registry.scope})
                        except BaseException:
                            stop_session(self.pid)
                            raise
                    guard.record('launched', launch=guard.sequence, pid=self.pid)
            def poll(self):
                result = super().poll()
                if result is not None:
                    guard.completed(self.pid)
                return result
            def wait(self, timeout=None):
                result = super().wait(timeout)
                guard.completed(self.pid)
                return result
        subprocess.Popen = TrackedPopen
        os.environ['AGENTBENCH_PILOT_SESSIONS'] = str(self.registry.path)
        os.environ['AGENTBENCH_PILOT_SCOPE'] = self.registry.scope
        for sig in (signal.SIGINT, signal.SIGTERM):
            signal.signal(sig, lambda signum, frame: self.abort('interrupted', 128+signum))
        self.hard_timer = threading.Timer(max(0, self.seconds-(time.monotonic()-self.started)),
                                          os._exit, args=(124,))
        self.hard_timer.daemon = True
        self.hard_timer.start()
        # Test inventory before any application process can launch.
        session_members(os.getsid(0))
        self.timer = threading.Timer(max(0, self.dispatch_seconds-(time.monotonic()-self.started)),
                                     self.abort, args=('deadline', 124))
        self.timer.daemon = True
        self.timer.start()

    def stop_children(self):
        errors = []
        known = set(self.sessions)
        try:
            with self.registry.locked() as (rows,append):
                known.update(self.registry.active(rows))
                append({'event':'stop','scope':self.registry.scope})
                for sid in list(known):
                    try:
                        stop_session(sid)
                        known.discard(sid)
                        append({'event':'closed','sid':sid})
                    except BaseException as error:
                        errors.append(type(error).__name__)
        except BaseException as error:
            errors.append(type(error).__name__)
            # Corrupt/missing accounting never suppresses known-process cleanup.
            try: known.update(self.registry.known_prefix())
            except BaseException as nested: errors.append(type(nested).__name__)
        for sid in known:
            try: stop_session(sid)
            except BaseException as error:
                errors.append(type(error).__name__)
                try: os.killpg(sid,signal.SIGKILL)
                except OSError: pass
        return errors

    def abort(self, reason, status):
        with self.lock:
            if self.closed:
                return
        try:
            with self.lock:
                self.closed = True
                errors = self.stop_children()
                self.record('terminal', status=reason, exit_code=status, cleanup_errors=errors,
                            remote_outcome='uncertain for in-flight calls; no automatic replay')
        finally:
            os._exit(status)

    def finish(self, status):
        with self.lock:
            self.closed = True
            self.timer.cancel()
            errors = self.stop_children()
            final_status = 1 if errors else status
            self.record('terminal', status='completed' if final_status == 0 else 'failed',
                        exit_code=final_status, cleanup_errors=errors)
            self.stream.close()
            self.hard_timer.cancel()
            return final_status
