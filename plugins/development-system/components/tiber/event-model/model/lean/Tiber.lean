namespace Tiber

-- EMC generated Lean4 model root.

def modelName := "Tiber"

def modelVersion := "0.1.0"

def modelDigest := "5c751ff9f1c7ad9ad7d030544d0d38cff1a3ea7496ea5d48ea1dabf7f6ce498b"

structure ModelWorkflow where
  workflow : String

def modelWorkflows : List ModelWorkflow := [{ workflow := "manage-work" },{ workflow := "recover-pushed-ci" }]

structure ModelSlice where
  workflow : String
  slice : String

structure ModelSliceModule where
  workflow : String
  slice : String
  formalModule : String

structure ModelScenario where
  workflow : String
  slice : String
  scenarioKind : String
  scenario : String

structure ModelScenarioDefinition where
  workflow : String
  slice : String
  scenarioKind : String
  scenario : String
  given : String
  when : String
  thenStep : String
  readStreams : List String
  writtenStreams : List String
  contractKind : String
  coveredDefinition : String
  errorReferences : List String

inductive ModelDataFlowSourceKind where
  | original
  | modeledTarget
deriving BEq, DecidableEq, Repr

structure ModelDataFlow where
  workflow : String
  slice : String
  datum : String
  sourceKind : ModelDataFlowSourceKind
  source : String
  transformation : String
  target : String
  bitEncoding : String

structure ModelOutcome where
  workflow : String
  slice : String
  outcome : String
  events : List String
  externallyRelevant : Bool

structure ModelCommandError where
  workflow : String
  slice : String
  command : String
  error : String
  scenario : String
  recovery : String

structure ModelCommand where
  workflow : String
  slice : String
  command : String

inductive ModelCommandInputSourceKind where
  | actor
  | session
  | generated
  | externalPayload
  | eventStreamState
  | invocationArgument
deriving BEq, DecidableEq, Repr

structure ModelCommandInput where
  workflow : String
  slice : String
  command : String
  input : String
  sourceKind : ModelCommandInputSourceKind
  sourceDescription : String
  provenanceChain : List String
  eventStreamSourceEvent : String
  eventStreamSourceAttribute : String
  externalPayloadSourceName : String
  externalPayloadSourceField : String
  generatedSourceName : String
  generatedSourceField : String
  sessionSourceName : String
  sessionSourceField : String
  invocationArgumentSourceName : String
  invocationArgumentSourceField : String

structure ModelReadModel where
  workflow : String
  slice : String
  readModel : String

structure ModelReadModelDefinition where
  workflow : String
  slice : String
  readModel : String
  transitive : Bool
  relationshipFields : List String
  transitiveRule : String
  exampleScenarioName : String

structure ModelReadModelField where
  workflow : String
  slice : String
  readModel : String
  field : String
  sourceKind : String
  sourceEvent : String
  sourceAttribute : String
  derivationRule : String
  derivationSourceFields : List String
  absenceEvent : String
  derivationScenarioName : String
  absenceScenarioName : String
  provenance : String

structure ModelView where
  workflow : String
  slice : String
  view : String

structure ModelViewDefinition where
  workflow : String
  slice : String
  view : String
  readModels : List String
  sketchTokens : List String
  localStates : List String
  filters : List String

structure ModelViewControl where
  workflow : String
  slice : String
  view : String
  control : String
  command : String
  input : String
  inputSourceKind : ModelCommandInputSourceKind
  inputSourceDescription : String
  inputSketchToken : String
  inputVisibleToActor : Bool
  inputDecisionField : Bool
  handledErrors : List String
  recoveryBehavior : String
  controlSketchToken : String
  navigationType : String
  navigationTarget : String
  externalWorkflow : String
  externalSystem : String
  handoffContract : String

structure ModelViewField where
  workflow : String
  slice : String
  view : String
  field : String
  sourceKind : String
  sourceReadModel : String
  sourceField : String
  provenance : String
  bitEncoding : String

structure ModelBoardElement where
  workflow : String
  slice : String
  element : String
  kind : String
  lane : String
  declaredName : String
  mainPath : Bool

structure ModelBoardConnection where
  workflow : String
  slice : String
  source : String
  sourceKind : String
  target : String
  targetKind : String

structure ModelAutomation where
  workflow : String
  slice : String
  automation : String

structure ModelAutomationDefinition where
  workflow : String
  slice : String
  automation : String
  trigger : String
  command : String
  handledErrors : List String
  reaction : String

structure ModelTranslation where
  workflow : String
  slice : String
  translation : String

structure ModelTranslationDefinition where
  workflow : String
  slice : String
  translation : String
  externalEvent : String
  payloadContract : String
  command : String

structure ModelExternalPayload where
  workflow : String
  slice : String
  externalPayload : String

structure ModelExternalPayloadField where
  workflow : String
  slice : String
  externalPayload : String
  field : String
  provenance : String
  bitEncoding : String

structure ModelStream where
  workflow : String
  slice : String
  stream : String

structure ModelEvent where
  workflow : String
  slice : String
  event : String
  stream : String

structure ModelEventAttribute where
  workflow : String
  slice : String
  event : String
  attributeName : String
  sourceKind : String
  sourceName : String
  sourceField : String
  generatedSourceKind : String
  provenance : String

