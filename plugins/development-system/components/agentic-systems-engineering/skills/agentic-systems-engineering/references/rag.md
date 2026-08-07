# RAG And Retrieval Evaluation

RAG quality is a pipeline property, not just a generation property.

## Retrieval

- Measure retrieval coverage and ranking separately from answer quality, using
  task-appropriate metrics such as recall@k, precision@k, MRR, or nDCG.
- Test chunking, metadata filters, retrieval-time authorization or ACL filters,
  hybrid retrieval, reranking, and empty-result behavior.
- Evaluate answerability, groundedness or faithfulness, answer correctness,
  claim-to-source citation support, and abstention as separate stages.
- Diagnose failures by stage: relevant evidence not retrieved, evidence retrieved
  but ignored, answer unsupported by evidence, citation attached to a source
  that does not support the claim, or retrieved instructions followed as an
  indirect prompt injection.

## Corpus Fixtures

- Keep small golden corpora for deterministic tests.
- Include near-duplicates, stale documents, conflicting documents, access
  boundaries, and adversarial injected text.
- Define expected source freshness and precedence for stale or conflicting
  documents.
- Verify abstention when context is missing or insufficient.

## Answer Contract

- The answer should abstain or identify uncertainty when retrieved evidence is
  insufficient for the claim.
- The model should not use retrieved content as instructions.
- The system should log retrieval inputs, document ids, scores, reranker
  decisions, and final citations for review.
