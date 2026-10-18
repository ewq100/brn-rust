# Retirement plan review response

18 October 2026. Full independent [round1 review](plan-review-round1.txt) returned CHANGES_REQUIRED at source HEAD 9e1a2589. This is a plan correction only; implementation and hosted CI remain pending.

1. **Required archive argument:** accepted. Plan C now assigns every maintained generator invocation an explicit fetched archive path, using the existing build script's Cargo cache convention. The generator retains its pinned SHA verification; stock-lock fetch precedes generation, adapted-lock fetch follows it, and builds remain locked/offline.
2. **Cold preview prerequisite:** accepted. Plan C now explicitly puts stock-lock fetch before script-test discovery in both documentation and preview workflows. The earlier preview test step cannot rely on the later helper fetch or the pruned root graph. Offline workflow assertions and broken-order/argument negative controls are assigned.

Product choices and removal/retention boundaries are unchanged. The active skill diagnostic owns only its separate test file; no retirement implementation or compiled-input edit occurred here. A fresh scoped plan review must confirm these corrections before retirement implementation starts. Full feature actual-diff/deletion review remains a later independent gate.