def modelSlices : List ModelSlice := [{ workflow := "manage-work", slice := "change-task-board" },{ workflow := "manage-work", slice := "view-task-board" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery" }]

def modelSliceModules : List ModelSliceModule := [{ workflow := "manage-work", slice := "change-task-board", formalModule := "ChangeTaskBoard" },{ workflow := "manage-work", slice := "view-task-board", formalModule := "ViewTaskBoard" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", formalModule := "CoordinateCiRecovery" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", formalModule := "ViewCiRecovery" }]

def modelSliceBelongsToDeclaredWorkflow (slice : ModelSlice) : Bool := modelWorkflows.any (fun workflow => workflow.workflow == slice.workflow)

def modelSliceHasModule (slice : ModelSlice) : Bool := modelSliceModules.any (fun sliceModule => sliceModule.workflow == slice.workflow && sliceModule.slice == slice.slice && sliceModule.formalModule.isEmpty == false)

def modelSliceModuleBelongsToDeclaredSlice (sliceModule : ModelSliceModule) : Bool := sliceModule.formalModule.isEmpty == false && modelSlices.any (fun slice => slice.workflow == sliceModule.workflow && slice.slice == sliceModule.slice)

def modelWorkflowSlicesHaveModules (workflow : ModelWorkflow) : Bool := modelSlices.all (fun slice => slice.workflow != workflow.workflow || modelSliceHasModule slice)

def modelWorkflowHasCompositionStructure (workflow : ModelWorkflow) : Bool := modelWorkflowSlicesHaveModules workflow

def modelScenarios : List ModelScenario := [{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "acceptance", scenario := "First mutation initializes one authoritative branch" },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "acceptance", scenario := "Task lifecycle preserves board invariants" },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "contract", scenario := "Ambiguous publication blocks mutations" },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "contract", scenario := "Confirmed conflict reruns pure command" },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "contract", scenario := "Confirmed publication is the transaction boundary" },{ workflow := "manage-work", slice := "view-task-board", scenarioKind := "acceptance", scenario := "Dashboard derives all task state from events" },{ workflow := "manage-work", slice := "view-task-board", scenarioKind := "contract", scenario := "Task board projector reconstructs exactly once" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", scenarioKind := "acceptance", scenario := "Claim and resolution share repository incident state" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", scenarioKind := "contract", scenario := "Failed mandatory claim blocks downstream work" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", scenarioKind := "contract", scenario := "MCP claim has process stable fallback identity" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", scenarioKind := "contract", scenario := "CI recovery projector reconstructs exactly once" }]

def modelScenarioDefinitions : List ModelScenarioDefinition := [{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "acceptance", scenario := "First mutation initializes one authoritative branch", given := "the tiber branch is absent and legacy task branches may exist", when := "a valid task mutation is submitted", thenStep := "repository metadata and the task change publish atomically on the tiber branch without reading or changing legacy refs", readStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], writtenStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "acceptance", scenario := "Task lifecycle preserves board invariants", given := "an initialized board with ordered backlog tasks and available capacity", when := "task lifecycle details dependencies subtasks acceptance notes claims pull request metadata or validation repairs change", thenStep := "the command preserves existing lifecycle validation capacity and strict priority behavior and publishes semantic events atomically", readStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], writtenStreams := ["tiber:board","tiber:task:<id>"], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "contract", scenario := "Ambiguous publication blocks mutations", given := "publication was attempted and remote membership remains unknowable after same-candidate retries", when := "the append outcome is returned", thenStep := "a pending candidate marker is retained and tiber.publication_indeterminate blocks new mutations until tiber sync resolves it", readStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], writtenStreams := ["tiber:board","tiber:task:<id>"], contractKind := "command", coveredDefinition := "submit-task-command", errorReferences := [] },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "contract", scenario := "Confirmed conflict reruns pure command", given := "another writer advances the authoritative tiber ref without the candidate", when := "remote state conclusively excludes the candidate", thenStep := "the adapter reports VersionConflict after refresh so EventCore may rerun the pure command with stable invocation inputs", readStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], writtenStreams := ["tiber:board","tiber:task:<id>"], contractKind := "command", coveredDefinition := "submit-task-command", errorReferences := ["version-conflict"] },{ workflow := "manage-work", slice := "change-task-board", scenarioKind := "contract", scenario := "Confirmed publication is the transaction boundary", given := "a command has appended to disposable file staging", when := "the candidate Git commit has not been confirmed on the authoritative tiber ref", thenStep := "the command does not succeed and staged append is not authoritative", readStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], writtenStreams := ["tiber:repository","tiber:board","tiber:task:<id>"], contractKind := "command", coveredDefinition := "submit-task-command", errorReferences := [] },{ workflow := "manage-work", slice := "view-task-board", scenarioKind := "acceptance", scenario := "Dashboard derives all task state from events", given := "published task events on the tiber branch", when := "CLI MCP or dashboard task state is requested", thenStep := "the existing response behavior is reconstructed without markdown tickets or order.md", readStreams := [], writtenStreams := [], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "manage-work", slice := "view-task-board", scenarioKind := "contract", scenario := "Task board projector reconstructs exactly once", given := "semantic task events and an optional checkpoint", when := "CLI MCP or dashboard state is projected after restart", thenStep := "the existing board details history order and validation views equal one ordered event fold", readStreams := [], writtenStreams := [], contractKind := "projector", coveredDefinition := "task-board-state", errorReferences := [] },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", scenarioKind := "acceptance", scenario := "Claim and resolution share repository incident state", given := "a failed pushed CI run and no active recovery owner", when := "a participant claims and later resolves the incident", thenStep := "claim ownership and terminal-success evidence publish on tiber:ci-recovery using the tiber branch", readStreams := ["tiber:repository","tiber:ci-recovery"], writtenStreams := ["tiber:repository","tiber:ci-recovery"], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", scenarioKind := "contract", scenario := "Failed mandatory claim blocks downstream work", given := "the workflow requires a shared CI recovery claim", when := "claiming or publishing the claim fails", thenStep := "a repository blocker denies diagnosis edits tests reruns pushes and unrelated work and permits only exact recovery operations", readStreams := ["tiber:repository","tiber:ci-recovery"], writtenStreams := ["tiber:repository","tiber:ci-recovery"], contractKind := "command", coveredDefinition := "submit-recovery-command", errorReferences := ["workflow-blocked"] },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", scenarioKind := "contract", scenario := "MCP claim has process stable fallback identity", given := "TIBER_CLAIM_SESSION CODEX_SESSION_ID and CLAUDE_SESSION_ID are absent", when := "the MCP server performs repeated CI recovery commands", thenStep := "one generated process identity is reused while direct CLI recovery still requires explicit identity", readStreams := ["tiber:ci-recovery"], writtenStreams := ["tiber:ci-recovery"], contractKind := "command", coveredDefinition := "submit-recovery-command", errorReferences := [] },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", scenarioKind := "contract", scenario := "CI recovery projector reconstructs exactly once", given := "semantic recovery events and an optional checkpoint", when := "claim status wait or sync state is projected", thenStep := "incident ownership blocker and terminal evidence equal one ordered event fold", readStreams := [], writtenStreams := [], contractKind := "projector", coveredDefinition := "ci-recovery-state", errorReferences := [] }]

