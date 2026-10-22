# Documentation-only classifier qualification

Requirements: ENG-01/02/04/05. This qualification branch changes only this
Markdown document relative to `quality/regression-gates`. It exercises the
checked-in classifier and aggregate workflow without changing application code.

Expected result: classification, documentation and verification-policy jobs
succeed; Windows, Edge, dependency and Stream Deck jobs are explicitly
unaffected and skipped; Quality gate succeeds. A cancelled, missing or failed
required job is never an acceptable replacement for that result.

The workflow run, tested SHA and actual conclusions must be inspected before
recording this demonstration as passing. This text states the expectation,
not an unrun result. Close the temporary draft PR after retaining its run URL;
it is qualification evidence and does not need to merge into the product.
