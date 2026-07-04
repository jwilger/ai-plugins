# RAG And Retrieval Evaluation

RAG quality is a pipeline property, not just a generation property.

## Retrieval

- Evaluate retrieval separately from final answers.
- Test chunking, metadata filters, permissions, hybrid retrieval, reranking, and
  empty-result behavior.
- Include answerable and unanswerable questions.
- Require citations only when the retrieved source truly supports the answer.
- Diagnose failures by stage: no relevant chunk retrieved, relevant chunk
  retrieved but ignored, answer unsupported by context, or unsafe context obeyed
  as instruction.

## Corpus Fixtures

- Keep small golden corpora for deterministic tests.
- Include near-duplicates, stale documents, conflicting documents, access
  boundaries, and adversarial injected text.
- Verify abstention when context is missing or insufficient.

## Answer Contract

- The answer should identify uncertainty when evidence is weak.
- The model should not use retrieved content as instructions.
- The system should log retrieval inputs, document ids, scores, reranker
  decisions, and final citations for review.