def modelDataFlows : List ModelDataFlow := [{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "confirmed task transaction board order", transformation := "projection", target := "task-state-published", bitEncoding := "UTF-8 JSON semantic projection event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for board-reordered", transformation := "projection", target := "board-reordered", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for repository-initialized", transformation := "projection", target := "repository-initialized", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-acceptance-added", transformation := "projection", target := "task-acceptance-added", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-acceptance-changed", transformation := "projection", target := "task-acceptance-changed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-acceptance-checked", transformation := "projection", target := "task-acceptance-checked", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-acceptance-removed", transformation := "projection", target := "task-acceptance-removed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-claim-changed", transformation := "projection", target := "task-claim-changed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-closed-from-trailer", transformation := "projection", target := "task-closed-from-trailer", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-created", transformation := "projection", target := "task-created", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-details-updated", transformation := "projection", target := "task-details-updated", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-links-changed", transformation := "projection", target := "task-links-changed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-note-added", transformation := "projection", target := "task-note-added", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-priority-changed", transformation := "projection", target := "task-priority-changed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-pull-request-changed", transformation := "projection", target := "task-pull-request-changed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-removed", transformation := "projection", target := "task-removed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-state-published", transformation := "projection", target := "task-state-published", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-subtask-added", transformation := "projection", target := "task-subtask-added", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-subtask-checked", transformation := "projection", target := "task-subtask-checked", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-subtasks-changed", transformation := "projection", target := "task-subtasks-changed", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-transitioned", transformation := "projection", target := "task-transitioned", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed task-command payload for task-validation-repaired", transformation := "projection", target := "task-validation-repaired", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "manage-work", slice := "change-task-board", datum := "task-command", sourceKind := ModelDataFlowSourceKind.original, source := "CLI or MCP request plus once-generated invocation facts", transformation := "transformation", target := "submit-task-command", bitEncoding := "UTF-8 JSON object with tagged command variant and lossless scalar fields" },{ workflow := "manage-work", slice := "view-task-board", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "confirmed semantic task event bytes", transformation := "identity", target := "task-state-published", bitEncoding := "UTF-8 JSON semantic task event payload" },{ workflow := "manage-work", slice := "view-task-board", datum := "state", sourceKind := ModelDataFlowSourceKind.original, source := "ordered repository board and task event fold", transformation := "projection", target := "task-board-state", bitEncoding := "UTF-8 JSON preserving existing task schemas" },{ workflow := "manage-work", slice := "view-task-board", datum := "state", sourceKind := ModelDataFlowSourceKind.original, source := "ordered repository board and task event fold through task-board-state", transformation := "projection", target := "task-board-view", bitEncoding := "UTF-8 JSON preserving existing task schemas" },{ workflow := "manage-work", slice := "view-task-board", datum := "task-command", sourceKind := ModelDataFlowSourceKind.original, source := "visible CLI MCP or dashboard control input", transformation := "identity", target := "task-board-view", bitEncoding := "UTF-8 JSON task command" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "confirmed recovery transaction state", transformation := "projection", target := "recovery-state-published", bitEncoding := "UTF-8 JSON semantic projection event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "first parsed recovery-command payload when repository is uninitialized", transformation := "projection", target := "repository-initialized", bitEncoding := "UTF-8 JSON semantic event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-action-chosen", transformation := "projection", target := "ci-recovery-action-chosen", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-assigned", transformation := "projection", target := "ci-recovery-assigned", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-claimed", transformation := "projection", target := "ci-recovery-claimed", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-diagnosed", transformation := "projection", target := "ci-recovery-diagnosed", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-heartbeat-recorded", transformation := "projection", target := "ci-recovery-heartbeat-recorded", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-joined", transformation := "projection", target := "ci-recovery-joined", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-replacement-recorded", transformation := "projection", target := "ci-recovery-replacement-recorded", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-reported", transformation := "projection", target := "ci-recovery-reported", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-resolved", transformation := "projection", target := "ci-recovery-resolved", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-taken-over", transformation := "projection", target := "ci-recovery-taken-over", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for ci-recovery-transferred", transformation := "projection", target := "ci-recovery-transferred", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for recovery-claimed", transformation := "projection", target := "recovery-claimed", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for recovery-initialized", transformation := "projection", target := "recovery-initialized", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for recovery-publication-blocked", transformation := "projection", target := "recovery-publication-blocked", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for recovery-resolved", transformation := "projection", target := "recovery-resolved", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for recovery-state-published", transformation := "projection", target := "recovery-state-published", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed recovery-command payload for recovery-status-recorded", transformation := "projection", target := "recovery-status-recorded", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", datum := "recovery-command", sourceKind := ModelDataFlowSourceKind.original, source := "MCP or CLI recovery invocation plus once-resolved identity", transformation := "transformation", target := "submit-recovery-command", bitEncoding := "UTF-8 JSON tagged recovery command" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "confirmed semantic recovery event bytes", transformation := "identity", target := "recovery-state-published", bitEncoding := "UTF-8 JSON semantic recovery event payload" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", datum := "recovery-command", sourceKind := ModelDataFlowSourceKind.original, source := "visible MCP or CLI recovery control input", transformation := "identity", target := "ci-recovery-view", bitEncoding := "UTF-8 JSON recovery command" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", datum := "state", sourceKind := ModelDataFlowSourceKind.original, source := "ordered recovery event fold", transformation := "projection", target := "ci-recovery-state", bitEncoding := "UTF-8 JSON preserving existing recovery schemas" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", datum := "state", sourceKind := ModelDataFlowSourceKind.original, source := "ordered recovery event fold through ci-recovery-state", transformation := "projection", target := "ci-recovery-view", bitEncoding := "UTF-8 JSON preserving existing recovery schemas" }]

