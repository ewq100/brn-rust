"""Optional Inspect 0.3.286 frozen-output adapter; never calls generate.

Input is already reviewed, anonymous checkpoint text and numeric grades. This
adapter is an inspection/regrading view, not an alternative candidate executor.
"""
import hashlib
from inspect_ai import Task
from inspect_ai.dataset import MemoryDataset, Sample
from inspect_ai.model import ModelOutput
from inspect_ai.scorer import Score, accuracy, scorer
from inspect_ai.solver import solver


@solver
def frozen_output():
    async def solve(state, generate):
        state.output = ModelOutput.from_content('frozen-brn-output', state.metadata['frozen_output'])
        state.completed = True
        return state
    return solve


@scorer(metrics=[accuracy()])
def reviewed_grade():
    async def score(state, target):
        return Score(value=state.metadata['reviewed_score'] / 4,
                     explanation=state.metadata['evidence'])
    return score


def frozen_task(rows):
    samples = []
    seen = set()
    for row in rows:
        if row['id'] in seen or type(row['score']) is not int or not 0 <= row['score'] <= 4:
            raise ValueError('Duplicate ID or invalid reviewed score')
        seen.add(row['id'])
        if not row['evidence'] or not isinstance(row['output'], str):
            raise ValueError('Frozen output and review evidence required')
        samples.append(Sample(id=row['id'], input='Inspect the frozen BRN evidence.',
                              metadata={'frozen_output': row['output'], 'reviewed_score': row['score'],
                                        'evidence': row['evidence'],
                                        'output_sha256': hashlib.sha256(row['output'].encode()).hexdigest()}))
    return Task(dataset=MemoryDataset(samples), solver=frozen_output(), scorer=reviewed_grade(), epochs=1)
