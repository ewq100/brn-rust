"""One subscription-backed judgment of exact frozen bytes, with no retries."""
import hashlib
import json
from pathlib import Path
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def validate(receipt, packet_bytes, instruction_bytes, timeout_seconds):
    """Validate transport evidence before consuming any semantic verdict."""
    if (receipt.get('status') != 'completed'
            or receipt.get('packet_file_sha256') != digest(packet_bytes)
            or receipt.get('instructions_sha256') != digest(instruction_bytes)
            or receipt.get('timeout_seconds') != timeout_seconds
            or receipt.get('provider') != 'Codex' or receipt.get('model') != 'gpt-6.1-sol'
            or receipt.get('effort') != 'high' or receipt.get('tools') != []):
        raise ValueError('Judge receipt does not match frozen inputs/settings')
    budget = receipt.get('model_call_budget') or {}
    requests = receipt.get('wire_requests')
    if (budget.get('limit') != 1 or budget.get('started') != 1
            or budget.get('refused') is not False or not isinstance(requests, list)
            or len(requests) != 1 or requests[0].get('model') != 'gpt-6.1-sol'
            or requests[0].get('effort') != 'high'):
        raise ValueError('Judge transport identity or single-request budget mismatch')
    result = receipt.get('result') or {}
    usage = result.get('usage') or {}
    if (result.get('returned_model') != 'gpt-6.1-sol'
            or not isinstance(result.get('text'), str) or not result['text'].strip()
            or any(type(usage.get(k)) is not int or usage[k] < 0 for k in ('input', 'output'))):
        raise ValueError('Judge result or usage is incomplete')
    return result


def run(binary, auth, packet, instructions, output, *, binary_sha256, timeout_seconds=180):
    """Persist failed attempts too; the caller owns experiment HTTP admission."""
    if type(timeout_seconds) is not int or not 1 <= timeout_seconds <= 600:
        raise ValueError('Judge timeout must be 1..600 seconds')
    binary, auth, packet, instructions, output = map(Path, (binary, auth, packet, instructions, output))
    if not all(p.is_absolute() for p in (binary, auth, packet, instructions, output)):
        raise ValueError('Judge paths must be absolute')
    before = digest(binary.read_bytes())
    if before != binary_sha256:
        raise ValueError('Judge binary differs from frozen runtime')
    packet_bytes, instruction_bytes = packet.read_bytes(), instructions.read_bytes()
    envelope = {'format': 1, 'judge_binary_sha256': before,
                'packet_file_sha256': digest(packet_bytes),
                'instructions_sha256': digest(instruction_bytes),
                'timeout_seconds': timeout_seconds, 'qualified': False}
    # Refuse accidental overwrite before dispatch, including an earlier failed attempt.
    with output.open('x') as handle:
        json.dump(envelope, handle)
        handle.flush()
        try:
            completed = subprocess.run([str(binary), '--live', str(auth), str(packet), str(instructions),
                                        str(timeout_seconds)], capture_output=True, timeout=timeout_seconds + 15)
            receipt = json.loads(completed.stdout)
            envelope['receipt'] = receipt
            validate(receipt, packet_bytes, instruction_bytes, timeout_seconds)
            if (completed.returncode != 0 or digest(binary.read_bytes()) != before
                    or packet.read_bytes() != packet_bytes or instructions.read_bytes() != instruction_bytes):
                raise ValueError('Judge runtime or input bytes changed during dispatch')
            envelope['qualified'] = True
        except (ValueError, OSError, subprocess.TimeoutExpired) as error:
            # Never retain stderr or raw provider bodies/credential-bearing paths.
            envelope['failure_kind'] = type(error).__name__
        handle.seek(0)
        handle.truncate()
        json.dump(envelope, handle, indent=2)
        handle.write('\n')
    return envelope