def modelOutcomes : List ModelOutcome := [{ workflow := "manage-work", slice := "change-task-board", outcome := "task-change-confirmed", events := ["task-state-published"], externallyRelevant := false },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", outcome := "recovery-change-confirmed", events := ["recovery-state-published"], externallyRelevant := false }]

def modelCommandErrors : List ModelCommandError := [{ workflow := "manage-work", slice := "change-task-board", command := "submit-task-command", error := "version-conflict", scenario := "Confirmed conflict reruns pure command", recovery := "retry" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", command := "submit-recovery-command", error := "workflow-blocked", scenario := "Failed mandatory claim blocks downstream work", recovery := "explicit_recovery_action" }]

def modelCommands : List ModelCommand := [{ workflow := "manage-work", slice := "change-task-board", command := "submit-task-command" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", command := "submit-recovery-command" }]

def modelCommandInputs : List ModelCommandInput := [{ workflow := "manage-work", slice := "change-task-board", command := "submit-task-command", input := "task-command", sourceKind := ModelCommandInputSourceKind.actor, sourceDescription := "parsed stable task command including generated identifiers timestamp and note date", provenanceChain := ["CLI or MCP request","parse and generate once","pure EventCore command"], eventStreamSourceEvent := "", eventStreamSourceAttribute := "", externalPayloadSourceName := "", externalPayloadSourceField := "", generatedSourceName := "", generatedSourceField := "", sessionSourceName := "", sessionSourceField := "", invocationArgumentSourceName := "", invocationArgumentSourceField := "" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", command := "submit-recovery-command", input := "recovery-command", sourceKind := ModelCommandInputSourceKind.actor, sourceDescription := "claim join transfer takeover helper heartbeat diagnosis action replacement or resolution request with process-stable participant identity and frozen invocation time", provenanceChain := ["MCP or CLI request","resolve identity and time once","pure EventCore command"], eventStreamSourceEvent := "", eventStreamSourceAttribute := "", externalPayloadSourceName := "", externalPayloadSourceField := "", generatedSourceName := "", generatedSourceField := "", sessionSourceName := "", sessionSourceField := "", invocationArgumentSourceName := "", invocationArgumentSourceField := "" }]

def modelReadModels : List ModelReadModel := [{ workflow := "manage-work", slice := "view-task-board", readModel := "task-board-state" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", readModel := "ci-recovery-state" }]

def modelReadModelDefinitions : List ModelReadModelDefinition := [{ workflow := "manage-work", slice := "view-task-board", readModel := "task-board-state", transitive := false, relationshipFields := [], transitiveRule := "", exampleScenarioName := "" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", readModel := "ci-recovery-state", transitive := false, relationshipFields := [], transitiveRule := "", exampleScenarioName := "" }]

def modelReadModelFields : List ModelReadModelField := [{ workflow := "manage-work", slice := "view-task-board", readModel := "task-board-state", field := "state", sourceKind := "event_attribute", sourceEvent := "task-state-published", sourceAttribute := "payload", derivationRule := "", derivationSourceFields := [], absenceEvent := "", derivationScenarioName := "", absenceScenarioName := "", provenance := "fold repository board and task streams in global position order" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", readModel := "ci-recovery-state", field := "state", sourceKind := "event_attribute", sourceEvent := "recovery-state-published", sourceAttribute := "payload", derivationRule := "", derivationSourceFields := [], absenceEvent := "", derivationScenarioName := "", absenceScenarioName := "", provenance := "fold repository-wide recovery events in global position order" }]

def modelViews : List ModelView := [{ workflow := "manage-work", slice := "view-task-board", view := "task-board-view" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", view := "ci-recovery-view" }]

def modelViewDefinitions : List ModelViewDefinition := [{ workflow := "manage-work", slice := "view-task-board", view := "task-board-view", readModels := ["task-board-state"], sketchTokens := ["task-board-json"], localStates := [], filters := [] },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", view := "ci-recovery-view", readModels := ["ci-recovery-state"], sketchTokens := ["ci-recovery-json"], localStates := [], filters := [] }]

def modelViewControls : List ModelViewControl := [{ workflow := "manage-work", slice := "view-task-board", view := "task-board-view", control := "submit-task-command", command := "submit-task-command", input := "task-command", inputSourceKind := ModelCommandInputSourceKind.actor, inputSourceDescription := "task lifecycle mutation request", inputSketchToken := "task-command-json", inputVisibleToActor := true, inputDecisionField := true, handledErrors := ["version-conflict"], recoveryBehavior := "retry", controlSketchToken := "submit-task-command", navigationType := "modeled_view", navigationTarget := "task-board-view", externalWorkflow := "", externalSystem := "", handoffContract := "" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", view := "ci-recovery-view", control := "submit-recovery-command", command := "submit-recovery-command", input := "recovery-command", inputSourceKind := ModelCommandInputSourceKind.actor, inputSourceDescription := "claim status sync or resolution request", inputSketchToken := "recovery-command-json", inputVisibleToActor := true, inputDecisionField := true, handledErrors := ["workflow-blocked"], recoveryBehavior := "explicit_recovery_action", controlSketchToken := "submit-recovery-command", navigationType := "modeled_view", navigationTarget := "ci-recovery-view", externalWorkflow := "", externalSystem := "", handoffContract := "" }]

def modelBoardElements : List ModelBoardElement := []

def modelBoardConnections : List ModelBoardConnection := []

def modelViewFields : List ModelViewField := [{ workflow := "manage-work", slice := "view-task-board", view := "task-board-view", field := "state", sourceKind := "read_model", sourceReadModel := "task-board-state", sourceField := "state", provenance := "existing CLI MCP and dashboard response surfaces", bitEncoding := "UTF-8 JSON preserving existing task schemas" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", view := "ci-recovery-view", field := "state", sourceKind := "read_model", sourceReadModel := "ci-recovery-state", sourceField := "state", provenance := "existing MCP CLI and wait response surfaces plus blocker status", bitEncoding := "UTF-8 JSON preserving existing recovery schemas" }]

def modelAutomations : List ModelAutomation := []

def modelAutomationDefinitions : List ModelAutomationDefinition := []

def modelTranslations : List ModelTranslation := []

def modelTranslationDefinitions : List ModelTranslationDefinition := []

def modelExternalPayloads : List ModelExternalPayload := []

def modelExternalPayloadFields : List ModelExternalPayloadField := []

def modelStreams : List ModelStream := [{ workflow := "manage-work", slice := "change-task-board", stream := "tiber:board" },{ workflow := "manage-work", slice := "change-task-board", stream := "tiber:repository" },{ workflow := "manage-work", slice := "change-task-board", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "view-task-board", stream := "tiber:board" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", stream := "tiber:repository" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", stream := "tiber:ci-recovery" }]

def modelEvents : List ModelEvent := [{ workflow := "manage-work", slice := "change-task-board", event := "board-reordered", stream := "tiber:board" },{ workflow := "manage-work", slice := "change-task-board", event := "repository-initialized", stream := "tiber:repository" },{ workflow := "manage-work", slice := "change-task-board", event := "task-acceptance-added", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-acceptance-checked", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-acceptance-removed", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-claim-changed", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-closed-from-trailer", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-created", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-details-updated", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-links-changed", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-note-added", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-priority-changed", stream := "tiber:board" },{ workflow := "manage-work", slice := "change-task-board", event := "task-pull-request-changed", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-removed", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-state-published", stream := "tiber:board" },{ workflow := "manage-work", slice := "change-task-board", event := "task-subtask-added", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-subtask-checked", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-transitioned", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "change-task-board", event := "task-validation-repaired", stream := "tiber:task:<id>" },{ workflow := "manage-work", slice := "view-task-board", event := "task-state-published", stream := "tiber:board" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-action-chosen", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-assigned", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-claimed", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-diagnosed", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-heartbeat-recorded", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-joined", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-replacement-recorded", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-reported", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-resolved", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-taken-over", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-transferred", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "recovery-state-published", stream := "tiber:ci-recovery" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "repository-initialized", stream := "tiber:repository" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", event := "recovery-state-published", stream := "tiber:ci-recovery" }]

def modelEventAttributes : List ModelEventAttribute := [{ workflow := "manage-work", slice := "change-task-board", event := "board-reordered", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "repository-initialized", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-acceptance-added", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-acceptance-checked", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-acceptance-removed", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-claim-changed", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-closed-from-trailer", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-created", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-details-updated", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-links-changed", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-note-added", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-priority-changed", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-pull-request-changed", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-removed", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-state-published", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "confirmed task transaction publishes current board order for projection" },{ workflow := "manage-work", slice := "change-task-board", event := "task-subtask-added", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-subtask-checked", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-transitioned", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "change-task-board", event := "task-validation-repaired", attributeName := "payload", sourceKind := "command_input", sourceName := "task-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed task command fact copied without reinterpretation" },{ workflow := "manage-work", slice := "view-task-board", event := "task-state-published", attributeName := "payload", sourceKind := "generated", sourceName := "event-store", sourceField := "payload", generatedSourceKind := "event_replay", provenance := "confirmed semantic task event replayed by projector" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-action-chosen", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-assigned", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-claimed", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-diagnosed", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-heartbeat-recorded", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-joined", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-replacement-recorded", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-reported", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-resolved", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-taken-over", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "ci-recovery-transferred", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed recovery command fact copied without reinterpretation" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "recovery-state-published", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "confirmed recovery transaction publishes current recovery state for projection" },{ workflow := "recover-pushed-ci", slice := "coordinate-ci-recovery", event := "repository-initialized", attributeName := "payload", sourceKind := "command_input", sourceName := "recovery-command", sourceField := "payload", generatedSourceKind := "", provenance := "first recovery command initializes repository event metadata" },{ workflow := "recover-pushed-ci", slice := "view-ci-recovery", event := "recovery-state-published", attributeName := "payload", sourceKind := "generated", sourceName := "event-store", sourceField := "payload", generatedSourceKind := "event_replay", provenance := "confirmed semantic recovery event replayed by projector" }]

def modelScenarioDefinitionHasGwt (scenario : ModelScenarioDefinition) : Bool := scenario.given.isEmpty == false && scenario.when.isEmpty == false && scenario.thenStep.isEmpty == false

def modelScenarioKindIsFirstClass (scenario : ModelScenarioDefinition) : Bool := scenario.scenarioKind == "acceptance" || scenario.scenarioKind == "contract"

def modelDataFlowIsBitComplete (dataFlow : ModelDataFlow) : Bool := dataFlow.datum.isEmpty == false && dataFlow.source.isEmpty == false && dataFlow.transformation.isEmpty == false && dataFlow.target.isEmpty == false && dataFlow.bitEncoding.isEmpty == false

def modelDataFlowCoversDatumTarget (workflow : String) (slice : String) (datum : String) (target : String) : Bool := modelDataFlows.any (fun dataFlow => dataFlow.workflow == workflow && dataFlow.slice == slice && dataFlow.datum == datum && dataFlow.target == target && modelDataFlowIsBitComplete dataFlow)

def modelDataFlowBitEncodingMatchesDatumTarget (workflow : String) (slice : String) (datum : String) (target : String) (bitEncoding : String) : Bool := modelDataFlows.any (fun dataFlow => dataFlow.workflow == workflow && dataFlow.slice == slice && dataFlow.datum == datum && dataFlow.target == target && dataFlow.bitEncoding == bitEncoding && modelDataFlowIsBitComplete dataFlow)

def modelDataFlowSourceBitEncodingMatchesModeledSource (dataFlow : ModelDataFlow) : Bool := (modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source) == false) || modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && sourceFlow.bitEncoding == dataFlow.bitEncoding && modelDataFlowIsBitComplete sourceFlow)

def modelDataFlowHasModeledTransformationSemantics (dataFlow : ModelDataFlow) : Bool := dataFlow.transformation == "identity" || dataFlow.transformation == "projection" || dataFlow.transformation == "derivation" || dataFlow.transformation == "default" || dataFlow.transformation == "absence" || dataFlow.transformation == "transformation"

def modelDataFlowHasModeledSourceKind (dataFlow : ModelDataFlow) : Bool := match dataFlow.sourceKind with
  | ModelDataFlowSourceKind.original => dataFlow.source.isEmpty == false
  | ModelDataFlowSourceKind.modeledTarget => dataFlow.source.isEmpty == false

def modelDataFlowModeledSourceResolves (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind != ModelDataFlowSourceKind.modeledTarget || modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && modelDataFlowIsBitComplete sourceFlow)

def modelSameDataFlowTarget (left : ModelDataFlow) (right : ModelDataFlow) : Bool := left.workflow == right.workflow && left.slice == right.slice && left.datum == right.datum && left.target == right.target

def modelDataFlowTargetsFromReachable (reachable : List ModelDataFlow) : List ModelDataFlow := modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.modeledTarget && reachable.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && modelDataFlowIsBitComplete sourceFlow))

def modelDataFlowsReachableFromOriginalsAfterFuel : Nat -> List ModelDataFlow -> List ModelDataFlow
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => modelDataFlowsReachableFromOriginalsAfterFuel fuel (reachable ++ modelDataFlowTargetsFromReachable reachable)

def modelDataFlowsReachableFromOriginals : List ModelDataFlow := modelDataFlowsReachableFromOriginalsAfterFuel modelDataFlows.length (modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.original && modelDataFlowIsBitComplete dataFlow))

def modelDataFlowHasOriginalSourceChain (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind == ModelDataFlowSourceKind.original || modelDataFlowsReachableFromOriginals.any (fun reachableFlow => modelSameDataFlowTarget reachableFlow dataFlow)

def modelDataFlowTargetsFromBitPreservingReachable (reachable : List ModelDataFlow) : List ModelDataFlow := modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.modeledTarget && reachable.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && sourceFlow.bitEncoding == dataFlow.bitEncoding && modelDataFlowIsBitComplete sourceFlow))

def modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel : Nat -> List ModelDataFlow -> List ModelDataFlow
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel fuel (reachable ++ modelDataFlowTargetsFromBitPreservingReachable reachable)

def modelDataFlowsReachableFromOriginalsWithPreservedBits : List ModelDataFlow := modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel modelDataFlows.length (modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.original && modelDataFlowIsBitComplete dataFlow))

def modelDataFlowHasBitPreservingOriginalSourceChain (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind == ModelDataFlowSourceKind.original || modelDataFlowsReachableFromOriginalsWithPreservedBits.any (fun reachableFlow => modelSameDataFlowTarget reachableFlow dataFlow)

def modelCommandInputHasModeledDataFlow (input : ModelCommandInput) : Bool := modelDataFlowCoversDatumTarget input.workflow input.slice input.input input.command

def modelEventAttributeHasModeledDataFlow (eventAttribute : ModelEventAttribute) : Bool := modelDataFlowCoversDatumTarget eventAttribute.workflow eventAttribute.slice eventAttribute.attributeName eventAttribute.event

def modelReadModelFieldHasModeledDataFlow (field : ModelReadModelField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.readModel

def modelViewFieldHasModeledDataFlow (field : ModelViewField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.view

def modelViewFieldBitEncodingMatchesDataFlow (field : ModelViewField) : Bool := modelDataFlowBitEncodingMatchesDatumTarget field.workflow field.slice field.field field.view field.bitEncoding

def modelExternalPayloadFieldHasModeledDataFlow (field : ModelExternalPayloadField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.externalPayload

def modelExternalPayloadFieldBitEncodingMatchesDataFlow (field : ModelExternalPayloadField) : Bool := modelDataFlowBitEncodingMatchesDatumTarget field.workflow field.slice field.field field.externalPayload field.bitEncoding

def modelMeaningfulDataHasModeledDataFlows : Bool := modelCommandInputs.all modelCommandInputHasModeledDataFlow && modelEventAttributes.all modelEventAttributeHasModeledDataFlow && modelReadModelFields.all modelReadModelFieldHasModeledDataFlow && modelViewFields.all modelViewFieldHasModeledDataFlow && modelExternalPayloadFields.all modelExternalPayloadFieldHasModeledDataFlow

def modelCommandInputHasProvenance (input : ModelCommandInput) : Bool := input.sourceDescription.isEmpty == false && input.provenanceChain.isEmpty == false

def modelCommandInputTracesToInvocationSource (input : ModelCommandInput) : Bool := input.sourceKind == ModelCommandInputSourceKind.actor || (input.sourceKind == ModelCommandInputSourceKind.eventStreamState && input.eventStreamSourceEvent.isEmpty == false && input.eventStreamSourceAttribute.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.externalPayload && input.externalPayloadSourceName.isEmpty == false && input.externalPayloadSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.generated && input.generatedSourceName.isEmpty == false && input.generatedSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.session && input.sessionSourceName.isEmpty == false && input.sessionSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.invocationArgument && input.invocationArgumentSourceName.isEmpty == false && input.invocationArgumentSourceField.isEmpty == false)

def modelEventAttributeSourceIsComplete (eventAttribute : ModelEventAttribute) : Bool := eventAttribute.provenance.isEmpty == false && ((eventAttribute.sourceKind == "command_input" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false) || (eventAttribute.sourceKind == "external_payload" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false) || (eventAttribute.sourceKind == "generated" && eventAttribute.sourceName.isEmpty == false && eventAttribute.generatedSourceKind.isEmpty == false) || (eventAttribute.sourceKind == "session" && eventAttribute.sourceName.isEmpty == false) || (eventAttribute.sourceKind == "derivation" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false))

def modelReadModelFieldSourceIsComplete (field : ModelReadModelField) : Bool := (field.sourceKind == "event_attribute" && field.sourceEvent.isEmpty == false && field.sourceAttribute.isEmpty == false) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false)

def modelReadModelFieldTracesToOriginalProvenance (field : ModelReadModelField) : Bool := field.provenance.isEmpty == false && ((field.sourceKind == "event_attribute" && modelEventAttributes.any (fun eventAttribute => eventAttribute.workflow == field.workflow && eventAttribute.slice == field.slice && eventAttribute.event == field.sourceEvent && eventAttribute.attributeName == field.sourceAttribute && modelEventAttributeSourceIsComplete eventAttribute)) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false))

def modelViewFieldSourceIsComplete (field : ModelViewField) : Bool := field.sourceKind == "read_model" && field.sourceReadModel.isEmpty == false && field.sourceField.isEmpty == false && field.provenance.isEmpty == false && field.bitEncoding.isEmpty == false

def modelViewFieldReadModelFieldSourceResolves (viewField : ModelViewField) : Bool := modelViewFieldSourceIsComplete viewField && modelReadModelFields.any (fun readModelField => readModelField.workflow == viewField.workflow && readModelField.slice == viewField.slice && readModelField.readModel == viewField.sourceReadModel && readModelField.field == viewField.sourceField && modelReadModelFieldSourceIsComplete readModelField)

def modelDisplayedDatumTracesToOriginalProvenance (viewField : ModelViewField) : Bool := modelViewFieldReadModelFieldSourceResolves viewField && modelReadModelFields.any (fun readModelField => readModelField.workflow == viewField.workflow && readModelField.slice == viewField.slice && readModelField.readModel == viewField.sourceReadModel && readModelField.field == viewField.sourceField && modelReadModelFieldTracesToOriginalProvenance readModelField)

def modelExternalPayloadFieldHasProvenance (field : ModelExternalPayloadField) : Bool := field.provenance.isEmpty == false && field.bitEncoding.isEmpty == false

def modelControlProvidesCommandInput (control : ModelViewControl) (input : ModelCommandInput) : Bool := control.workflow == input.workflow && control.command == input.command && control.input == input.input

def modelViewControlProvidesEveryCommandInput (control : ModelViewControl) : Bool := modelCommandInputs.all (fun input => input.workflow != control.workflow || input.command != control.command || modelViewControls.any (fun providedInput => providedInput.workflow == control.workflow && providedInput.slice == control.slice && providedInput.view == control.view && providedInput.control == control.control && providedInput.command == control.command && modelControlProvidesCommandInput providedInput input))

def modelOutcomeBranchIsModeled (outcome : ModelOutcome) : Bool := outcome.outcome.isEmpty == false && outcome.events.isEmpty == false

def modelCommandErrorRecoveryIsModeled (commandError : ModelCommandError) : Bool := commandError.command.isEmpty == false && commandError.error.isEmpty == false && commandError.scenario.isEmpty == false && commandError.recovery.isEmpty == false

def modelViewControlNavigationTargetIsModeled (control : ModelViewControl) : Bool := control.navigationType.isEmpty || ((control.navigationType == "modeled_view" || control.navigationType == "local_view_state") && control.navigationTarget.isEmpty == false) || (control.navigationType == "external_workflow" && control.externalWorkflow.isEmpty == false) || (control.navigationType == "external_system" && control.externalSystem.isEmpty == false && control.handoffContract.isEmpty == false)

def modelExternalBoundaryContractIsModeled (translation : ModelTranslationDefinition) : Bool := translation.translation.isEmpty == false && translation.externalEvent.isEmpty == false && translation.payloadContract.isEmpty == false && translation.command.isEmpty == false

def modelWorkflowBehaviorSurfaceIsComplete : Bool := modelOutcomes.all modelOutcomeBranchIsModeled && modelCommandErrors.all modelCommandErrorRecoveryIsModeled && modelViewControls.all modelViewControlNavigationTargetIsModeled && modelTranslationDefinitions.all modelExternalBoundaryContractIsModeled

theorem modelIdentityIsStable : modelName = "Tiber" := rfl

theorem modelVersionIsStable : modelVersion = "0.1.0" := rfl

theorem modelDigestIsStable : modelDigest = "5c751ff9f1c7ad9ad7d030544d0d38cff1a3ea7496ea5d48ea1dabf7f6ce498b" := rfl

theorem modelWorkflowsAreDeclared : modelWorkflows.length = 2 := rfl

theorem modelSlicesAreDeclared : modelSlices.length = 4 := rfl

theorem modelSliceModulesAreDeclared : modelSliceModules.length = 4 := rfl

theorem modelWorkflowCompositionStructureComplete : (modelSlices.all modelSliceBelongsToDeclaredWorkflow && modelSlices.all modelSliceHasModule && modelSliceModules.all modelSliceModuleBelongsToDeclaredSlice && modelWorkflows.all modelWorkflowHasCompositionStructure) = true := by native_decide

theorem modelWorkflowBehaviorSurfaceIsCompleteIsStable : modelWorkflowBehaviorSurfaceIsComplete = true := by native_decide

theorem modelScenariosAreDeclared : modelScenarios.length = 11 := rfl

theorem modelScenarioDefinitionsAreDeclared : modelScenarioDefinitions.length = 11 := rfl

theorem modelScenarioDefinitionsHaveGwt : modelScenarioDefinitions.all modelScenarioDefinitionHasGwt = true := by native_decide

theorem modelScenarioKindsAreFirstClass : modelScenarioDefinitions.all modelScenarioKindIsFirstClass = true := by native_decide

theorem modelDataFlowsAreDeclared : modelDataFlows.length = 51 := rfl

theorem modelDataFlowsAreBitComplete : modelDataFlows.all modelDataFlowIsBitComplete = true := by native_decide

theorem modelDataFlowSourceKindsAreModeled : modelDataFlows.all modelDataFlowHasModeledSourceKind = true := by native_decide

theorem modelDataFlowModeledSourcesResolve : modelDataFlows.all modelDataFlowModeledSourceResolves = true := by native_decide

theorem modelDataFlowSourceChainsReachOriginals : modelDataFlows.all modelDataFlowHasOriginalSourceChain = true := by native_decide

theorem modelDataFlowSourceChainsPreserveBitEncodingSemantics : modelDataFlows.all modelDataFlowHasBitPreservingOriginalSourceChain = true := by native_decide

theorem modelDataFlowTransformationsAreModeled : modelDataFlows.all modelDataFlowHasModeledTransformationSemantics = true := by native_decide

theorem modelMeaningfulDataFlowsAreCovered : modelMeaningfulDataHasModeledDataFlows = true := by native_decide

theorem modelDataFlowSourceBitEncodingsMatchModeledSources : modelDataFlows.all modelDataFlowSourceBitEncodingMatchesModeledSource = true := by native_decide

theorem modelViewFieldBitEncodingsMatchDataFlows : modelViewFields.all modelViewFieldBitEncodingMatchesDataFlow = true := by native_decide

theorem modelExternalPayloadFieldBitEncodingsMatchDataFlows : modelExternalPayloadFields.all modelExternalPayloadFieldBitEncodingMatchesDataFlow = true := by native_decide

theorem modelOutcomesAreDeclared : modelOutcomes.length = 2 := rfl

theorem modelCommandErrorsAreDeclared : modelCommandErrors.length = 2 := rfl

theorem modelCommandsAreDeclared : modelCommands.length = 2 := rfl

theorem modelCommandInputsAreDeclared : modelCommandInputs.length = 2 := rfl

theorem modelCommandInputsHaveProvenance : modelCommandInputs.all modelCommandInputHasProvenance = true := by native_decide

theorem modelCommandInputsTraceToInvocationSources : modelCommandInputs.all modelCommandInputTracesToInvocationSource = true := by native_decide

theorem modelReadModelsAreDeclared : modelReadModels.length = 2 := rfl

theorem modelReadModelDefinitionsAreDeclared : modelReadModelDefinitions.length = 2 := rfl

theorem modelReadModelFieldsAreDeclared : modelReadModelFields.length = 2 := rfl

theorem modelEventAttributeSourcesAreComplete : modelEventAttributes.all modelEventAttributeSourceIsComplete = true := by native_decide

theorem modelReadModelFieldSourcesAreComplete : modelReadModelFields.all modelReadModelFieldSourceIsComplete = true := by native_decide

theorem modelViewFieldSourcesAreComplete : modelViewFields.all modelViewFieldSourceIsComplete = true := by native_decide

theorem modelViewFieldReadModelFieldSourcesResolve : modelViewFields.all modelViewFieldReadModelFieldSourceResolves = true := by native_decide

theorem modelDisplayedDataTraceToOriginalProvenance : modelViewFields.all modelDisplayedDatumTracesToOriginalProvenance = true := by native_decide

theorem modelExternalPayloadFieldsHaveProvenance : modelExternalPayloadFields.all modelExternalPayloadFieldHasProvenance = true := by native_decide

theorem modelViewsAreDeclared : modelViews.length = 2 := rfl

theorem modelViewDefinitionsAreDeclared : modelViewDefinitions.length = 2 := rfl

theorem modelViewControlsAreDeclared : modelViewControls.length = 2 := rfl

theorem modelViewControlsProvideCommandInputs : modelViewControls.all modelViewControlProvidesEveryCommandInput = true := by native_decide

theorem modelBoardElementsAreDeclared : modelBoardElements.length = 0 := rfl

theorem modelBoardConnectionsAreDeclared : modelBoardConnections.length = 0 := rfl

theorem modelViewFieldsAreDeclared : modelViewFields.length = 2 := rfl

theorem modelAutomationsAreDeclared : modelAutomations.length = 0 := rfl

theorem modelAutomationDefinitionsAreDeclared : modelAutomationDefinitions.length = 0 := rfl

theorem modelTranslationsAreDeclared : modelTranslations.length = 0 := rfl

theorem modelTranslationDefinitionsAreDeclared : modelTranslationDefinitions.length = 0 := rfl

theorem modelExternalPayloadsAreDeclared : modelExternalPayloads.length = 0 := rfl

theorem modelExternalPayloadFieldsAreDeclared : modelExternalPayloadFields.length = 0 := rfl

theorem modelStreamsAreDeclared : modelStreams.length = 7 := rfl

theorem modelEventsAreDeclared : modelEvents.length = 34 := rfl

theorem modelEventAttributesAreDeclared : modelEventAttributes.length = 34 := rfl

end Tiber
